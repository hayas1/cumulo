use crate::map_bridge;
use crate::platform::{CategoryAttribute, CategoryId, Filters, ResourceAttribute, ResourceId};
use cumulo_model::Bipartite;
use leptos::*;

#[component]
pub fn MapCanvas(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    zoom_dim: RwSignal<CategoryId>,
    selected_entity: RwSignal<Option<ResourceId>>,
    zoom_level: RwSignal<u32>,
) -> impl IntoView {
    // ── Effect 1: D3初期化（一度だけ。シグナル依存なし）──────────────────────
    create_effect(move |_| {
        map_bridge::init_map("main-svg");

        // JS からの id は空文字列になり得るため、空なら無視する
        map_bridge::on_entity_select(move |id| {
            if let Ok(v) = id.try_into() {
                selected_entity.set(Some(v));
            }
        });

        map_bridge::on_zoom_level_change(move |level| {
            zoom_level.set(level);
        });

        // クラスタへのズームイン → そのディメンション値を絞り込み軸へ反映（置換）
        // JS からの axis/value は空文字列になり得るため、どちらかが空なら無視する
        map_bridge::on_cluster_drill(move |axis, value| {
            if let (Ok(k), Ok(v)) = (CategoryId::try_from(axis), CategoryId::try_from(value)) {
                selected_tags.update(|t| t.set(k, v));
            }
        });

        // 全体表示へのズームアウト → 現在のズーム軸の絞り込みだけ解除
        map_bridge::on_zoom_reset(move || {
            let zd = zoom_dim.get_untracked();
            selected_tags.update(|t| t.remove_root(&zd));
        });
    });

    // ── Effect 2: リソースデータ更新 ─────────────────────────────────────────
    create_effect(move |_| {
        let resources = &bipartite.get().catalog;
        if let Ok(json) = serde_json::to_string(resources) {
            map_bridge::update_entities(&json);
        }
    });

    // ── Effect 3b: ディメンション（カラー定義含む）更新 ──────────────────────
    create_effect(move |_| {
        let dimensions = bipartite.get().taxonomy;
        if let Ok(json) = serde_json::to_string(&dimensions) {
            map_bridge::update_attributes(&json);
        }
    });

    // ── Effect 4: フィルター更新 ──────────────────────────────────────────────
    create_effect(move |_| {
        // map.js は [[axis, value], ...] の配列を期待するため、Filters を組に変換して渡す
        let tags: Vec<(CategoryId, CategoryId)> =
            selected_tags.with(|f| f.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        if let Ok(json) = serde_json::to_string(&tags) {
            map_bridge::update_filter(&json);
        }
    });

    // ── Effect 5: ズーム軸（ディメンション）更新 ──────────────────────────────
    create_effect(move |_| {
        let dim = zoom_dim.get();
        let payload = serde_json::json!({ "dim": dim });
        map_bridge::update_zoom_dim(&payload.to_string());
    });

    view! {
        <div id="map-container">
            <svg id="main-svg" />
        </div>
    }
}
