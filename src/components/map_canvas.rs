use crate::map_bridge;
use crate::model::AppStore;
use leptos::*;

#[component]
pub fn MapCanvas(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_root: RwSignal<(String, String)>,
    selected_resource: RwSignal<Option<String>>,
    zoom_level: RwSignal<u32>,
) -> impl IntoView {
    // ── Effect 1: D3初期化（一度だけ。シグナル依存なし）──────────────────────
    create_effect(move |_| {
        map_bridge::init_map("main-svg");

        map_bridge::on_resource_select(move |id| {
            selected_resource.set(Some(id));
        });

        map_bridge::on_zoom_level_change(move |level| {
            zoom_level.set(level);
        });
    });

    // ── Effect 2: リソースデータ更新 ─────────────────────────────────────────
    create_effect(move |_| {
        let store_data = store.get();
        let resources = &store_data.resources;
        // D3へ送る前に effective_attrs を展開（子リソースが親の attrs を継承）
        let resources_for_d3: Vec<_> = resources
            .iter()
            .map(|r| {
                let mut r2 = r.clone();
                r2.attrs = r.effective_attrs(resources);
                r2
            })
            .collect();
        if let Ok(json) = serde_json::to_string(&resources_for_d3) {
            map_bridge::update_resources(&json);
        }
    });

    // ── Effect 3b: ディメンション（カラー定義含む）更新 ──────────────────────
    create_effect(move |_| {
        let dimensions = store.get().dimensions;
        if let Ok(json) = serde_json::to_string(&dimensions) {
            map_bridge::update_dimensions(&json);
        }
    });

    // ── Effect 4: フィルター更新 ──────────────────────────────────────────────
    create_effect(move |_| {
        let tags = selected_tags.get();
        if let Ok(json) = serde_json::to_string(&tags) {
            map_bridge::update_filter(&json);
        }
    });

    // ── Effect 5: ズーム軸（フォレストの根）更新 ──────────────────────────────
    create_effect(move |_| {
        let (dim, root) = zoom_root.get();
        let payload = serde_json::json!({ "dim": dim, "root": root });
        map_bridge::update_zoom_root(&payload.to_string());
    });

    view! {
        <div id="map-container">
            <svg id="main-svg" />
        </div>
    }
}
