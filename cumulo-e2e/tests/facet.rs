#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sidebar_value_click_filters_the_list() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.wait_for(".facet-value").await;

    app.click_nth(".facet-value", 0).await;
    app.wait_for(".facet-value.selected").await;
    app.wait_for(".tag-pill").await;
    app.wait_for_query("filters").await;
}
