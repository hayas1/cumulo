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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expanding_a_row_reveals_its_child_rows() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 2).await;
    app.wait_for(".matrix-view").await;
    app.wait_for(".matrix-rowhead .matrix-tree-chevron").await;

    let before = app.count(".matrix-rowhead").await;
    app.click_nth(".matrix-rowhead .matrix-tree-chevron", 0).await;
    app.wait_until(&format!(
        "document.querySelectorAll('.matrix-rowhead').length > {before}"
    ))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expanding_a_child_keeps_the_parent_expanded() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 2).await;
    app.wait_for(".matrix-view").await;
    app.wait_for(".matrix-rowhead .matrix-tree-chevron").await;

    app.click_nth(".matrix-rowhead .matrix-tree-chevron", 0).await;
    app.wait_until(
        "document.querySelectorAll('.matrix-rowhead .matrix-tree-chevron.open').length === 1",
    )
    .await;

    app.click_nth(".matrix-rowhead .matrix-tree-chevron", 1).await;
    app.wait_until(
        "document.querySelectorAll('.matrix-rowhead .matrix-tree-chevron.open').length === 2",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expanding_a_sibling_collapses_the_previous_branch() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 2).await;
    app.wait_for(".matrix-view").await;
    app.wait_for(".matrix-rowhead .matrix-tree-chevron").await;

    app.click_nth(".matrix-rowhead .matrix-tree-chevron", 0).await;
    app.click_nth(".matrix-rowhead .matrix-tree-chevron", 1).await;
    app.wait_until(
        "document.querySelectorAll('.matrix-rowhead .matrix-tree-chevron.open').length === 2",
    )
    .await;

    let last = app.count(".matrix-rowhead .matrix-tree-chevron").await - 1;
    app.click_nth(".matrix-rowhead .matrix-tree-chevron", last)
        .await;
    app.wait_until(
        "document.querySelectorAll('.matrix-rowhead .matrix-tree-chevron.open').length === 1",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn picking_a_row_axis_in_the_facet_updates_the_selection() {
    let app = Session::open("/").await;

    app.wait_for(".facet-view").await;
    app.click_nth(".app-nav .nav-link", 2).await;
    app.wait_for(".matrix-view").await;
    app.wait_for(".matrix-axis-facet .facet-panel-title-btn").await;

    app.click_nth(".matrix-axis-facet .facet-panel-title-btn", 1)
        .await;
    app.wait_for_class(".matrix-axis-facet .facet-panel-title-btn", 1, "selected")
        .await;
}
