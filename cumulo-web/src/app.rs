use crate::category::{CategoryAttribute, CategoryId, Filters};
use crate::client::Client;
use crate::platform::Platform;
use crate::query::QueryState;
use crate::resource::form::ResourceForm;
use crate::resource::ResourceAttribute;
use crate::shared::{palette::Palette, settings_modal::SettingsModal};
use crate::views::{facet::FacetView, map::MapView};
use cumulo_model::{Forest, Resource};

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
    // マップのズーム軸。URL 共有のため App が保持する（URL ⇄ 状態同期を1箇所に集約）。
    // 既定は taxonomy の先頭根。空なら使われないダミー id（page 旧実装と同じ既定）。
    let default_zoom_axis: CategoryId = client.read().with_untracked(|s| {
        s.taxonomy
            .roots()
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_else(Platform::new_node_id)
    });
    let zoom_axis = RwSignal::new(default_zoom_axis.clone());
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

    // 絞り込み・ズーム軸を URL クエリと双方向同期する。同じ bipartite を持つ相手に URL を
    // 共有すれば同じ画面（フィルタ＋マップのズーム）が開ける。両方向に同値ガードを置いて
    // 往復ループを断つ。ParamsMap との変換と名前空間管理は QueryState に集約する。
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();

    // URL → state: 共有リンクを開いた時・ブラウザの戻る/進み時にクエリから復元する。
    let default_in = default_zoom_axis.clone();
    Effect::new(move |_| {
        let qs = query.with(QueryState::from_params);
        if selected_tags.get_untracked() != qs.filters {
            selected_tags.set(qs.filters);
        }
        // None（クエリに無い）は既定軸を表す。
        let axis = qs.zoom_axis.unwrap_or_else(|| default_in.clone());
        if zoom_axis.get_untracked() != axis {
            zoom_axis.set(axis);
        }
    });

    // state → URL: 絞り込み/ズーム軸の変更のたびにクエリを書き換える。
    let default_out = default_zoom_axis;
    Effect::new(move |_| {
        let current = query.with_untracked(QueryState::from_params);
        // current を複製して自分の担当フィールドだけ差し替える。rest（外部キー）など他フィールドは
        // そのまま引き継がれるので、to_params が全状態を書き出しても他要素を消さない。
        let mut desired = current.clone();
        desired.filters = selected_tags.get();
        // 既定軸は URL を汚さないよう省略（None）。
        let axis = zoom_axis.get();
        desired.zoom_axis = (axis != default_out).then_some(axis);
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
                        <MapView client=client selected_tags=selected_tags zoom_axis=zoom_axis editing=editing />
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
