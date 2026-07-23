use crate::category::{CategoryId, Filters};
use crate::client::Client;
use crate::i18n::*;
use crate::query::QueryState;
use cumulo_model::Forest;
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn AxisFacet(
    client: Client,
    state: RwSignal<QueryState>,
    selected: CategoryId,
    is_row: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let collapsed = RwSignal::new(HashSet::<CategoryId>::new());

    let set_axis = move |id: CategoryId| {
        state.update(move |q| {
            if is_row {
                q.row_axis = Some(id);
            } else {
                q.col_axis = Some(id);
            }
        });
    };

    view! {
        <aside class="matrix-axis-facet">
            <div class="matrix-axis-facet-title">
                {if is_row {
                    t!(i18n, matrix_rows).into_any()
                } else {
                    t!(i18n, matrix_cols).into_any()
                }}
            </div>
            <div class="facet-sidebar">
                {move || {
                    let s = bipartite.get();
                    s.taxonomy
                        .roots()
                        .into_iter()
                        .filter(|root| !s.taxonomy.children_of(&root.id).is_empty())
                        .map(|root| {
                            let counts = s.subtree_counts(&root.id, &Filters::new());
                            let root_count = counts.get(root.id.as_str()).copied().unwrap_or(0);
                            let mut ordered = Vec::new();
                            s.taxonomy.dfs_collect_counts(&root.id, 0, &counts, &mut ordered);
                            let root_id = root.id.clone();
                            let root_label = s.taxonomy.label_of(&root.id);

                            let rid_toggle = root_id.clone();
                            let rid_icon = root_id.clone();
                            let chevron = view! {
                                <button
                                    class="facet-panel-chevron"
                                    on:click=move |_| {
                                        collapsed.update(|c| {
                                            if !c.remove(&rid_toggle) {
                                                c.insert(rid_toggle.clone());
                                            }
                                        });
                                    }
                                >
                                    {move || {
                                        if collapsed.with(|c| c.contains(&rid_icon)) {
                                            "\u{25b6}"
                                        } else {
                                            "\u{25bc}"
                                        }
                                    }}
                                </button>
                            };

                            let tid = root_id.clone();
                            let title_selected = root_id == selected;
                            let title_btn = view! {
                                <button
                                    class="facet-panel-title facet-panel-title-btn"
                                    class:selected=title_selected
                                    on:click=move |_| set_axis(tid.clone())
                                >
                                    <span class="fv-label">{root_label}</span>
                                    <span class="fv-count">{root_count}</span>
                                </button>
                            };

                            let rid_vis = root_id.clone();
                            let selected_v = selected.clone();
                            view! {
                                <div class="facet-panel">
                                    <div class="facet-panel-header">{chevron}{title_btn}</div>
                                    {move || {
                                        if collapsed.with(|c| c.contains(&rid_vis)) {
                                            return None;
                                        }
                                        Some(
                                            ordered
                                                .iter()
                                                .map(|(node_id, node_label, depth, count, has_children)| {
                                                    let indent = format!(
                                                        "padding-left:{}rem",
                                                        0.5 + *depth as f32 * 0.85,
                                                    );
                                                    let nid = node_id.clone();
                                                    let is_sel = node_id == &selected_v;
                                                    view! {
                                                        <div class="facet-row" style=indent>
                                                            <button
                                                                class="facet-value"
                                                                class:selected=is_sel
                                                                class:disabled=!*has_children
                                                                disabled=!*has_children
                                                                on:click=move |_| set_axis(nid.clone())
                                                            >
                                                                <span class="fv-label">
                                                                    {node_label.clone()}
                                                                </span>
                                                                <span class="fv-count">{*count}</span>
                                                            </button>
                                                        </div>
                                                    }
                                                })
                                                .collect::<Vec<_>>(),
                                        )
                                    }}
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>
        </aside>
    }
}
