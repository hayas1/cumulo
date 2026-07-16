#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn app_shell_mounts_on_the_default_route() {
    let app = Session::open("/").await;
    app.wait_for(".app").await;
    app.wait_for(".app-nav .nav-link.active").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_link_selects_the_map_view() {
    let app = Session::open("/?view=map").await;
    app.wait_for(".map-view").await;
}
