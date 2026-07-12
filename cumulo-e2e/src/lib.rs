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
pub fn dist() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../cumulo-web/dist"))
}

pub struct Site {
    pub base_url: String,
    _server: JoinHandle<()>,
}

impl Site {
    pub async fn serve(dist: impl AsRef<Path>) -> Site {
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

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

pub struct Chrome {
    pub browser: Browser,
    profile_dir: PathBuf,
    handler: JoinHandle<()>,
}

impl Chrome {
    pub async fn launch() -> Chrome {
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

    pub async fn open(&self, url: impl AsRef<str>) -> Page {
        // Create the tab on about:blank, then fire navigation via raw CDP.
        // `new_page(url)` waits for a load event that never arrives for the WASM
        // app on some CI runners; readiness is asserted with `wait_for` instead.
        let page = timeout(OPEN_TIMEOUT, self.browser.new_page("about:blank"))
            .await
            .expect("create tab timed out")
            .expect("create tab");
        let navigate = NavigateParams::builder()
            .url(url.as_ref())
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

/// A Leptos CSR app mounts asynchronously after the document loads, and
/// chromiumoxide (unlike Playwright) does not retry element lookups. Poll until
/// the selector resolves so scenarios read against a rendered DOM.
pub async fn wait_for(page: &Page, selector: &str) -> Element {
    let deadline = Instant::now() + SELECTOR_TIMEOUT;
    loop {
        if let Ok(element) = page.find_element(selector).await {
            return element;
        }
        if Instant::now() >= deadline {
            panic!("selector `{selector}` did not appear within {SELECTOR_TIMEOUT:?}");
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Tests run in parallel threads; a shared Chromium profile deadlocks on its
/// singleton lock, so each browser gets an isolated user-data dir.
fn unique_profile_dir() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cumulo-e2e-{}-{seq}", std::process::id()))
}

/// Poll a JavaScript boolean expression until it holds. Complements
/// [`wait_for`] for state that is not a bare element (URL, text content,
/// localStorage), again offsetting chromiumoxide's lack of retries.
pub async fn wait_until(page: &Page, expression: &str) {
    let deadline = Instant::now() + SELECTOR_TIMEOUT;
    loop {
        if eval_bool(page, expression).await {
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
pub async fn eval_bool(page: &Page, expression: &str) -> bool {
    match timeout(CALL_TIMEOUT, page.evaluate(expression)).await {
        Ok(Ok(result)) => result.into_value::<bool>().unwrap_or(false),
        Ok(Err(_)) => false,
        Err(_) => panic!("evaluate timed out after {CALL_TIMEOUT:?}: {expression}"),
    }
}

/// Wait until some element matching `selector` contains `needle` in its text.
pub async fn wait_for_text(page: &Page, selector: &str, needle: &str) {
    let expression = format!(
        "Array.from(document.querySelectorAll({selector:?})).some(e => e.textContent.includes({needle:?}))"
    );
    wait_until(page, &expression).await;
}

/// Focus `selector` and dispatch a `keydown` for `key` (e.g. "ArrowDown",
/// "Enter"). Handlers that call preventDefault cancel the event, so success is
/// the element existing rather than the dispatch return value.
pub async fn press_key(page: &Page, selector: &str, key: &str) {
    let expression = format!(
        "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.focus(); el.dispatchEvent(new KeyboardEvent('keydown', {{ key: {key:?}, bubbles: true, cancelable: true }})); return true; }})()"
    );
    assert!(
        eval_bool(page, &expression).await,
        "no element `{selector}` to press `{key}` on"
    );
}

/// Click the `index`-th match of `selector` via a native CDP input event.
pub async fn click_nth(page: &Page, selector: &str, index: usize) {
    let elements = page.find_elements(selector).await.expect("find elements");
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
pub async fn click(page: &Page, selector: &str) {
    let element = wait_for(page, selector).await;
    match timeout(CALL_TIMEOUT, element.click()).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => panic!("click `{selector}` failed: {e:?}"),
        Err(_) => panic!("click `{selector}` timed out after {CALL_TIMEOUT:?}"),
    }
}

/// Wait for `selector`, focus it, and type `text`.
pub async fn fill(page: &Page, selector: &str, text: &str) {
    let element = wait_for(page, selector).await;
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
pub async fn reload(page: &Page) {
    match timeout(CALL_TIMEOUT, page.reload()).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => panic!("reload failed: {e:?}"),
        Err(_) => panic!("reload timed out after {CALL_TIMEOUT:?}"),
    }
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
