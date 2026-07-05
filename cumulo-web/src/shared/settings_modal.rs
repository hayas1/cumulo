use crate::category::attributes_tab::AttributesTab;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::resource::entities_tab::EntitiesTab;
use crate::resource::ResourceAttribute;
use crate::shared::ConfirmDialog;
use cumulo_model::ExportData;
use cumulo_model::Resource;
use icondata as icon;
use leptos::html::Input;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// 設定モーダル⇄リソース編集フォームの往復フロー。
/// 「設定→編集フォームを開く」と「フォームを閉じたら設定へ戻す」を対で持つ。
/// この 3 signal はこの protocol でしか一緒に動かないので、束ねて出入りを型に閉じる。
#[derive(Clone, Copy)]
pub struct SettingsEditFlow {
    pub editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    pub settings_open: RwSignal<bool>,
    pub return_to_settings: RwSignal<bool>,
}

impl SettingsEditFlow {
    /// 設定を閉じて編集フォームを開く。戻り先が設定だと印を付けておく。
    pub fn open_editor(&self, resource: Resource<ResourceAttribute, CategoryAttribute>) {
        self.return_to_settings.set(true);
        self.editing.set(Some(resource));
        self.settings_open.set(false);
    }

    /// 編集フォームが閉じたとき、設定から来ていたら設定へ戻す。`editing` を購読するので Effect 内で呼ぶ。
    pub fn return_from_editor(&self) {
        if self.editing.get().is_none() && self.return_to_settings.get_untracked() {
            self.return_to_settings.set(false);
            self.settings_open.set(true);
        }
    }
}

#[component]
pub fn SettingsModal(
    client: Client,
    open: RwSignal<bool>,
    import_toast: RwSignal<Option<String>>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    return_to_settings: RwSignal<bool>,
) -> impl IntoView {
    let active_tab = RwSignal::new("data".to_string());
    let file_input_ref = NodeRef::<Input>::new();
    let confirm_clear = RwSignal::new(false);

    // 設定⇄編集フォームの往復フロー。開く側は EntitiesTab、戻す側はこの下の Effect。
    let flow = SettingsEditFlow {
        editing,
        settings_open: open,
        return_to_settings,
    };
    // フォームが閉じたら（設定から来ていれば）設定に戻す。
    Effect::new(move |_| flow.return_from_editor());

    let do_export = move || {
        let s = client.read().get_untracked();
        let json = ExportData::new(s, Platform::now_iso()).to_json();
        let date = Platform::now_iso().chars().take(10).collect::<String>();
        Platform::trigger_download(&format!("cumulo-{date}.json"), &json);
    };

    let on_export = move |_| do_export();

    let on_clear = move |_| {
        // 消去前に必ずエクスポート（強制バックアップ）
        do_export();
        client.clear();
        confirm_clear.set(false);
        open.set(false);
        import_toast.set(Some("ローカルのデータを削除しました".to_string()));
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
                            match ExportData::parse(&json) {
                                Ok(imported) => {
                                    let msg = format!(
                                        "インポート完了: リソース {}件、カテゴリ {}件",
                                        imported.catalog.len(),
                                        imported.taxonomy.len(),
                                    );
                                    client.set(imported);
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
                            "カテゴリ"
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
                                    <AttributesTab client=client />
                                }.into_any(),
                                "resource" => view! {
                                    <EntitiesTab client=client flow=flow />
                                }.into_any(),
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
                                    <div class="settings-section">
                                        <h3 class="settings-section-title settings-danger-title">"ローカルのデータを削除"</h3>
                                        <button
                                            class="settings-action-btn settings-danger-btn"
                                            on:click=move |_| confirm_clear.set(true)
                                        >
                                            <Icon icon=icon::HiTrashOutlineLg width="15" height="15" />
                                            "エクスポートして消去"
                                        </button>
                                    </div>
                                }.into_any(),
                                _ => view! { <div /> }.into_any(),
                            }
                        }}
                    </div>
                </div>
            </div>

            {move || confirm_clear.get().then(|| view! {
                <ConfirmDialog
                    message="エクスポートしてから消去します。"
                    confirm_label="エクスポートして消去"
                    on_confirm=Callback::new(on_clear)
                    on_cancel=Callback::new(move |_| confirm_clear.set(false))
                />
            })}
        </Show>
    }
}
