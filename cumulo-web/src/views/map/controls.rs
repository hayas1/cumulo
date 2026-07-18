use super::zoom::ZoomController;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::i18n::*;
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
    let i18n = use_i18n();
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
                    {t!(i18n, map_add)}
                </button>
                <span class="level-badge">
                    "Lv." {move || zoom_level.get()}
                </span>
                <span class="resource-count">
                    {move || t_string!(i18n, map_count, current = resource_count.get(), total = total_count.get())}
                </span>
                <div class="zoom-buttons">
                    <button
                        class="zoom-btn"
                        title=move || t_string!(i18n, map_zoom_out)
                        on:click=move |_| controller.zoom_out()
                    >
                        "−"
                    </button>
                    <button
                        class="zoom-btn"
                        title=move || t_string!(i18n, map_zoom_in)
                        on:click=move |_| controller.zoom_in()
                    >
                        "+"
                    </button>
                    <button
                        class="zoom-btn zoom-fit"
                        title=move || t_string!(i18n, map_fit)
                        on:click=move |_| fit_action.run(())
                    >
                        "⊡"
                    </button>
                </div>
            </div>
        </div>
    }
}
