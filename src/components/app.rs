use crate::model::FacetState;
use crate::storage::load_from_storage;
use leptos::*;
use super::{facet_panel::FacetPanel, result_panel::ResultPanel};

#[component]
pub fn App() -> impl IntoView {
    let (store, _set_store) = create_signal(load_from_storage());
    let facet_state = create_rw_signal(FacetState::default());

    view! {
        <div class="app">
            <header class="app-header">
                <span class="app-header-logo">"☁ Cumulo"</span>
                <span class="app-header-subtitle">
                    "マルチクラウド リソースナビゲーター"
                </span>
            </header>
            <div class="main-content">
                <FacetPanel store=store facet_state=facet_state />
                <ResultPanel store=store facet_state=facet_state />
            </div>
        </div>
    }
}
