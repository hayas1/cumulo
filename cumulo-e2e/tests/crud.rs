#![cfg(feature = "browser")]

use cumulo_e2e::{
    click, click_nth, count, dist, fill, set_value, wait_for, wait_for_text, wait_until, Chrome,
    Site,
};

const LABEL: &str = ".form-panel .form-input";
const URL_FIELD: &str = "document.querySelectorAll('.form-panel .form-input')[1]";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_a_resource_through_the_form_lists_it() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".result-card").await;
    let before = count(&page, ".result-card").await;
    let label = format!("e2e-create-{}", std::process::id());

    click(&page, ".facet-results .add-resource-btn").await;
    fill(&page, LABEL, &label).await;
    click(&page, ".form-save-btn").await;

    wait_for_text(&page, ".result-name", &label).await;
    assert_eq!(count(&page, ".result-card").await, before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opening_the_editor_prefills_the_resource_fields() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".result-card").await;
    click_nth(&page, ".result-edit-btn", 0).await;

    wait_for(&page, ".form-panel").await;
    wait_until(
        &page,
        &format!("{URL_FIELD} && {URL_FIELD}.value.length > 0"),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_a_resource_updates_it_in_place() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    wait_for(&page, ".result-card").await;
    let before = count(&page, ".result-card").await;
    let label = format!("e2e-update-{}", std::process::id());

    click_nth(&page, ".result-edit-btn", 0).await;
    wait_for(&page, ".form-panel").await;
    set_value(&page, LABEL, &label).await;
    click(&page, ".form-save-btn").await;

    wait_for_text(&page, ".result-name", &label).await;
    assert_eq!(count(&page, ".result-card").await, before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_a_resource_removes_it_from_the_list() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    click(&page, ".header-settings-btn").await;
    wait_for(&page, ".settings-modal").await;
    click_nth(&page, ".settings-tab", 1).await; // resource tab
    wait_for(&page, ".resource-row").await;
    let before = count(&page, ".resource-row").await;

    click_nth(&page, ".resource-row-delete", 0).await;
    wait_for(&page, ".confirm-dialog").await;
    click(&page, ".confirm-ok").await;

    wait_until(
        &page,
        &format!(
            "document.querySelectorAll('.resource-row').length === {}",
            before - 1
        ),
    )
    .await;
}
