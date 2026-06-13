use crate::map_bridge;
use crate::platform::{AttributeValue, EntityValue};
use cumulo_model::model::{Bipartite, Entity};
use leptos::*;

#[component]
pub fn Controls(
    bipartite: ReadSignal<Bipartite<EntityValue, AttributeValue>>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_level: ReadSignal<u32>,
    editing: RwSignal<Option<Entity<EntityValue>>>,
) -> impl IntoView {
    let entity_count = create_memo(move |_| {
        let s = bipartite.get();
        let tags = selected_tags.get();
        s.filter_entities(&tags).len()
    });

    let total_count = create_memo(move |_| bipartite.get().entities.len());

    view! {
        <div class="controls-bar">
            <div class="controls-left"></div>
            <div class="controls-right">
                <button
                    class="add-resource-btn"
                    on:click=move |_| editing.set(Some(Entity::default()))
                >
                    "+ 追加"
                </button>
                <span class="level-badge">
                    "Lv." {move || zoom_level.get()}
                </span>
                <span class="resource-count">
                    {move || entity_count.get()}
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
