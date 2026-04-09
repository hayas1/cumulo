use crate::model::AppStore;
use crate::storage::load_from_storage;
use leptos::*;
use super::{
    controls::Controls, detail_panel::DetailPanel, map_canvas::MapCanvas, palette::Palette,
};

#[component]
pub fn App() -> impl IntoView {
    let (store, _set_store) = create_signal::<AppStore>(load_from_storage());

    // パレットで選択中のタグ (attr_key, value) ペアのリスト
    let selected_tags = create_rw_signal(Vec::<(String, String)>::new());

    // D3で選択されたリソースのID
    let selected_resource_id = create_rw_signal(Option::<String>::None);

    // 現在のズームレベル（D3から通知）
    let zoom_level = create_rw_signal(0u32);

    // ズーム軸（1軸から始め、Controls の＋ボタンで追加）
    let zoom_axes = create_rw_signal({
        let cfg = store.get_untracked();
        vec![cfg.map_config.zoom_axes[0].clone()]
    });

    view! {
        <div class="app">
            <header class="app-header">
                <span class="app-logo">"☁ Cumulo"</span>
                <span class="app-tagline">"マルチクラウド リソースマップ"</span>
            </header>

            <Palette store=store selected_tags=selected_tags />

            <Controls
                store=store
                selected_tags=selected_tags
                zoom_axes=zoom_axes
                zoom_level=zoom_level.read_only()
            />

            <div class="map-area">
                <MapCanvas
                    store=store
                    selected_tags=selected_tags
                    zoom_axes=zoom_axes
                    selected_resource=selected_resource_id
                    zoom_level=zoom_level
                />
                <DetailPanel store=store selected_id=selected_resource_id />
            </div>
        </div>
    }
}
