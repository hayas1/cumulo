use crate::category::{CategoryAttribute, Filters};
use crate::client::Client;
use crate::platform::Platform;
use crate::query::QueryState;
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
use crate::views::{facet::FacetView, map::MapView};
use cumulo_model::Resource;

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::{path, NavigateOptions};

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
    let client = Client::load();
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

    // 絞り込み状態を URL クエリ（軸ごとに f.<軸>=<値>）と双方向同期する。同じ bipartite を
    // 持つ相手に URL を共有すれば、同じ絞り込み画面が開ける。両方向に同値ガードを置いて
    // 往復ループを断つ。ParamsMap との変換と名前空間管理は QueryState に集約する。
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();

    // URL → state: 共有リンクを開いた時・ブラウザの戻る/進み時にクエリから復元する。
    Effect::new(move |_| {
        let restored = query.with(|p| QueryState::from_params(p).filters);
        if selected_tags.get_untracked() != restored {
            selected_tags.set(restored);
        }
    });

    // state → URL: 絞り込み操作のたびにクエリを書き換える。
    Effect::new(move |_| {
        let current = query.with_untracked(QueryState::from_params);
        // current を複製して filters だけ差し替える。rest（外部キー）や将来フィールドは
        // そのまま引き継がれるので、to_params が全状態を書き出しても他要素を消さない。
        let mut desired = current.clone();
        desired.filters = selected_tags.get();
        // URL→state 経由で来た更新を打ち返さない（クエリが既に同じ状態なら何もしない）。
        if current == desired {
            return;
        }
        // 現在のパスを保ったままクエリだけ差し替える。戻る/進むを汚さぬよう履歴は積まず replace。
        let url = format!(
            "{}{}",
            location.pathname.get_untracked(),
            desired.to_params().to_query_string()
        );
        navigate(
            &url,
            NavigateOptions {
                resolve: false,
                replace: true,
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

            <Palette client=client selected_tags=selected_tags />

            <div class="route-content">
                <Routes fallback=|| view! { <div class="route-404">"ページが見つかりません"</div> }>
                    <Route path=path!("/") view=move || view! {
                        <FacetView client=client selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path=path!("/facet") view=move || view! {
                        <FacetView client=client selected_tags=selected_tags editing=editing />
                    }/>
                    <Route path=path!("/map") view=move || view! {
                        <MapView client=client selected_tags=selected_tags editing=editing />
                    }/>
                </Routes>
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
