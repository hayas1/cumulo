use crate::model::{AppStore, Resource};
use crate::storage::load_from_storage;
use leptos::*;
use leptos_router::*;
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
            </header>
            <Palette store=store.read_only() selected_tags=selected_tags />
            <div class="route-content">
                <Routes>
                    <Route
                        path="/"
                        view=move || {
                            view! {
                                <FacetView
                                    store=store.read_only()
                                    selected_tags=selected_tags
                                    editing=editing
                                />
                            }
                        }
                    />
                    <Route
                        path="/facet"
                        view=move || {
                            view! {
                                <FacetView
                                    store=store.read_only()
                                    selected_tags=selected_tags
                                    editing=editing
                                />
                            }
                        }
                    />
                    <Route
                        path="/map"
                        view=move || {
                            view! {
                                <MapView
                                    store=store.read_only()
                                    selected_tags=selected_tags
                                    editing=editing
                                />
                            }
                        }
                    />
                </Routes>
            </div>
            // モーダルフォーム（ルート切替でも消えないよう Routes の外に置く）
            <ResourceForm store=store editing=editing />
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
