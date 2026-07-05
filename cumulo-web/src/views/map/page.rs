//! マップ画面（ルート `/map`）。ズーム状態を生成し、Controls / Canvas / Sidebar / DetailPanel を束ねる枠。

use super::canvas::MapCanvas;
use super::controls::Controls;
use super::zoom::ZoomController;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::query::QueryState;
use crate::resource::detail_panel::DetailPanel;
use crate::resource::{ResourceAttribute, ResourceId};
use crate::views::facet::sidebar::FacetSidebar;
use cumulo_model::Resource;
use leptos::prelude::*;

#[component]
pub fn MapView(
    client: Client,
    state: RwSignal<QueryState>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let selected_resource_id = RwSignal::new(Option::<ResourceId>::None);
    let zoom_level = RwSignal::new(0u32);

    // ズーム状態は Controls（ボタン）と MapCanvas（描画・操作）で共有する。
    let controller = ZoomController::new();

    // 全体表示は「ズーム軸の絞り込み解除」と「ズームレベル 0」を伴う。
    // 「全体表示」ボタンと背景クリックの両方から呼べるよう Callback にまとめる。
    let fit_action = Callback::new(move |()| {
        controller.zoom_to_fit();
        zoom_level.set(0);
        let zd = state
            .with_untracked(|q| q.zoom_axis.clone())
            .unwrap_or_else(|| client.default_zoom_axis());
        state.update(|q| q.filters.remove_root(&zd));
    });

    view! {
        <div class="map-view">
            <Controls
                client=client
                state=state
                zoom_level=zoom_level.read_only()
                editing=editing
                controller=controller
                fit_action=fit_action
            />
            <div class="map-area">
                <FacetSidebar client=client state=state />
                <MapCanvas
                    client=client
                    state=state
                    selected_resource=selected_resource_id
                    zoom_level=zoom_level
                    controller=controller
                    fit_action=fit_action
                />
                <DetailPanel client=client selected_id=selected_resource_id editing=editing />
            </div>
        </div>
    }
}
