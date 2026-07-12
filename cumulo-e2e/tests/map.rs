#![cfg(feature = "browser")]

use cumulo_e2e::{
    click_nth, dist, wait_for, wait_for_class, wait_for_text, wait_until, Chrome, Site,
};

const AXIS: &str = ".facet-panel-title-btn";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selecting_a_map_axis_sets_the_zoom_axis() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/?view=map")).await;

    wait_for(&page, ".map-view").await;
    wait_for(&page, AXIS).await;

    // A non-default axis becomes the active zoom axis when clicked.
    click_nth(&page, AXIS, 1).await;
    wait_for_class(&page, AXIS, 1, "active").await;
    wait_until(&page, "location.search.includes('zoom_axis')").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clicking_a_map_cluster_drills_down_and_updates_url() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/?view=map")).await;

    wait_for(&page, ".map-view").await;
    wait_for(&page, ".cluster-bg").await;

    // Clicking a cluster zooms into it (level badge advances) and drills down by
    // setting a filter, which is reflected in the URL.
    click_nth(&page, ".cluster-bg", 0).await;
    wait_for(&page, ".tag-pill").await;
    wait_until(&page, "location.search.includes('filters')").await;
    wait_for_text(&page, ".level-badge", "1").await;
}
