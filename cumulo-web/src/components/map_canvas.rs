use crate::map_bridge;
use crate::platform::{AttributeValue, EntityValue};
use cumulo_model::model::Bipartite;
use leptos::*;

#[component]
pub fn MapCanvas(
    bipartite: ReadSignal<Bipartite<EntityValue, AttributeValue>>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_dim: RwSignal<String>,
    selected_entity: RwSignal<Option<String>>,
    zoom_level: RwSignal<u32>,
) -> impl IntoView {
    // ── Effect 1: D3初期化（一度だけ。シグナル依存なし）──────────────────────
    create_effect(move |_| {
        map_bridge::init_map("main-svg");

        map_bridge::on_entity_select(move |id| {
            selected_entity.set(Some(id));
        });

        map_bridge::on_zoom_level_change(move |level| {
            zoom_level.set(level);
        });

        // クラスタへのズームイン → そのディメンション値を絞り込み軸へ反映（置換）
        map_bridge::on_cluster_drill(move |axis, value| {
            selected_tags.update(|t| {
                t.retain(|(k, _)| k != &axis);
                t.push((axis, value));
            });
        });

        // 全体表示へのズームアウト → 現在のズーム軸の絞り込みだけ解除
        map_bridge::on_zoom_reset(move || {
            let zd = zoom_dim.get_untracked();
            selected_tags.update(|t| t.retain(|(k, _)| k != &zd));
        });
    });

    // ── Effect 2: リソースデータ更新 ─────────────────────────────────────────
    create_effect(move |_| {
        let resources = &bipartite.get().entities;
        if let Ok(json) = serde_json::to_string(resources) {
            map_bridge::update_entities(&json);
        }
    });

    // ── Effect 3b: ディメンション（カラー定義含む）更新 ──────────────────────
    create_effect(move |_| {
        let dimensions = bipartite.get().attributes;
        if let Ok(json) = serde_json::to_string(&dimensions) {
            map_bridge::update_attributes(&json);
        }
    });

    // ── Effect 4: フィルター更新 ──────────────────────────────────────────────
    create_effect(move |_| {
        let tags = selected_tags.get();
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
