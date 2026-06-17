use super::{
    controls::Controls, detail_panel::DetailPanel, entity_form::EntityForm,
    facet_sidebar::FacetSidebar, facet_view::FacetView, map_canvas::MapCanvas, palette::Palette,
    settings_modal::SettingsModal,
};
use crate::platform::{CategoryAttribute, Filters, Platform, ResourceAttribute, ResourceId};
use crate::storage::AppStorage;
use cumulo_model::{Bipartite, Forest, Resource};

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Route, Routes, A};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    let bipartite =
        RwSignal::<Bipartite<ResourceAttribute, CategoryAttribute>>::new(AppStorage::load());
    let selected_tags = RwSignal::new(Filters::default());
    let editing = RwSignal::new(Option::<Resource<ResourceAttribute, CategoryAttribute>>::None);
    let settings_open = RwSignal::new(false);
    let import_toast = RwSignal::new(Option::<String>::None);
    let return_to_settings = RwSignal::new(false);

    // When the resource form closes and it was opened from settings, reopen settings.
    Effect::new(move |_| {
        if editing.get().is_none() && return_to_settings.get_untracked() {
            return_to_settings.set(false);
            settings_open.set(true);
        }
    });

    view! {
        <div class="app">
            <header class="app-header">
                <A href=Platform::href("/") attr:class="app-logo">"☁ Cumulo"</A>
                <nav class="app-nav">
                    <A href=Platform::href("/facet") attr:class="nav-link">"ファセット"</A>
                    <A href=Platform::href("/map") attr:class="nav-link">"マップ"</A>
                </nav>
                <button
                    class="header-settings-btn"
                    on:click=move |_| settings_open.set(true)
                    title="設定"
                >
                    <Icon icon=icon::HiCog6ToothOutlineLg width="18" height="18" />
                </button>
            </header>

            <Palette bipartite=bipartite.read_only() selected_tags=selected_tags />

            <div class="route-content">
                <Routes fallback=|| view! { <div class="route-404">"ページが見つかりません"</div> }>
                    <Route path=path!("/") view=move || view! {
                        <FacetView bipartite=bipartite.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path=path!("/facet") view=move || view! {
                        <FacetView bipartite=bipartite.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path=path!("/map") view=move || view! {
                        <MapView bipartite=bipartite.read_only() selected_tags=selected_tags editing=editing />
                    }/>
                </Routes>
            </div>

            <EntityForm bipartite=bipartite editing=editing />
            <SettingsModal bipartite=bipartite open=settings_open import_toast=import_toast editing=editing return_to_settings=return_to_settings />

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
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let selected_entity_id = RwSignal::new(Option::<ResourceId>::None);
    let zoom_level = RwSignal::new(0u32);
    // ズーム軸＝軸（根カテゴリ）。既定は最初の根。セレクタの候補も根なので既定も根に揃える。
    // taxonomy が空の場合は表示対象がないため、使われないダミー id を割り当てる
    let zoom_dim = RwSignal::new({
        let s = bipartite.get_untracked();
        s.taxonomy
            .roots()
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_else(crate::platform::Platform::new_node_id)
    });

    view! {
        <div class="map-view">
            <Controls
                bipartite=bipartite
                selected_tags=selected_tags
                zoom_level=zoom_level.read_only()
                editing=editing
            />
            <div class="map-area">
                <FacetSidebar bipartite=bipartite selected_tags=selected_tags zoom_dim=zoom_dim />
                <MapCanvas
                    bipartite=bipartite
                    selected_tags=selected_tags
                    zoom_dim=zoom_dim
                    selected_entity=selected_entity_id
                    zoom_level=zoom_level
                />
                <DetailPanel bipartite=bipartite selected_id=selected_entity_id editing=editing />
            </div>
        </div>
    }
}
