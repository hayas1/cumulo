#![cfg(feature = "browser")]

use cumulo_e2e::{dist, wait_for, wait_for_class, wait_for_text, Chrome, Site};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_prefilters_the_facet_list() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/?filters.platform=gcp")).await;

    wait_for(&page, ".facet-view").await;

    // The URL filter is adopted as an active pill and the list renders narrowed.
    wait_for_text(&page, ".tag-pill", "gcp").await;
    wait_for(&page, ".result-card").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_restores_map_axis_and_filter() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome
        .open(site.url("/?view=map&zoom_axis=product&filters.platform=gcp"))
        .await;

    wait_for(&page, ".map-view").await;

    // The map opens on the deep-linked axis (product, the second root) already
    // filtered by the deep-linked tag.
    wait_for_class(&page, ".facet-panel-title-btn", 1, "active").await;
    wait_for_text(&page, ".tag-pill", "gcp").await;
}
