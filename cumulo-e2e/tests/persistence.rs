#![cfg(feature = "browser")]

use cumulo_e2e::Session;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn added_resource_survives_a_reload() {
    let app = Session::open("/").await;

    let label = format!("e2e-persist-{}", std::process::id());

    app.click(".facet-results .add-resource-btn").await;
    app.fill(".form-panel .form-input", &label).await;
    app.click(".form-save-btn").await;

    app.wait_for_text(".result-name", &label).await;

    app.reload().await;
    app.wait_for_text(".result-name", &label).await;
}
