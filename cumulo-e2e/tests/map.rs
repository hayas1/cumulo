#![cfg(feature = "browser")]

use cumulo_e2e::{click_nth, dist, wait_for, wait_until, Chrome, Site};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selecting_a_map_axis_sets_the_zoom_axis() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/?view=map")).await;

    wait_for(&page, ".map-view").await;
    wait_for(&page, ".facet-panel-title-btn").await;

    // A non-default axis becomes the active zoom axis when clicked.
    click_nth(&page, ".facet-panel-title-btn", 1).await;
    wait_until(
        &page,
        "document.querySelectorAll('.facet-panel-title-btn')[1].classList.contains('active')",
    )
    .await;
    wait_until(&page, "location.search.includes('zoom_axis')").await;
}
