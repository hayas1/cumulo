#![cfg(feature = "browser")]

use cumulo_e2e::Session;

const INPUT: &str = ".palette-input";
const SUGGESTION: &str = ".suggestion-btn";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_selects_a_suggestion_and_filters() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    app.fill(INPUT, "auth").await;
    app.wait_for(SUGGESTION).await;

    app.press_key(INPUT, "ArrowDown").await;
    app.wait_for(".suggestion-btn.focused").await;

    app.press_key(INPUT, "Enter").await;
    app.wait_for(".tag-pill").await;
    app.wait_until("location.search.includes('filters')").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_moves_between_candidates_and_back_to_input() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    // "cloud" surfaces several categories, so left/right can move between them.
    app.fill(INPUT, "cloud").await;
    app.wait_until("document.querySelectorAll('.suggestion-btn').length >= 2")
        .await;

    app.press_key(INPUT, "ArrowDown").await;
    app.wait_for_class(SUGGESTION, 0, "focused").await;

    app.press_key(INPUT, "ArrowRight").await;
    app.wait_for_class(SUGGESTION, 1, "focused").await;

    app.press_key(INPUT, "ArrowLeft").await;
    app.wait_for_class(SUGGESTION, 0, "focused").await;

    app.press_key(INPUT, "ArrowUp").await;
    app.wait_for_gone(".suggestion-btn.focused").await;
}
