#![cfg(feature = "browser")]

use cumulo_e2e::{dist, fill, press_key, wait_for, wait_until, Chrome, Site};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_selects_a_suggestion_and_filters() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".facet-view").await;

    // Typing surfaces category suggestions; ArrowDown highlights the first and
    // Enter commits it as a filter pill.
    fill(&page, ".palette-input", "auth").await;
    wait_for(&page, ".suggestion-btn").await;

    press_key(&page, ".palette-input", "ArrowDown").await;
    wait_for(&page, ".suggestion-btn.focused").await;

    press_key(&page, ".palette-input", "Enter").await;
    wait_for(&page, ".tag-pill").await;
    wait_until(&page, "location.search.includes('filters')").await;
}
