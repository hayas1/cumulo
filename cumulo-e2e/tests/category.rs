#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_a_category_lists_it() {
    let app = Session::open("/").await;

    app.click(".header-settings-btn").await;
    app.wait_for(".settings-modal").await;
    app.click_nth(".settings-tab", 2).await;
    app.wait_for(".category-node-row").await;
    let before = app.count(".category-node-row").await;

    app.click_nth(".category-add-root", 0).await;
    app.wait_for_count(".category-node-row", before + 1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_category_removes_it() {
    let app = Session::open("/").await;

    app.click(".header-settings-btn").await;
    app.wait_for(".settings-modal").await;
    app.click_nth(".settings-tab", 2).await;
    app.wait_for(".category-node-row").await;
    let before = app.count(".category-node-row").await;

    app.click_nth(".category-node-delete", before - 1).await;
    app.wait_for(".confirm-dialog").await;
    app.click(".confirm-ok").await;

    app.wait_for_count(".category-node-row", before - 1).await;
}
