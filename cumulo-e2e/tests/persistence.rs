#![cfg(feature = "browser")]

use cumulo_e2e::{click, dist, fill, reload, wait_for_text, Chrome, Site};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn added_resource_survives_a_reload() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    let label = format!("e2e-persist-{}", std::process::id());

    click(&page, ".facet-results .add-resource-btn").await;
    fill(&page, ".form-panel .form-input", &label).await;
    click(&page, ".form-save-btn").await;

    wait_for_text(&page, ".result-name", &label).await;

    // A reload drops the in-memory signal; the resource must be rehydrated
    // from the LocalStore (localStorage) to still be listed.
    reload(&page).await;
    wait_for_text(&page, ".result-name", &label).await;
}
