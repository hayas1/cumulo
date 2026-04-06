use crate::logic::facet::all_facets_with_values;
use crate::model::*;
use leptos::*;

#[component]
pub fn FacetPanel(
    store: ReadSignal<AppStore>,
    facet_state: RwSignal<FacetState>,
) -> impl IntoView {
    let facets = create_memo(move |_| {
        let s = store.get();
        let fs = facet_state.get();
        all_facets_with_values(&s.resources, &fs.selected, &s.dimensions)
    });

    let has_selection = create_memo(move |_| facet_state.with(|fs| !fs.selected.is_empty()));

    view! {
        <div class="facet-panel">
            <div class="facet-panel-header">
                <span class="facet-panel-title">"絞り込み"</span>
                <Show when=move || has_selection.get()>
                    <button
                        class="reset-btn"
                        on:click=move |_| facet_state.update(|fs| fs.clear())
                    >
                        "リセット"
                    </button>
                </Show>
            </div>
            <div class="facet-panel-body">
                <For
                    each=move || facets.get()
                    key=|(dim, _)| dim.id.clone()
                    children=move |(dim, values)| {
                        let dim_id = dim.id.clone();
                        view! {
                            <DimensionGroup
                                dim=dim
                                values=values
                                dim_id=dim_id
                                facet_state=facet_state
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}

#[component]
fn DimensionGroup(
    dim: Dimension,
    values: Vec<String>,
    dim_id: String,
    facet_state: RwSignal<FacetState>,
) -> impl IntoView {
    view! {
        <div class="dimension-group">
            <div class="dimension-label">{dim.label}</div>
            <div class="facet-values">
                {values.into_iter().map(move |val| {
                    let check_dim = dim_id.clone();
                    let check_val = val.clone();
                    let click_dim = dim_id.clone();
                    let click_val = val.clone();

                    view! {
                        <button
                            class="facet-btn"
                            class=("selected", move || {
                                facet_state.with(|fs| {
                                    fs.get_selected(&check_dim) == Some(check_val.as_str())
                                })
                            })
                            on:click=move |_| {
                                facet_state.update(|fs| {
                                    fs.toggle(click_dim.clone(), click_val.clone())
                                })
                            }
                        >
                            {val}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
