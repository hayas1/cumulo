use std::time::Duration;

use chromiumoxide::{Element, Page};
use tokio::time::{timeout, Instant};

use crate::chrome::Chrome;
use crate::site::Site;

const SELECTOR_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Session {
    page: Page,
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

    pub async fn wait_for_count(&self, selector: &str, n: usize) {
        let expression = format!("document.querySelectorAll({selector:?}).length === {n}");
        self.wait_until(&expression).await;
    }

    pub async fn wait_for_nonempty_value(&self, selector: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); return !!el && el.value.length > 0; }})()"
        );
        self.wait_until(&expression).await;
    }

    pub async fn wait_for_query(&self, substring: &str) {
        let expression = format!("location.search.includes({substring:?})");
        self.wait_until(&expression).await;
    }

    pub async fn wait_for_no_query(&self, substring: &str) {
        let expression = format!("!location.search.includes({substring:?})");
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
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.value = {value:?}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to set value on"
        );
    }

    pub async fn press_key(&self, selector: &str, key: &str) {
        let expression = format!(
            "(() => {{ const el = document.querySelector({selector:?}); if (!el) return false; el.focus(); el.dispatchEvent(new KeyboardEvent('keydown', {{ key: {key:?}, bubbles: true, cancelable: true }})); return true; }})()"
        );
        assert!(
            self.eval_bool(&expression).await,
            "no element `{selector}` to press `{key}` on"
        );
    }

    pub async fn click_nth(&self, selector: &str, index: usize) {
        let deadline = Instant::now() + SELECTOR_TIMEOUT;
        let element = loop {
            let elements = self.page.find_elements(selector).await.unwrap_or_default();
            let found = elements.len();
            if let Some(element) = elements.into_iter().nth(index) {
                break element;
            }
            if Instant::now() >= deadline {
                panic!(
                    "no `{selector}` at index {index} within {SELECTOR_TIMEOUT:?} ({found} found)"
                );
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        };
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
