use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
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

    // URL に載る UI 状態（view / 絞り込み / ズーム軸）を 1 つの signal(QueryState) で持ち、
    // prop-drill する。zoom_axis は URL 未指定でも既定軸（先頭根）に解決し、常に具体値に
    // する（＝URL にも既定を出す）。field 単位の細粒度は各所で Memo を派生して得る。
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();
    let state = RwSignal::new({
        let mut qs = query.with_untracked(QueryState::from_params);
        qs.zoom_axis = Some(qs.zoom_axis.unwrap_or_else(|| client.default_zoom_axis()));
        qs
    });
    // view だけを購読する Memo。これが無いと下の match / nav が state 全体を購読し、
    // 絞り込み変更のたびに FacetView/MapView を作り直して（再マウント）しまう。
    let view = Memo::new(move |_| state.with(|q| q.view));

    // URL → state: 共有リンクを開いた時・ブラウザの戻る/進み時にクエリから丸ごと復元する。
    // 同値ガードで state→URL 側と往復しない。
    Effect::new(move |_| {
        let mut incoming = query.with(QueryState::from_params);
        incoming.zoom_axis = Some(
            incoming
                .zoom_axis
                .unwrap_or_else(|| client.default_zoom_axis()),
        );
        if state.get_untracked() != incoming {
            state.set(incoming);
        }
    });

    // state → URL: view/絞り込み/ズーム軸の変更のたびにクエリを書き換える。
    Effect::new(move |_| {
        let desired = state.get();
        let current = query.with_untracked(QueryState::from_params);
        // URL→store 経由で来た更新を打ち返さない（クエリが既に同じ状態なら何もしない）。
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
                        class:active=move || view.get() == View::Facet
                        on:click=move |_| state.update(|q| q.view = View::Facet)
                    >
                        "ファセット"
                    </button>
                    <button
                        class="nav-link"
                        class:active=move || view.get() == View::Map
                        on:click=move |_| state.update(|q| q.view = View::Map)
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
        </div>
    }
}
