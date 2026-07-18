#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_selects_a_suggestion_and_filters() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    app.fill(".palette-input", "auth").await;
    app.wait_for(".suggestion-btn").await;

    app.press_key(".palette-input", "ArrowDown").await;
    app.wait_for(".suggestion-btn.focused").await;

    app.press_key_native(".palette-input", "Enter").await;
    app.wait_for(".tag-pill").await;
    app.wait_for_query("filters").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn palette_keyboard_moves_between_candidates_and_back_to_input() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    app.fill(".palette-input", "cloud").await;
    app.wait_for(".suggestion-btn").await;

    app.press_key(".palette-input", "ArrowDown").await;
    app.wait_for_class(".suggestion-btn", 0, "focused").await;

    app.press_key(".palette-input", "ArrowRight").await;
    app.wait_for_class(".suggestion-btn", 1, "focused").await;

    app.press_key(".palette-input", "ArrowLeft").await;
    app.wait_for_class(".suggestion-btn", 0, "focused").await;

    app.press_key(".palette-input", "ArrowUp").await;
    app.wait_for_gone(".suggestion-btn.focused").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn removing_a_filter_pill_clears_it() {
    let app = Session::open("/?filters.platform=gcp").await;

    app.wait_for(".facet-view").await;
    app.wait_for(".tag-pill").await;

    app.click(".pill-remove").await;
    app.wait_for_gone(".tag-pill").await;
    app.wait_for_no_query("filters").await;
}
