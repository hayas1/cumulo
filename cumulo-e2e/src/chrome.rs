use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::Page;
use futures::StreamExt;
use tokio::task::JoinHandle;
use tokio::time::timeout;

const LAUNCH_TIMEOUT: Duration = Duration::from_secs(45);
const OPEN_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct Chrome {
    browser: Browser,
    profile_dir: PathBuf,
    handler: JoinHandle<()>,
}

impl Chrome {
    pub(crate) async fn launch() -> Chrome {
        let profile_dir = Self::unique_profile_dir();
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
        if let Some(exe) = Self::executable() {
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

    pub(crate) async fn open(&self, url: &str) -> Page {
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

    fn unique_profile_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("cumulo-e2e-{}-{seq}", std::process::id()))
    }

    fn executable() -> Option<PathBuf> {
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
}

impl Drop for Chrome {
    fn drop(&mut self) {
        self.handler.abort();
        let _ = std::process::Command::new("pkill")
            .args(["-9", "-f", &self.profile_dir.to_string_lossy()])
            .status();
        std::thread::sleep(Duration::from_millis(200));
    }
}
