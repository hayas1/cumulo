#![cfg(feature = "browser")]

use cumulo_e2e::{click_nth, dist, eval_bool, wait_for, wait_until, Chrome, Site};

// Nav buttons render in a fixed order in the header (see app.rs).
const NAV_FACET: usize = 0;
const NAV_MAP: usize = 1;
const NAV: &str = ".app-nav .nav-link";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nav_switches_view_and_syncs_the_url() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".facet-view").await;

    click_nth(&page, NAV, NAV_MAP).await;
    wait_for(&page, ".map-view").await;
    wait_until(&page, "location.search.includes('view=map')").await;

    click_nth(&page, NAV, NAV_FACET).await;
    wait_for(&page, ".facet-view").await;
    wait_until(&page, "!location.search.includes('view=map')").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_back_restores_the_previous_view() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".facet-view").await;
    click_nth(&page, NAV, NAV_MAP).await;
    wait_for(&page, ".map-view").await;

    eval_bool(&page, "(window.history.back(), true)").await;
    wait_for(&page, ".facet-view").await;
}
