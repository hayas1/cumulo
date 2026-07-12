#![cfg(feature = "browser")]

use cumulo_e2e::{
    dist, fill, press_key, wait_for, wait_for_class, wait_for_gone, wait_until, Chrome, Site,
};

const INPUT: &str = ".palette-input";
const SUGGESTION: &str = ".suggestion-btn";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_selects_a_suggestion_and_filters() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".facet-view").await;

    // Typing surfaces category suggestions; ArrowDown highlights the first and
    // Enter commits it as a filter pill.
    fill(&page, INPUT, "auth").await;
    wait_for(&page, SUGGESTION).await;

    press_key(&page, INPUT, "ArrowDown").await;
    wait_for(&page, ".suggestion-btn.focused").await;

    press_key(&page, INPUT, "Enter").await;
    wait_for(&page, ".tag-pill").await;
    wait_until(&page, "location.search.includes('filters')").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_moves_between_candidates_and_back_to_input() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".facet-view").await;

    // "cloud" surfaces several categories, so left/right can move between them.
    fill(&page, INPUT, "cloud").await;
    wait_until(
        &page,
        "document.querySelectorAll('.suggestion-btn').length >= 2",
    )
    .await;

    // Down focuses the first candidate, Right/Left move between candidates.
    press_key(&page, INPUT, "ArrowDown").await;
    wait_for_class(&page, SUGGESTION, 0, "focused").await;

    press_key(&page, INPUT, "ArrowRight").await;
    wait_for_class(&page, SUGGESTION, 1, "focused").await;

    press_key(&page, INPUT, "ArrowLeft").await;
    wait_for_class(&page, SUGGESTION, 0, "focused").await;

    // Up returns focus to the input, leaving no candidate highlighted.
    press_key(&page, INPUT, "ArrowUp").await;
    wait_for_gone(&page, ".suggestion-btn.focused").await;
}
