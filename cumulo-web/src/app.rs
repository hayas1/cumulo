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

    // URL に載る UI 状態（view / 絞り込み / ズーム軸）を 1 つの signal(QueryState) に集約し、
    // URL クエリと双方向に束ねて prop-drill する。初期値はクエリから同期に seed（ちらつき防止）、
    // 以後は下の 2 Effect で URL⇄signal を対称に張る。中身のロジックは QueryState 側に置く。
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();
    let state = RwSignal::new(query.with_untracked(|p| QueryState::resolved_from(p, &client)));
    // view だけを購読する Memo。これが無いと下の match / nav が state 全体を購読し、
    // 絞り込み変更のたびに FacetView/MapView を作り直して（再マウント）しまう。
    let view = Memo::new(move |_| state.with(|q| q.view));

    // URL → signal（共有リンク復元・戻る/進む）
    Effect::new(move |_| QueryState::load_from_url(state, query, &client));
    // signal → URL（UI 操作の反映）
    Effect::new(move |_| QueryState::store_to_url(state, query, location.pathname, &navigate));

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
