use crate::map::zoom::ZoomController;
use crate::platform::{CategoryAttribute, Filters, Platform, ResourceAttribute};
use cumulo_model::{Bipartite, Resource};
use leptos::prelude::*;

#[component]
pub fn Controls(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    zoom_level: ReadSignal<u32>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    controller: ZoomController,
    /// 全体表示（フィルタ解除込み）。MapCanvas の背景クリックと共有する。
    fit_action: Callback<()>,
) -> impl IntoView {
    let entity_count = Memo::new(move |_| {
        let s = bipartite.get();
        let tags = selected_tags.get();
        s.filter_resources(&tags).len()
    });

    let total_count = Memo::new(move |_| bipartite.get().catalog.len());

    view! {
        <div class="controls-bar">
            <div class="controls-left"></div>
            <div class="controls-right">
                <button
                    class="add-resource-btn"
                    on:click=move |_| editing.set(Some(Platform::new_resource()))
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
                        on:click=move |_| controller.zoom_out()
                    >
                        "−"
                    </button>
                    <button
                        class="zoom-btn"
                        title="ズームイン"
                        on:click=move |_| controller.zoom_in()
                    >
                        "+"
                    </button>
                    <button
                        class="zoom-btn zoom-fit"
                        title="全体表示"
                        on:click=move |_| fit_action.run(())
                    >
                        "⊡"
                    </button>
                </div>
            </div>
        </div>
    }
}
