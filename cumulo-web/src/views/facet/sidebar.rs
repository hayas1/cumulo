use crate::category::CategoryId;
use crate::client::Client;
use crate::i18n::*;
use crate::query::{QueryState, View};
use cumulo_model::Forest;
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn FacetSidebar(client: Client, state: RwSignal<QueryState>) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let selected_tags = Memo::new(move |_| state.with(|q| q.filters.clone()));
    let map_mode = state.with_untracked(|q| q.view) == View::Map;
    let collapsed = RwSignal::new(HashSet::<CategoryId>::new());

    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = bipartite.get();
                let tags = selected_tags.get();

                s.taxonomy.roots()
                    .into_iter()
                    .filter_map(|root| {
                        let counts = s.subtree_counts(&root.id, &tags.without_root(&root.id));

                        if counts.is_empty() {
                            return None;
                        }

                        let selected_val = tags.get(&root.id).cloned();

                        let root_count = counts.get(root.id.as_str()).copied().unwrap_or(0);
                        let mut ordered = Vec::new();
                        s.taxonomy.dfs_collect_counts(&root.id, 0, &counts, &mut ordered);

                        let has_children = !s.taxonomy.children_of(&root.id).is_empty();
                        let root_id = root.id.clone();
                        let root_label = if root.label.is_empty() {
                            root.id.to_string()
                        } else {
                            root.label.clone()
                        };

                        let chevron = has_children.then(|| {
                            let rid_toggle = root_id.clone();
                            let rid_icon = root_id.clone();
                            view! {
                                <button
                                    class="facet-panel-chevron"
                                    title=move || t_string!(i18n, facet_collapse)
                                    on:click=move |_| {
                                        collapsed.update(|c| {
                                            if !c.remove(&rid_toggle) {
                                                c.insert(rid_toggle.clone());
                                            }
                                        });
                                    }
                                >
                                    {move || if collapsed.with(|c| c.contains(&rid_icon)) { "▶" } else { "▼" }}
                                </button>
                            }
                        });

                        let axis_btn = if map_mode {
                            let did = root_id.clone();
                            let did_eq = root_id.clone();
                            view! {
                                <button
                                    class="facet-panel-title facet-panel-title-btn"
                                    class:active=move || state.with(|q| q.zoom_axis.as_ref() == Some(&did_eq))
                                    title=move || t_string!(i18n, facet_zoom_axis)
                                    on:click=move |_| state.update(|q| q.zoom_axis = Some(did.clone()))
                                >
                                    <span class="fv-label">{root_label}</span>
                                    <span class="fv-count">{root_count}</span>
                                </button>
                            }
                            .into_any()
                        } else {
                            let rid = root_id.clone();
                            let is_sel = selected_val.as_deref() == Some(root_id.as_str());
                            view! {
                                <button
                                    class=if is_sel {
                                        "facet-panel-title facet-panel-title-btn selected"
                                    } else {
                                        "facet-panel-title facet-panel-title-btn"
                                    }
                                    title=move || t_string!(i18n, facet_filter_axis)
                                    on:click=move |_| {
                                        state.update(|q| q.filters.toggle(rid.clone(), rid.clone()));
                                    }
                                >
                                    <span class="fv-label">{root_label}</span>
                                    <span class="fv-count">{root_count}</span>
                                </button>
                            }
                            .into_any()
                        };

                        let rid_vis = root_id.clone();

                        Some(view! {
                            <div class="facet-panel">
                                <div class="facet-panel-header">
                                    {chevron}
                                    {axis_btn}
                                </div>
                                {move || {
                                    if collapsed.with(|c| c.contains(&rid_vis)) {
                                        return None;
                                    }
                                    Some(
                                        ordered
                                            .iter()
                                            .map(|(node_id, node_label, depth, count)| {
                                                let is_sel =
                                                    selected_val.as_deref() == Some(node_id.as_str());
                                                let indent = format!(
                                                    "padding-left:{}rem",
                                                    0.5 + *depth as f32 * 0.85
                                                );
                                                let rid = root_id.clone();
                                                let nid = node_id.clone();
                                                view! {
                                                    <div class="facet-row" style=indent>
                                                        <button
                                                            class=if is_sel {
                                                                "facet-value selected"
                                                            } else {
                                                                "facet-value"
                                                            }
                                                            on:click=move |_| {
                                                                state.update(|q| q.filters.toggle(rid.clone(), nid.clone()));
                                                            }
                                                        >
                                                            <span class="fv-label">{node_label.clone()}</span>
                                                            <span class="fv-count">{*count}</span>
                                                        </button>
                                                    </div>
                                                }
                                            })
                                            .collect::<Vec<_>>(),
                                    )
                                }}
                            </div>
                        })
                    })
                    .collect::<Vec<_>>()
            }}
        </aside>
    }
}
