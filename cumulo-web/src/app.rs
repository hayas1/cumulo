use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
use crate::state::State;
use crate::views::{facet::FacetView, map::MapView};
use cumulo_model::Resource;

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Router, A};
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::NavigateOptions;

/// マウントのエントリ。App を Router で包むだけの最上位ラッパ。
/// view はクエリで持つので per-view ルートは無いが、クエリ系 hook の文脈確保に Router は要る。
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
    let client = Client::load();
    // URL に載る UI 状態（view / 絞り込み / ズーム軸）を 1 ハンドルに束ねて prop-drill する。
    let state = State::new(client);
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

    // URL ⇄ state を双方向同期する。同じ bipartite を持つ相手に URL を共有すれば同じ画面
    // （view・絞り込み・マップのズーム）が開ける。signal 群と QueryState の対応は State が持ち、
    // ここは router 文脈での購読（query）と書き込み（navigate）だけを担う。同値ガードで往復を断つ。
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();

    // URL → state: 共有リンクを開いた時・ブラウザの戻る/進み時にクエリから復元する。
    Effect::new(move |_| state.apply(query.with(QueryState::from_params)));

    // state → URL: view/絞り込み/ズーム軸の変更のたびにクエリを書き換える。
    Effect::new(move |_| {
        let current = query.with_untracked(QueryState::from_params);
        let desired = state.overlay(current.clone());
        // URL→state 経由で来た更新を打ち返さない（クエリが既に同じ状態なら何もしない）。
        if current == desired {
            return;
        }
        // view 切替は履歴に積み（戻るで前の view へ）、絞り込み等の微調整は replace で履歴を汚さない。
        let push = current.view != desired.view;
        let url = format!(
            "{}{}",
            location.pathname.get_untracked(),
            desired.to_params().to_query_string()
        );
        navigate(
            &url,
            NavigateOptions {
                resolve: false,
                replace: !push,
                scroll: false,
                ..Default::default()
            },
        );
    });

    view! {
        <div class="app">
            <header class="app-header">
                <A href=Platform::href("/") attr:class="app-logo">
                    <span class="app-logo-icon" aria-hidden="true" inner_html=include_str!("../public/favicon.svg") />
                    "Cumulo"
                </A>
                <nav class="app-nav">
                    <button
                        class="nav-link"
                        class:active=move || state.view.get() == View::Facet
                        on:click=move |_| state.view.set(View::Facet)
                    >
                        "ファセット"
                    </button>
                    <button
                        class="nav-link"
                        class:active=move || state.view.get() == View::Map
                        on:click=move |_| state.view.set(View::Map)
                    >
                        "マップ"
                    </button>
                </nav>
                <button
                    class="header-settings-btn"
                    on:click=move |_| settings_open.set(true)
                    title="設定"
                >
                    <Icon icon=icon::HiCog6ToothOutlineLg width="18" height="18" />
                </button>
            </header>

            <Palette client=client state=state />

            <div class="route-content">
                {move || match state.view.get() {
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
        </div>
    }
}
