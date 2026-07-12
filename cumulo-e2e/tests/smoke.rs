#![cfg(feature = "browser")]

use cumulo_e2e::{dist, wait_for, Chrome, Site};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_shell_mounts_on_the_default_route() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;

    let page = chrome.open(site.url("/")).await;
    wait_for(&page, ".app").await;
    wait_for(&page, ".app-nav .nav-link.active").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_selects_the_map_view() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;

    let page = chrome.open(site.url("/?view=map")).await;
    wait_for(&page, ".map-view").await;
}
