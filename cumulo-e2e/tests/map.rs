#![cfg(feature = "browser")]

use cumulo_e2e::Session;

const AXIS: &str = ".facet-panel-title-btn";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selecting_a_map_axis_sets_the_zoom_axis() {
    let app = Session::open("/?view=map").await;

    app.wait_for(".map-view").await;
    app.wait_for(AXIS).await;

    // A non-default axis becomes the active zoom axis when clicked.
    app.click_nth(AXIS, 1).await;
    app.wait_for_class(AXIS, 1, "active").await;
    app.wait_until("location.search.includes('zoom_axis')")
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clicking_a_map_cluster_drills_down_and_updates_url() {
    let app = Session::open("/?view=map").await;

    app.wait_for(".map-view").await;
    app.wait_for(".cluster-bg").await;

    // Clicking a cluster zooms into it (level badge advances) and drills down by
    // setting a filter, which is reflected in the URL.
    app.click_nth(".cluster-bg", 0).await;
    app.wait_for(".tag-pill").await;
    app.wait_until("location.search.includes('filters')").await;
    app.wait_for_text(".level-badge", "1").await;
}
