use crate::io::{export_json, import_json, trigger_download};
use crate::model::{AppStore, Resource};
use crate::storage::save_to_storage;
use icondata as icon;
use leptos::*;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use super::dimensions_tab::DimensionsTab;
use super::resources_tab::ResourcesTab;

#[component]
pub fn SettingsModal(
    store: RwSignal<AppStore>,
    open: RwSignal<bool>,
    import_toast: RwSignal<Option<String>>,
    editing: RwSignal<Option<Resource>>,
    return_to_settings: RwSignal<bool>,
) -> impl IntoView {
    let active_tab = create_rw_signal("data".to_string());
    let file_input_ref = create_node_ref::<html::Input>();

    let on_export = move |_| {
        let s = store.get_untracked();
        let json = export_json(&s);
        let date = js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default()
            .chars()
            .take(10)
            .collect::<String>();
        trigger_download(&format!("cumulo-{date}.json"), &json);
    };

    let on_import_click = move |_| {
        if let Some(el) = file_input_ref.get() {
            el.click();
        }
    };

    let on_file_change = move |ev: web_sys::Event| {
        let input: web_sys::HtmlInputElement = ev.target().unwrap().dyn_into().unwrap();
        let input_clone = input.clone();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let text_promise = file.text();
                spawn_local(async move {
                    match JsFuture::from(text_promise).await {
                        Ok(js_text) => {
                            let json = js_text.as_string().unwrap_or_default();
                            match import_json(&json) {
                                Ok(imported) => {
                                    let msg = format!(
                                        "インポート完了: リソース {}件、ディメンション {}件",
                                        imported.resources.len(),
                                        imported.dimensions.len(),
                                    );
                                    store.set(imported);
                                    save_to_storage(&store.get_untracked());
                                    open.set(false);
                                    import_toast.set(Some(msg));
                                }
                                Err(e) => {
                                    web_sys::console::error_1(
                                        &format!("[cumulo] import failed: {e}").into(),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("[cumulo] file read failed: {e:?}").into(),
                            );
                        }
                    }
                    input_clone.set_value("");
                });
            }
        }
    };

    view! {
        <Show when=move || open.get()>
            <div class="settings-backdrop" on:click=move |_| open.set(false) />
            <div class="settings-modal">
                <div class="settings-header">
                    <span class="settings-title">"設定"</span>
                    <button class="settings-close" on:click=move |_| open.set(false)>"×"</button>
                </div>
                <div class="settings-body">
                    <nav class="settings-sidebar">
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "data"
                            on:click=move |_| active_tab.set("data".into())
                        >
                            "データ"
                        </button>
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "resource"
                            on:click=move |_| active_tab.set("resource".into())
                        >
                            "リソース"
                        </button>
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "dim"
                            on:click=move |_| active_tab.set("dim".into())
                        >
                            "ディメンション"
                        </button>
                    </nav>
                    <div class="settings-content">
                        <input
                            node_ref=file_input_ref
                            type="file"
                            accept=".json"
                            style="display:none"
                            on:change=on_file_change
                        />
                        {move || {
                            let tab = active_tab.get();
                            match tab.as_str() {
                                "dim" => view! {
                                    <DimensionsTab store=store />
                                }.into_view(),
                                "resource" => view! {
                                    <ResourcesTab store=store editing=editing settings_open=open return_to_settings=return_to_settings />
                                }.into_view(),
                                "data" => view! {
                                    <div class="settings-section">
                                        <h3 class="settings-section-title">"エクスポート"</h3>
                                        <button class="settings-action-btn" on:click=on_export>
                                            <Icon icon=icon::HiArrowDownTrayOutlineLg width="15" height="15" />
                                            "エクスポート"
                                        </button>
                                    </div>
                                    <div class="settings-section">
                                        <h3 class="settings-section-title">"インポート"</h3>
                                        <button class="settings-action-btn" on:click=on_import_click>
                                            <Icon icon=icon::HiArrowUpTrayOutlineLg width="15" height="15" />
                                            "インポート..."
                                        </button>
                                    </div>
                                }.into_view(),
                                _ => view! { <div /> }.into_view(),
                            }
                        }}
                    </div>
                </div>
            </div>
        </Show>
    }
}
