//! マップ画面（ルート `/map`）。ズーム状態を生成し、Controls / Canvas / Sidebar / DetailPanel を束ねる枠。

use super::canvas::MapCanvas;
use super::controls::Controls;
use super::zoom::ZoomController;
use crate::category::{CategoryAttribute, Filters};
use crate::client::Client;
use crate::platform::Platform;
use crate::resource::detail_panel::DetailPanel;
use crate::resource::{ResourceAttribute, ResourceId};
use crate::views::facet::sidebar::FacetSidebar;
use cumulo_model::{Forest, Resource};
use leptos::prelude::*;

#[component]
pub fn MapView(
    client: Client,
    selected_tags: RwSignal<Filters>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let bipartite = client.read();
    let selected_resource_id = RwSignal::new(Option::<ResourceId>::None);
    let zoom_level = RwSignal::new(0u32);
    // ズーム軸＝軸（根カテゴリ）。既定は最初の根。セレクタの候補も根なので既定も根に揃える。
    // taxonomy が空の場合は表示対象がないため、使われないダミー id を割り当てる
    let zoom_dim = RwSignal::new({
        let s = bipartite.get_untracked();
        s.taxonomy
            .roots()
            .first()
            .map(|d| d.id.clone())
            .unwrap_or_else(Platform::new_node_id)
    });

    // ズーム状態は Controls（ボタン）と MapCanvas（描画・操作）で共有する。
    let controller = ZoomController::new();

    // 全体表示は「ズーム軸の絞り込み解除」と「ズームレベル 0」を伴う。
    // 「全体表示」ボタンと背景クリックの両方から呼べるよう Callback にまとめる。
    let fit_action = Callback::new(move |()| {
        controller.zoom_to_fit();
        zoom_level.set(0);
        let zd = zoom_dim.get_untracked();
        selected_tags.update(|t| t.remove_root(&zd));
    });

    view! {
        <div class="map-view">
            <Controls
                client=client
                selected_tags=selected_tags
                zoom_level=zoom_level.read_only()
                editing=editing
                controller=controller
                fit_action=fit_action
            />
            <div class="map-area">
                <FacetSidebar client=client selected_tags=selected_tags zoom_dim=zoom_dim />
                <MapCanvas
                    client=client
                    selected_tags=selected_tags
                    zoom_dim=zoom_dim
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
