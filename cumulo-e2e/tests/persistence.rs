#![cfg(feature = "browser")]

use cumulo_e2e::{click, dist, fill, reload, wait_until, Chrome, Site};

fn result_contains(label: &str) -> String {
    format!(
        "Array.from(document.querySelectorAll('.result-name')).some(e => e.textContent.includes({label:?}))"
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn added_resource_survives_a_reload() {
    let site = Site::serve(dist()).await;
    let chrome = Chrome::launch().await;
    let page = chrome.open(site.url("/")).await;

    let label = format!("e2e-persist-{}", std::process::id());

    click(&page, ".facet-results .add-resource-btn").await;
    fill(&page, ".form-panel .form-input", &label).await;
    click(&page, ".form-save-btn").await;

    wait_until(&page, &result_contains(&label)).await;

    // A reload drops the in-memory signal; the resource must be rehydrated
    // from the LocalStore (localStorage) to still be listed.
    reload(&page).await;
    wait_until(&page, &result_contains(&label)).await;
}
