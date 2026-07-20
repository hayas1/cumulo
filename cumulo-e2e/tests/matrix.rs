#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clicking_a_cell_lists_the_intersection_and_opens_in_facet() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 2).await;
    app.wait_for(".matrix-view").await;
    app.wait_for(".matrix-cell").await;
    app.wait_for(".matrix-detail-empty").await;

    app.click_nth(".matrix-cell", 0).await;
    app.wait_for(".matrix-cell.matrix-cell-sel").await;
    app.wait_for(".matrix-detail-header").await;

    app.click(".matrix-detail-facet").await;
    app.wait_for(".facet-view").await;
    app.wait_for_query("filters").await;
}
