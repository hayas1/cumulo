use super::zoom::ZoomController;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::query::QueryState;
use crate::resource::ResourceAttribute;
use cumulo_model::{Resource, Selection};
use leptos::prelude::*;

#[component]
pub fn Controls(
    client: Client,
    state: RwSignal<QueryState>,
    zoom_level: ReadSignal<u32>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    controller: ZoomController,
    fit_action: Callback<()>,
) -> impl IntoView {
    let bipartite = client.read();
    let selected_tags = Memo::new(move |_| state.with(|q| q.filters.clone()));
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
