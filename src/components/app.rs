use crate::io::{export_json, import_json, trigger_download};
use crate::model::{AppStore, Resource};
use crate::storage::{load_from_storage, save_to_storage};
use leptos::*;
use leptos_router::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use super::{
    controls::Controls,
    detail_panel::DetailPanel,
    facet_sidebar::FacetSidebar,
    facet_view::FacetView,
    map_canvas::MapCanvas,
    palette::Palette,
    resource_form::ResourceForm,
};

#[component]
pub fn App() -> impl IntoView {
    let store = create_rw_signal::<AppStore>(load_from_storage());
    let selected_tags = create_rw_signal(Vec::<(String, String)>::new());
    let editing = create_rw_signal(Option::<Resource>::None);
    let import_toast = create_rw_signal(Option::<String>::None);

    let file_input_ref = create_node_ref::<html::Input>();

    let on_export = move |_| {
        let s = store.get_untracked();
        let json = export_json(&s.resources, &s.dimensions);
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
                                    let r_count = imported.resources.len();
                                    let d_count = imported.dimensions.len();
                                    store.update(|s| {
                                        for r in imported.resources {
                                            if let Some(pos) =
                                                s.resources.iter().position(|x| x.id == r.id)
                                            {
                                                s.resources[pos] = r;
                                            } else {
                                                s.resources.push(r);
                                            }
                                        }
                                        for d in imported.dimensions {
                                            if let Some(pos) =
                                                s.dimensions.iter().position(|x| x.id == d.id)
                                            {
                                                s.dimensions[pos] = d;
                                            } else {
                                                s.dimensions.push(d);
                                            }
                                        }
                                    });
                                    save_to_storage(&store.get_untracked());

                                    let msg = if d_count > 0 {
                                        format!(
                                            "インポート完了: リソース {}件、ディメンション {}件",
                                            r_count, d_count
                                        )
                                    } else {
                                        format!("インポート完了: リソース {}件", r_count)
                                    };
                                    import_toast.set(Some(msg));

                                    // Auto-dismiss after 4s using a JS Promise timeout
                                    let dismiss = import_toast;
                                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                                        web_sys::window()
                                            .unwrap()
                                            .set_timeout_with_callback_and_timeout_and_arguments_0(
                                                &resolve, 4000,
                                            )
                                            .unwrap();
                                    });
                                    let _ = JsFuture::from(promise).await;
                                    dismiss.set(None);
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
        <div class="app">
            <header class="app-header">
                <A href="/" class="app-logo">
                    "☁ Cumulo"
                </A>
                <nav class="app-nav">
                    <A href="/facet" class="nav-link">
                        "ファセット"
                    </A>
                    <A href="/map" class="nav-link">
                        "マップ"
                    </A>
                </nav>
                <div class="header-actions">
                    <input
                        node_ref=file_input_ref
                        type="file"
                        accept=".json"
                        style="display:none"
                        on:change=on_file_change
                    />
                    <button class="header-btn" on:click=on_import_click>
                        "インポート"
                    </button>
                    <button class="header-btn" on:click=on_export>
                        "エクスポート"
                    </button>
                </div>
            </header>
            <Palette store=store.read_only() selected_tags=selected_tags />
            <div class="route-content">
                <Routes>
                    <Route
                        path="/"
                        view=move || view! {
                            <FacetView
                                store=store.read_only()
                                selected_tags=selected_tags
                                editing=editing
                            />
                        }
                    />
                    <Route
                        path="/facet"
                        view=move || view! {
                            <FacetView
                                store=store.read_only()
                                selected_tags=selected_tags
                                editing=editing
                            />
                        }
                    />
                    <Route
                        path="/map"
                        view=move || view! {
                            <MapView
                                store=store.read_only()
                                selected_tags=selected_tags
                                editing=editing
                            />
                        }
                    />
                </Routes>
            </div>
            <ResourceForm store=store editing=editing />

            // インポート完了トースト
            {move || import_toast.get().map(|msg| view! {
                <div class="import-toast">{msg}</div>
            })}
        </div>
    }
}

#[component]
fn MapView(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let selected_resource_id = create_rw_signal(Option::<String>::None);
    let zoom_level = create_rw_signal(0u32);
    let zoom_axes = create_rw_signal({
        let cfg = store.get_untracked();
        vec![cfg.map_config.zoom_axes[0].clone()]
    });

    view! {
        <div class="map-view">
            <Controls
                store=store
                selected_tags=selected_tags
                zoom_axes=zoom_axes
                zoom_level=zoom_level.read_only()
                editing=editing
            />
            <div class="map-area">
                <FacetSidebar store=store selected_tags=selected_tags />
                <MapCanvas
                    store=store
                    selected_tags=selected_tags
                    zoom_axes=zoom_axes
                    selected_resource=selected_resource_id
                    zoom_level=zoom_level
                />
                <DetailPanel store=store selected_id=selected_resource_id editing=editing />
            </div>
        </div>
    }
}
