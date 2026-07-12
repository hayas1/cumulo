#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_prefilters_the_facet_list() {
    let app = Session::open("/?filters.platform=gcp").await;

    app.wait_for(".facet-view").await;

    app.wait_for_text(".tag-pill", "gcp").await;
    app.wait_for(".result-card").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_restores_map_axis_and_filter() {
    let app = Session::open("/?view=map&zoom_axis=product&filters.platform=gcp").await;

    app.wait_for(".map-view").await;

    app.wait_for_class(".facet-panel-title-btn", 1, "active")
        .await;
    app.wait_for_text(".tag-pill", "gcp").await;
}
