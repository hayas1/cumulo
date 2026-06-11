use super::{
    controls::Controls, detail_panel::DetailPanel, facet_sidebar::FacetSidebar,
    facet_view::FacetView, map_canvas::MapCanvas, palette::Palette, resource_form::ResourceForm,
    settings_modal::SettingsModal,
};
use crate::model::{AppStore, Resource};

use icondata as icon;
use leptos::*;
use leptos_icons::Icon;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    let store = create_rw_signal::<AppStore>(AppStore::load_from_storage());
    let selected_tags = create_rw_signal(Vec::<(String, String)>::new());
    let editing = create_rw_signal(Option::<Resource>::None);
    let settings_open = create_rw_signal(false);
    let import_toast = create_rw_signal(Option::<String>::None);
    let return_to_settings = create_rw_signal(false);

    // When the resource form closes and it was opened from settings, reopen settings.
    create_effect(move |_| {
        if editing.get().is_none() && return_to_settings.get_untracked() {
            return_to_settings.set(false);
            settings_open.set(true);
        }
    });

    view! {
        <div class="app">
            <header class="app-header">
                <A href="/" class="app-logo">"☁ Cumulo"</A>
                <nav class="app-nav">
                    <A href="/facet" class="nav-link">"ファセット"</A>
                    <A href="/map" class="nav-link">"マップ"</A>
                </nav>
                <button
                    class="header-settings-btn"
                    on:click=move |_| settings_open.set(true)
                    title="設定"
                >
                    <Icon icon=icon::HiCog6ToothOutlineLg width="18" height="18" />
                </button>
            </header>

            <Palette store=store.read_only() selected_tags=selected_tags />

            <div class="route-content">
                <Routes>
                    <Route path="/" view=move || view! {
                        <FacetView store=store.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path="/facet" view=move || view! {
                        <FacetView store=store.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path="/map" view=move || view! {
                        <MapView store=store.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                </Routes>
            </div>

            <ResourceForm store=store editing=editing />
            <SettingsModal store=store open=settings_open import_toast=import_toast editing=editing return_to_settings=return_to_settings />

            {move || import_toast.get().map(|msg| view! {
                <div class="import-toast">
                    <span class="import-toast-msg">{msg}</span>
                    <button
                        class="import-toast-close"
                        on:click=move |_| import_toast.set(None)
                    >
                        "×"
                    </button>
                </div>
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
    // ズーム軸＝ディメンション。既定は一番上の facet（最初のディメンション）。
    let zoom_dim = create_rw_signal({
        let s = store.get_untracked();
        s.dimensions
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_default()
    });

    view! {
        <div class="map-view">
            <Controls
                store=store
                selected_tags=selected_tags
                zoom_level=zoom_level.read_only()
                editing=editing
            />
            <div class="map-area">
                <FacetSidebar store=store selected_tags=selected_tags zoom_dim=zoom_dim />
                <MapCanvas
                    store=store
                    selected_tags=selected_tags
                    zoom_dim=zoom_dim
                    selected_resource=selected_resource_id
                    zoom_level=zoom_level
                />
                <DetailPanel store=store selected_id=selected_resource_id editing=editing />
            </div>
        </div>
    }
}
