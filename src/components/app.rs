use crate::model::AppStore;
use crate::storage::load_from_storage;
use leptos::*;
use leptos_router::*;
use super::{
    controls::Controls,
    detail_panel::DetailPanel,
    facet_view::FacetView,
    map_canvas::MapCanvas,
    palette::Palette,
};

#[component]
pub fn App() -> impl IntoView {
    let (store, _set_store) = create_signal::<AppStore>(load_from_storage());

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
            <div class="route-content">
                <Routes>
                    <Route path="/" view=move || view! { <FacetView store=store /> } />
                    <Route path="/facet" view=move || view! { <FacetView store=store /> } />
                    <Route path="/map" view=move || view! { <MapView store=store /> } />
                </Routes>
            </div>
        </div>
    }
}

#[component]
fn MapView(store: ReadSignal<AppStore>) -> impl IntoView {
    let selected_tags = create_rw_signal(Vec::<(String, String)>::new());
    let selected_resource_id = create_rw_signal(Option::<String>::None);
    let zoom_level = create_rw_signal(0u32);
    let zoom_axes = create_rw_signal({
        let cfg = store.get_untracked();
        vec![cfg.map_config.zoom_axes[0].clone()]
    });

    view! {
        <div class="map-view">
            <Palette store=store selected_tags=selected_tags />
            <Controls
                store=store
                selected_tags=selected_tags
                zoom_axes=zoom_axes
                zoom_level=zoom_level.read_only()
            />
            <div class="map-area">
                <MapCanvas
                    store=store
                    selected_tags=selected_tags
                    zoom_axes=zoom_axes
                    selected_resource=selected_resource_id
                    zoom_level=zoom_level
                />
                <DetailPanel store=store selected_id=selected_resource_id />
            </div>
        </div>
    }
}
