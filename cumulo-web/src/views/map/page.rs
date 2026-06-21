//! マップ画面（ルート `/map`）。ズーム状態を生成し、Controls / Canvas / Sidebar / DetailPanel を束ねる枠。

use super::canvas::MapCanvas;
use super::controls::Controls;
use super::zoom::ZoomController;
use crate::platform::{CategoryAttribute, Filters, Platform, ResourceAttribute, ResourceId};
use crate::resource::detail_panel::DetailPanel;
use crate::views::facet::sidebar::FacetSidebar;
use cumulo_model::{Bipartite, Forest, Resource};
use leptos::prelude::*;

#[component]
pub fn MapView(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let selected_entity_id = RwSignal::new(Option::<ResourceId>::None);
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
                bipartite=bipartite
                selected_tags=selected_tags
                zoom_level=zoom_level.read_only()
                editing=editing
                controller=controller
                fit_action=fit_action
            />
            <div class="map-area">
                <FacetSidebar bipartite=bipartite selected_tags=selected_tags zoom_dim=zoom_dim />
                <MapCanvas
                    bipartite=bipartite
                    selected_tags=selected_tags
                    zoom_dim=zoom_dim
                    selected_entity=selected_entity_id
                    zoom_level=zoom_level
                    controller=controller
                    fit_action=fit_action
                />
                <DetailPanel bipartite=bipartite selected_id=selected_entity_id editing=editing />
            </div>
        </div>
    }
}
