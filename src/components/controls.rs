use crate::logic::facet::filter_resources;
use crate::map_bridge;
use crate::model::{AppStore, Resource};
use leptos::*;

#[component]
pub fn Controls(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_level: ReadSignal<u32>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let resource_count = create_memo(move |_| {
        let s = store.get();
        let tags = selected_tags.get();
        filter_resources(&s.resources, &tags, &s.dimensions).len()
    });

    let total_count = create_memo(move |_| store.get().resources.len());

    view! {
        <div class="controls-bar">
            <div class="controls-left">
                <span class="controls-hint">"左パネルの ◉ でズーム軸（フォレストの根）を選択"</span>
            </div>
            <div class="controls-right">
                <button
                    class="add-resource-btn"
                    on:click=move |_| editing.set(Some(Resource::default()))
                >
                    "+ 追加"
                </button>
                <span class="level-badge">
                    "Lv." {move || zoom_level.get()}
                </span>
                <span class="resource-count">
                    {move || resource_count.get()}
                    " / "
                    {move || total_count.get()}
                    " 件"
                </span>
                <div class="zoom-buttons">
                    <button
                        class="zoom-btn"
                        title="ズームアウト"
                        on:click=|_| map_bridge::zoom_out()
                    >
                        "−"
                    </button>
                    <button
                        class="zoom-btn"
                        title="ズームイン"
                        on:click=|_| map_bridge::zoom_in()
                    >
                        "+"
                    </button>
                    <button
                        class="zoom-btn zoom-fit"
                        title="全体表示"
                        on:click=|_| map_bridge::zoom_to_fit()
                    >
                        "⊡"
                    </button>
                </div>
            </div>
        </div>
    }
}
