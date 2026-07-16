#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nav_switches_view_and_syncs_the_url() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;

    app.click_nth(".app-nav .nav-link", 1).await;
    app.wait_for(".map-view").await;
    app.wait_for_query("view=map").await;

    app.click_nth(".app-nav .nav-link", 0).await;
    app.wait_for(".facet-view").await;
    app.wait_for_no_query("view=map").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_back_restores_the_previous_view() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 1).await;
    app.wait_for(".map-view").await;

    app.eval_bool("(window.history.back(), true)").await;
    app.wait_for(".facet-view").await;
}
