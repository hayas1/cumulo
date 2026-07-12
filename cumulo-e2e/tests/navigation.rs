#![cfg(feature = "browser")]

use cumulo_e2e::Session;

// Nav buttons render in a fixed order in the header (see app.rs).
const NAV_FACET: usize = 0;
const NAV_MAP: usize = 1;
const NAV: &str = ".app-nav .nav-link";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nav_switches_view_and_syncs_the_url() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    app.click_nth(NAV, NAV_MAP).await;
    app.wait_for(".map-view").await;
    app.wait_until("location.search.includes('view=map')").await;

    app.click_nth(NAV, NAV_FACET).await;
    app.wait_for(".facet-view").await;
    app.wait_until("!location.search.includes('view=map')")
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_back_restores_the_previous_view() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(NAV, NAV_MAP).await;
    app.wait_for(".map-view").await;

    app.eval_bool("(window.history.back(), true)").await;
    app.wait_for(".facet-view").await;
}
