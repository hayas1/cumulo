#![cfg(feature = "browser")]

use cumulo_e2e::{DropZone, Session};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dragging_a_category_reorders_the_tree() {
    let app = Session::open("/").await;

    app.click(".header-settings-btn").await;
    app.wait_for(".settings-modal").await;
    app.click_nth(".settings-tab", 2).await; // category tab
    app.wait_for(".category-node-row").await;

    let rows = app.count(".category-node-row").await;
    let first_before = app.text(".category-node-row").await;

    app.drag(
        ".category-drag-handle",
        rows - 1,
        ".category-node-row",
        0,
        DropZone::Before,
    )
    .await;

    app.wait_until(&format!(
        "!!document.querySelector('.category-node-row') && document.querySelector('.category-node-row').textContent !== {first_before:?}"
    ))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dragging_a_category_into_another_nests_it() {
    let app = Session::open("/").await;

    app.click(".header-settings-btn").await;
    app.wait_for(".settings-modal").await;
    app.click_nth(".settings-tab", 2).await; // category tab
    app.wait_for(".category-node-row").await;

    let rows = app.count(".category-node-row").await;
    let first_before = app.text(".category-node-row").await;

    app.drag(
        ".category-drag-handle",
        0,
        ".category-node-row",
        rows - 1,
        DropZone::Inside,
    )
    .await;

    app.wait_until(&format!(
        "!!document.querySelector('.category-node-row') && document.querySelector('.category-node-row').textContent !== {first_before:?}"
    ))
    .await;
}
