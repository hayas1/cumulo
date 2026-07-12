use std::time::Duration;

use chromiumoxide::{Element, Page};
use tokio::time::{timeout, Instant};

use crate::chrome::Chrome;
use crate::site::Site;

const SELECTOR_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

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
        let site = Site::serve().await;
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
