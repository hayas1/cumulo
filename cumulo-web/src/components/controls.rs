use crate::map_bridge;
use crate::platform::{DimValue, ResourceValue};
use cumulo_model::model::{Bipartite, Resource};
use leptos::*;

#[component]
pub fn Controls(
    bipartite: ReadSignal<Bipartite<ResourceValue, DimValue>>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_level: ReadSignal<u32>,
    editing: RwSignal<Option<Resource<ResourceValue>>>,
) -> impl IntoView {
    let resource_count = create_memo(move |_| {
        let s = bipartite.get();
        let tags = selected_tags.get();
        s.filter_resources(&tags).len()
    });

    let total_count = create_memo(move |_| bipartite.get().resources.len());

    view! {
        <div class="controls-bar">
            <div class="controls-left"></div>
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
