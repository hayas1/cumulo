use crate::category::attributes_tab::AttributesTab;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::i18n::*;
use crate::locale::Lang;
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

#[derive(Clone, Copy)]
pub struct SettingsEditFlow {
    pub editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    pub settings_open: RwSignal<bool>,
    pub return_to_settings: RwSignal<bool>,
}

impl SettingsEditFlow {
    pub fn open_editor(&self, resource: Resource<ResourceAttribute, CategoryAttribute>) {
        self.return_to_settings.set(true);
        self.editing.set(Some(resource));
        self.settings_open.set(false);
    }

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
    let i18n = use_i18n();
    let active_tab = RwSignal::new("data".to_string());
    let file_input_ref = NodeRef::<Input>::new();
    let confirm_clear = RwSignal::new(false);

    let flow = SettingsEditFlow {
        editing,
        settings_open: open,
        return_to_settings,
    };
    Effect::new(move |_| flow.return_from_editor());

    let do_export = move || {
        let s = client.read().get_untracked();
        let json = ExportData::new(s, Platform::now_iso()).to_json();
        let date = Platform::now_iso().chars().take(10).collect::<String>();
        Platform::trigger_download(&format!("cumulo-{date}.json"), &json);
    };

    let on_export = move |_| do_export();

    let on_clear = move |_| {
        do_export();
        client.clear();
        confirm_clear.set(false);
        open.set(false);
        import_toast.set(Some(t_string!(i18n, settings_cleared).to_string()));
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
                                    let msg = t_string!(
                                        i18n,
                                        import_done,
                                        resources = imported.catalog.len(),
                                        categories = imported.taxonomy.len(),
                                    )
                                    .to_string();
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
                    <span class="settings-title">{t!(i18n, settings_title)}</span>
                    <select
                        class="settings-lang"
                        prop:value=move || Lang::from(i18n.get_locale()).as_str()
                        on:change=move |ev| {
                            if let Ok(lang) = event_target_value(&ev).parse::<Lang>() {
                                i18n.set_locale(lang.into());
                            }
                        }
                    >
                        {Lang::ALL
                            .into_iter()
                            .map(|lang| {
                                let label = match lang {
                                    Lang::En => t_string!(i18n, lang_en),
                                    Lang::Ja => t_string!(i18n, lang_ja),
                                };
                                view! { <option value=lang.as_str()>{label}</option> }
                            })
                            .collect_view()}
                    </select>
                    <button class="settings-close" on:click=move |_| open.set(false)>"×"</button>
                </div>
                <div class="settings-body">
                    <nav class="settings-sidebar">
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "data"
                            on:click=move |_| active_tab.set("data".into())
                        >
                            {t!(i18n, settings_tab_data)}
                        </button>
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "resource"
                            on:click=move |_| active_tab.set("resource".into())
                        >
                            {t!(i18n, settings_tab_resource)}
                        </button>
                        <button
                            class="settings-tab"
                            class:active=move || active_tab.get() == "dim"
                            on:click=move |_| active_tab.set("dim".into())
                        >
                            {t!(i18n, settings_tab_category)}
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
                                        <h3 class="settings-section-title">{t!(i18n, settings_export)}</h3>
                                        <button class="settings-action-btn" on:click=on_export>
                                            <Icon icon=icon::HiArrowDownTrayOutlineLg width="15" height="15" />
                                            {t!(i18n, settings_export)}
                                        </button>
                                    </div>
                                    <div class="settings-section">
                                        <h3 class="settings-section-title">{t!(i18n, settings_import)}</h3>
                                        <button class="settings-action-btn" on:click=on_import_click>
                                            <Icon icon=icon::HiArrowUpTrayOutlineLg width="15" height="15" />
                                            {t!(i18n, settings_import_action)}
                                        </button>
                                    </div>
                                    <div class="settings-section">
                                        <h3 class="settings-section-title settings-danger-title">{t!(i18n, settings_delete_local)}</h3>
                                        <button
                                            class="settings-action-btn settings-danger-btn"
                                            on:click=move |_| confirm_clear.set(true)
                                        >
                                            <Icon icon=icon::HiTrashOutlineLg width="15" height="15" />
                                            {t!(i18n, settings_export_and_clear)}
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
                    message=t_string!(i18n, settings_clear_confirm)
                    confirm_label=t_string!(i18n, settings_export_and_clear)
                    on_confirm=Callback::new(on_clear)
                    on_cancel=Callback::new(move |_| confirm_clear.set(false))
                />
            })}
        </Show>
    }
}
