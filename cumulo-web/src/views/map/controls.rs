use super::zoom::ZoomController;
use crate::category::{CategoryAttribute, Filters};
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use cumulo_model::{Bipartite, Resource, Selection};
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
    let resource_count = Memo::new(move |_| {
        let s = bipartite.get();
        let tags = selected_tags.get();
        s.filtered(&tags).len()
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
                    {move || resource_count.get()}
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
