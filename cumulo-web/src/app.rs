use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
use crate::storage::{DynStore, LOCAL_STORE};
use crate::views::{facet::FacetView, map::MapView};
use cumulo_model::Resource;

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::Router;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

#[component]
pub fn RootLocalStore() -> impl IntoView {
    view! {
        <Root store=&LOCAL_STORE />
    }
}

#[component]
pub fn Root(store: &'static DynStore) -> impl IntoView {
    view! {
        <Router base=Platform::router_base()>
            <I18nContextProvider>
                <App store=store />
            </I18nContextProvider>
        </Router>
    }
}

#[component]
pub fn App(store: &'static DynStore) -> impl IntoView {
    let i18n = use_i18n();
    let client = Client::new(store);
    let editing = RwSignal::new(Option::<Resource<ResourceAttribute, CategoryAttribute>>::None);
    let settings_open = RwSignal::new(false);
    let import_toast = RwSignal::new(Option::<String>::None);
    let return_to_settings = RwSignal::new(false);

    let location = use_location();
    let navigate = use_navigate();
    let query = location.query;
    let pathname = location.pathname;
    let state = RwSignal::new(query.with_untracked(|p| QueryState::resolved_from(p, &client)));
    let view = Memo::new(move |_| state.with(|q| q.view));

    Effect::new(move |_| {
        if let Some(next) = state.get_untracked().adopt_url(&query.get(), &client) {
            state.set(next);
        }
    });
    Effect::new(move |_| {
        let desired = state.get();
        if let Some((url, push)) =
            desired.url_update(&query.get_untracked(), &pathname.get_untracked())
        {
            navigate(
                &url,
                NavigateOptions {
                    resolve: false,
                    replace: !push,
                    scroll: false,
                    ..Default::default()
                },
            );
        }
    });

    view! {
        <div class="app">
            <header class="app-header">
                <a href=Platform::href("/") rel="external" class="app-logo">
                    <span class="app-logo-icon" aria-hidden="true" inner_html=include_str!("../public/favicon.svg") />
                    "Cumulo"
                </a>
                <nav class="app-nav">
                    <button
                        class="nav-link"
                        class:active=move || view.get() == View::Facet
                        on:click=move |_| state.update(|q| q.view = View::Facet)
                    >
                        {t!(i18n, nav_facet)}
                    </button>
                    <button
                        class="nav-link"
                        class:active=move || view.get() == View::Map
                        on:click=move |_| state.update(|q| q.view = View::Map)
                    >
                        {t!(i18n, nav_map)}
                    </button>
                </nav>
                <button
                    class="header-settings-btn"
                    on:click=move |_| settings_open.set(true)
                    title=move || t_string!(i18n, settings_title)
                >
                    <Icon icon=icon::HiCog6ToothOutlineLg width="18" height="18" />
                </button>
            </header>

            <Palette client=client state=state />

            <div class="route-content">
                {move || match view.get() {
                    View::Facet => view! {
                        <FacetView client=client state=state editing=editing />
                    }
                    .into_any(),
                    View::Map => view! {
                        <MapView client=client state=state editing=editing />
                    }
                    .into_any(),
                }}
            </div>

            <ResourceForm client=client editing=editing />
            <SettingsModal client=client open=settings_open import_toast=import_toast editing=editing return_to_settings=return_to_settings />

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

            {move || client.toast().get().map(|msg| view! {
                <div class="import-toast error-toast">
                    <span class="import-toast-msg">{msg}</span>
                    <button
                        class="import-toast-close"
                        on:click=move |_| client.toast().set(None)
                    >
                        "×"
                    </button>
                </div>
            })}
        </div>
    }
}
