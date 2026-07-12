//! Browser-driven end-to-end harness. The whole crate is gated behind the
//! `browser` feature so `cargo test --all` never compiles Chromium tooling;
//! run the scenarios with `cargo test -p cumulo-e2e --features browser`.
#![cfg(feature = "browser")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::{Element, Page};
use futures::StreamExt;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Instant};
use tower_http::services::{ServeDir, ServeFile};

const SELECTOR_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(45);
const OPEN_TIMEOUT: Duration = Duration::from_secs(60);

/// The `cumulo-web` build output the harness serves. Produce it with
/// `trunk build` in `cumulo-web` before running the scenarios.
fn dist() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../cumulo-web/dist"))
}

struct Site {
    base_url: String,
    _server: JoinHandle<()>,
}

impl Site {
    async fn serve(dist: impl AsRef<Path>) -> Site {
        let dist = dist.as_ref();
        let index = dist.join("index.html");
        let service = ServeDir::new(dist).not_found_service(ServeFile::new(index));
        let app = Router::new().fallback_service(service);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Site {
            base_url: format!("http://{addr}"),
            _server: server,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

struct Chrome {
    browser: Browser,
    profile_dir: PathBuf,
    handler: JoinHandle<()>,
}

impl Chrome {
    async fn launch() -> Chrome {
        let profile_dir = unique_profile_dir();
        // chromiumoxide's default headless flag is the legacy `--headless`, which
        // recent Chromium ignores and then runs headful (target creation hangs
        // where there is no display). Drive the new headless mode ourselves.
        let mut builder = BrowserConfig::builder()
            .with_head()
            .arg("--headless=new")
            .arg("--no-sandbox")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .user_data_dir(&profile_dir);
        if let Some(exe) = chrome_executable() {
            builder = builder.chrome_executable(exe);
        }
        let config = builder.build().expect("browser config");
        let launched = timeout(LAUNCH_TIMEOUT, Browser::launch(config))
            .await
            .expect("chromium launch timed out")
            .expect("launch chromium");
        let (browser, mut handler) = launched;
        let handler = tokio::spawn(async move { while handler.next().await.is_some() {} });
        Chrome {
            browser,
            profile_dir,
            handler,
        }
    }

    async fn open(&self, url: &str) -> Page {
        // Create the tab on about:blank, then fire navigation via raw CDP.
        // `new_page(url)` waits for a load event that never arrives for the WASM
        // app on some CI runners; readiness is asserted with `wait_for` instead.
        let page = timeout(OPEN_TIMEOUT, self.browser.new_page("about:blank"))
            .await
            .expect("create tab timed out")
            .expect("create tab");
        let navigate = NavigateParams::builder()
            .url(url)
            .build()
            .expect("navigate params");
        timeout(OPEN_TIMEOUT, page.execute(navigate))
            .await
            .expect("navigate timed out")
            .expect("navigate");
        page
    }
}

/// chromiumoxide does not reliably reap the Chromium process tree on drop, and
/// with limited RAM the leftovers starve later launches. Kill the process tree
/// by its unique `--user-data-dir` (safe: the path matches only this browser).
impl Drop for Chrome {
    fn drop(&mut self) {
        self.handler.abort();
        let _ = std::process::Command::new("pkill")
            .args(["-9", "-f", &self.profile_dir.to_string_lossy()])
            .status();
        // Let the OS reap the tree before the next test launches a browser, so
        // teardown does not race a fresh launch on a memory-constrained host.
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// One browser session against the built app: it bundles the static server, the
/// Chromium instance, and the open page, and exposes the DOM interactions the
/// scenarios drive. chromiumoxide (unlike Playwright) does not retry element
/// lookups, so the waiting methods poll and every CDP call is time-capped.
pub struct Session {
    page: Page,
    // Held only for their Drop: Chrome reaps the browser tree, Site owns the
    // server task. They are never read after construction.
    _chrome: Chrome,
    _site: Site,
}

impl Session {
    /// Serve `dist`, launch Chromium, and open the app at `path` (e.g. "/" or
    /// "/?view=map").
    pub async fn open(path: &str) -> Session {
        let site = Site::serve(dist()).await;
        let chrome = Chrome::launch().await;
        let page = chrome.open(&site.url(path)).await;
        Session {
            page,
            _chrome: chrome,
            _site: site,
        }
    }

    /// The underlying page, for interactions the harness does not wrap.
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Poll until `selector` resolves against the rendered DOM. A Leptos CSR app
    /// mounts asynchronously after the document loads, so a bare lookup races it.
    pub async fn wait_for(&self, selector: &str) -> Element {
        let deadline = Instant::now() + SELECTOR_TIMEOUT;
        loop {
            if let Ok(element) = self.page.find_element(selector).await {
                return element;
            }
            if Instant::now() >= deadline {
                panic!("selector `{selector}` did not appear within {SELECTOR_TIMEOUT:?}");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// Poll a JavaScript boolean expression until it holds. Complements
    /// [`Session::wait_for`] for state that is not a bare element (URL, text
    /// content, localStorage).
    pub async fn wait_until(&self, expression: &str) {
        let deadline = Instant::now() + SELECTOR_TIMEOUT;
        loop {
            if self.eval_bool(expression).await {
                return;
            }
            if Instant::now() >= deadline {
                panic!("condition never held within {SELECTOR_TIMEOUT:?}: {expression}");
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// Evaluate a boolean expression once. A CDP call that never returns would
    /// otherwise hang the whole test past every deadline, so cap it.
    pub async fn eval_bool(&self, expression: &str) -> bool {
        match timeout(CALL_TIMEOUT, self.page.evaluate(expression)).await {
            Ok(Ok(result)) => result.into_value::<bool>().unwrap_or(false),
            Ok(Err(_)) => false,
            Err(_) => panic!("evaluate timed out after {CALL_TIMEOUT:?}: {expression}"),
        }
    }

    /// Wait until some element matching `selector` contains `needle` in its text.
    pub async fn wait_for_text(&self, selector: &str, needle: &str) {
        let expression = format!(
            "Array.from(document.querySelectorAll({selector:?})).some(e => e.textContent.includes({needle:?}))"
        );
        self.wait_until(&expression).await;
    }

    /// Wait until the `index`-th match of `selector` carries `class`.
    pub async fn wait_for_class(&self, selector: &str, index: usize, class: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelectorAll({selector:?})[{index}]; return !!el && el.classList.contains({class:?}); }})()"
        );
        self.wait_until(&expression).await;
    }

    /// Wait until nothing matches `selector`.
    pub async fn wait_for_gone(&self, selector: &str) {
        let expression = format!("!document.querySelector({selector:?})");
        self.wait_until(&expression).await;
    }

    /// Count the elements matching `selector`.
    pub async fn count(&self, selector: &str) -> usize {
        let expression = format!("document.querySelectorAll({selector:?}).length");
        match timeout(CALL_TIMEOUT, self.page.evaluate(expression)).await {
            Ok(Ok(result)) => result.into_value::<f64>().unwrap_or(0.0) as usize,
            _ => 0,
        }
    }

    /// Replace the value of the input at `selector` and fire `input` so the
    /// framework picks the change up (unlike typing, this does not append).
    pub async fn set_value(&self, selector: &str, value: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.value = {value:?}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to set value on"
        );
    }

    /// Focus `selector` and dispatch a `keydown` for `key` (e.g. "ArrowDown",
    /// "Enter"). Handlers that call preventDefault cancel the event, so success
    /// is the element existing rather than the dispatch return value.
    pub async fn press_key(&self, selector: &str, key: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.focus(); el.dispatchEvent(new KeyboardEvent('keydown', {{ key: {key:?}, bubbles: true, cancelable: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to press `{key}` on"
        );
    }

    /// Click the `index`-th match of `selector` via a native CDP input event.
    pub async fn click_nth(&self, selector: &str, index: usize) {
        let elements = self
            .page
            .find_elements(selector)
            .await
            .expect("find elements");
        let element = elements.get(index).unwrap_or_else(|| {
            panic!(
                "no `{selector}` at index {index} ({} found)",
                elements.len()
            )
        });
        match timeout(CALL_TIMEOUT, element.click()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("click `{selector}`[{index}] failed: {e:?}"),
            Err(_) => panic!("click `{selector}`[{index}] timed out after {CALL_TIMEOUT:?}"),
        }
    }

    /// Wait for `selector`, then click it via a native CDP input event.
    pub async fn click(&self, selector: &str) {
        let element = self.wait_for(selector).await;
        match timeout(CALL_TIMEOUT, element.click()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("click `{selector}` failed: {e:?}"),
            Err(_) => panic!("click `{selector}` timed out after {CALL_TIMEOUT:?}"),
        }
    }

    /// Wait for `selector`, focus it, and type `text`.
    pub async fn fill(&self, selector: &str, text: &str) {
        let element = self.wait_for(selector).await;
        let typing = async {
            element.click().await?;
            element.type_str(text).await?;
            Ok::<_, chromiumoxide::error::CdpError>(())
        };
        match timeout(CALL_TIMEOUT, typing).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("fill `{selector}` failed: {e:?}"),
            Err(_) => panic!("fill `{selector}` timed out after {CALL_TIMEOUT:?}"),
        }
    }

    /// Reload and wait for the document to load again.
    pub async fn reload(&self) {
        match timeout(CALL_TIMEOUT, self.page.reload()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("reload failed: {e:?}"),
            Err(_) => panic!("reload timed out after {CALL_TIMEOUT:?}"),
        }
    }
}

/// Tests run in parallel threads; a shared Chromium profile deadlocks on its
/// singleton lock, so each browser gets an isolated user-data dir.
fn unique_profile_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cumulo-e2e-{}-{seq}", std::process::id()))
}

fn chrome_executable() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CUMULO_E2E_CHROME") {
        return Some(PathBuf::from(path));
    }
    let cache = PathBuf::from(std::env::var("HOME").ok()?).join(".cache/ms-playwright");
    for entry in std::fs::read_dir(&cache).ok()?.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("chromium") {
            continue;
        }
        for relative in ["chrome-linux64/chrome", "chrome-linux/chrome"] {
            let candidate = entry.path().join(relative);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
