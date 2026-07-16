#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_a_resource_through_the_form_lists_it() {
    let app = Session::open("/").await;

    app.wait_for(".result-card").await;
    let before = app.count(".result-card").await;
    let label = format!("e2e-create-{}", std::process::id());

    app.click(".facet-results .add-resource-btn").await;
    app.fill(".form-panel .form-input", &label).await;
    app.click(".form-save-btn").await;

    app.wait_for_text(".result-name", &label).await;
    assert_eq!(app.count(".result-card").await, before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opening_the_editor_prefills_the_resource_fields() {
    let app = Session::open("/").await;

    app.wait_for(".result-card").await;
    app.click_nth(".result-edit-btn", 0).await;

    app.wait_for(".form-panel").await;
    app.wait_for_nonempty_value(".form-panel .form-input").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_a_resource_updates_it_in_place() {
    let app = Session::open("/").await;

    app.wait_for(".result-card").await;
    let before = app.count(".result-card").await;
    let label = format!("e2e-update-{}", std::process::id());

    app.click_nth(".result-edit-btn", 0).await;
    app.wait_for(".form-panel").await;
    app.set_value(".form-panel .form-input", &label).await;
    app.click(".form-save-btn").await;

    app.wait_for_text(".result-name", &label).await;
    assert_eq!(app.count(".result-card").await, before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_resource_removes_it_from_the_list() {
    let app = Session::open("/").await;

    app.click(".header-settings-btn").await;
    app.wait_for(".settings-modal").await;
    app.click_nth(".settings-tab", 1).await;
    app.wait_for(".resource-row").await;
    let before = app.count(".resource-row").await;

    app.click_nth(".resource-row-delete", 0).await;
    app.wait_for(".confirm-dialog").await;
    app.click(".confirm-ok").await;

    app.wait_for_count(".resource-row", before - 1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_a_resource_with_a_category_shows_a_chip() {
    let app = Session::open("/").await;

    app.wait_for(".result-card").await;
    let label = format!("e2e-cat-{}", std::process::id());

    app.click(".facet-results .add-resource-btn").await;
    app.fill(".form-panel .form-input", &label).await;
    app.click_nth(".form-panel .attr-chip", 0).await;
    app.click(".form-save-btn").await;

    app.wait_until(&format!(
        "Array.from(document.querySelectorAll('.result-card')).some(c => {{ const n = c.querySelector('.result-name'); return n && n.textContent.includes({label:?}) && c.querySelector('.result-chip'); }})"
    ))
    .await;
}
