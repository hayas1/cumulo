#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selecting_a_map_axis_sets_the_zoom_axis() {
    let app = Session::open("/?view=map").await;

    app.wait_for(".map-view").await;
    app.wait_for(".facet-panel-title-btn").await;

    app.click_nth(".facet-panel-title-btn", 1).await;
    app.wait_for_class(".facet-panel-title-btn", 1, "active")
        .await;
    app.wait_for_query("zoom_axis").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clicking_a_map_cluster_drills_down_and_updates_url() {
    let app = Session::open("/?view=map").await;

    app.wait_for(".map-view").await;
    app.wait_for(".cluster-bg.drillable").await;

    app.click_nth_js(".cluster-bg.drillable", 0).await;
    app.wait_for(".tag-pill").await;
    app.wait_for_query("filters").await;
    app.wait_for_text(".level-badge", "1").await;
}
