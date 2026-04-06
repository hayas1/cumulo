use crate::logic::facet::filtered_resources;
use crate::model::*;
use leptos::*;
use super::resource_card::ResourceCard;

#[component]
pub fn ResultPanel(
    store: ReadSignal<AppStore>,
    facet_state: RwSignal<FacetState>,
) -> impl IntoView {
    let results = create_memo(move |_| {
        let s = store.get();
        let fs = facet_state.get();
        filtered_resources(&s.resources, &fs.selected, &s.dimensions)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    });

    let selected_chips = create_memo(move |_| facet_state.with(|fs| fs.selected.clone()));

    let is_single = create_memo(move |_| results.with(|r| r.len() == 1));
    let highlighted_signal = Signal::derive(move || is_single.get());

    view! {
        <div class="result-panel">
            <div class="result-panel-header">
                <Show when=move || selected_chips.with(|s| !s.is_empty())>
                    <div class="active-filters">
                        <For
                            each=move || selected_chips.get()
                            key=|(k, v)| format!("{k}={v}")
                            children=move |(dim_id, value)| {
                                let remove_id = dim_id.clone();
                                view! {
                                    <span class="filter-chip">
                                        <span class="chip-key">{dim_id}</span>
                                        <span class="chip-sep">": "</span>
                                        <strong>{value}</strong>
                                        <button
                                            class="filter-chip-remove"
                                            on:click=move |_| {
                                                facet_state.update(|fs| fs.remove(&remove_id))
                                            }
                                        >
                                            "×"
                                        </button>
                                    </span>
                                }
                            }
                        />
                    </div>
                </Show>
                <p class="result-count">
                    <strong>{move || results.with(|r| r.len())}</strong>
                    " 件のリソース"
                </p>
            </div>
            <div class="resource-list">
                <For
                    each=move || results.get()
                    key=|r| r.id.clone()
                    children=move |resource| {
                        view! {
                            <ResourceCard resource=resource highlighted=highlighted_signal />
                        }
                    }
                />
                <Show when=move || results.with(|r| r.is_empty())>
                    <div class="empty-state">
                        <div class="empty-state-icon">"🔍"</div>
                        <p class="empty-state-text">
                            "条件に一致するリソースが見つかりませんでした"
                        </p>
                    </div>
                </Show>
            </div>
        </div>
    }
}
