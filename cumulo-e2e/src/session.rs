use std::time::Duration;

use chromiumoxide::{Element, Page};
use tokio::time::{timeout, Instant};

use crate::chrome::Chrome;
use crate::site::Site;

const SELECTOR_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// One browser session against the built app, bundling the static server, the
/// Chromium instance, and the open page. chromiumoxide (unlike Playwright) does
/// not retry element lookups, so the waiting methods poll and every CDP call is
/// time-capped.
pub struct Session {
    page: Page,
    // Held only for their Drop: Chrome reaps the browser tree, Site owns the
    // server task. They are never read after construction.
    _chrome: Chrome,
    _site: Site,
}

impl Session {
    pub async fn open(path: &str) -> Session {
        let site = Site::serve().await;
        let chrome = Chrome::launch().await;
        let page = chrome.open(&site.url(path)).await;
        Session {
            page,
            _chrome: chrome,
            _site: site,
        }
    }

    /// For interactions the harness does not wrap.
    pub fn page(&self) -> &Page {
        &self.page
    }

    pub async fn wait_for(&self, selector: &str) -> Element {
        // A Leptos CSR app mounts asynchronously, so a bare lookup races it.
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

    pub async fn eval_bool(&self, expression: &str) -> bool {
        // Cap the call so a never-returning CDP evaluate cannot outlive the deadlines.
        match timeout(CALL_TIMEOUT, self.page.evaluate(expression)).await {
            Ok(Ok(result)) => result.into_value::<bool>().unwrap_or(false),
            Ok(Err(_)) => false,
            Err(_) => panic!("evaluate timed out after {CALL_TIMEOUT:?}: {expression}"),
        }
    }

    pub async fn wait_for_text(&self, selector: &str, needle: &str) {
        let expression = format!(
            "Array.from(document.querySelectorAll({selector:?})).some(e => e.textContent.includes({needle:?}))"
        );
        self.wait_until(&expression).await;
    }

    pub async fn wait_for_class(&self, selector: &str, index: usize, class: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelectorAll({selector:?})[{index}]; return !!el && el.classList.contains({class:?}); }})()"
        );
        self.wait_until(&expression).await;
    }

    pub async fn wait_for_gone(&self, selector: &str) {
        let expression = format!("!document.querySelector({selector:?})");
        self.wait_until(&expression).await;
    }

    pub async fn count(&self, selector: &str) -> usize {
        let expression = format!("document.querySelectorAll({selector:?}).length");
        match timeout(CALL_TIMEOUT, self.page.evaluate(expression)).await {
            Ok(Ok(result)) => result.into_value::<f64>().unwrap_or(0.0) as usize,
            _ => 0,
        }
    }

    pub async fn set_value(&self, selector: &str, value: &str) {
        // Unlike fill's typing this replaces the value; fire `input` so the framework syncs.
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.value = {value:?}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to set value on"
        );
    }

    pub async fn press_key(&self, selector: &str, key: &str) {
        // preventDefault cancels the event, so success is the element existing
        // rather than the dispatch return value.
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.focus(); el.dispatchEvent(new KeyboardEvent('keydown', {{ key: {key:?}, bubbles: true, cancelable: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to press `{key}` on"
        );
    }

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

    pub async fn click(&self, selector: &str) {
        let element = self.wait_for(selector).await;
        match timeout(CALL_TIMEOUT, element.click()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("click `{selector}` failed: {e:?}"),
            Err(_) => panic!("click `{selector}` timed out after {CALL_TIMEOUT:?}"),
        }
    }

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

    pub async fn reload(&self) {
        match timeout(CALL_TIMEOUT, self.page.reload()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("reload failed: {e:?}"),
            Err(_) => panic!("reload timed out after {CALL_TIMEOUT:?}"),
        }
    }
}
