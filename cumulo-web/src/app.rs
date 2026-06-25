use crate::category::{CategoryAttribute, Filters};
use crate::platform::Platform;
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
use crate::storage::AppStorage;
use crate::views::{facet::FacetView, map::MapView};
use cumulo_model::{Bipartite, Resource};

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;

/// マウントのエントリ。App を Router で包むだけの最上位ラッパ。
/// Router 依存をここに閉じ込め、lib.rs は mount するだけに保つ。
#[component]
pub fn Root() -> impl IntoView {
    view! {
        <Router base=Platform::router_base()>
            <App />
        </Router>
    }
}

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
                <A href=Platform::href("/") attr:class="app-logo">
                    <span class="app-logo-icon" aria-hidden="true" inner_html=include_str!("../public/favicon.svg") />
                    "Cumulo"
                </A>
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

            <ResourceForm bipartite=bipartite editing=editing />
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
