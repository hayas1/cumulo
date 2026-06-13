use crate::platform::{AttributeValue, EntityValue};
use cumulo_model::model::{Attribute, Bipartite, Id};
use leptos::*;
use std::collections::{HashMap, HashSet};

#[component]
pub fn FacetSidebar(
    bipartite: ReadSignal<Bipartite<EntityValue, AttributeValue>>,
    selected_tags: RwSignal<Vec<(Id<Attribute>, Id<Attribute>)>>,
    /// マップビューでのみ渡す。渡されたときはディメンション軸タイトルをクリックで
    /// ズーム軸に設定できるようにする。
    #[prop(optional)]
    zoom_dim: Option<RwSignal<Id<Attribute>>>,
) -> impl IntoView {
    // 折りたたまれているパネルの根id を管理（ノード単位ではなくパネル単位）
    let collapsed = create_rw_signal(HashSet::<Id<Attribute>>::new());

    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = bipartite.get();
                let tags = selected_tags.get();

                s.attributes.roots()
                    .into_iter()
                    .filter_map(|root| {
                        let tags_minus: Vec<_> = tags
                            .iter()
                            .filter(|(k, _)| k != &root.id)
                            .cloned()
                            .collect();
                        let base = s.filter_entities(&tags_minus);

                        let mut counts: HashMap<Id<Attribute>, usize> = HashMap::new();
                        for r in &base {
                            if let Some(leaf_id) = r.attribute(&root.id) {
                                *counts.entry(leaf_id.clone()).or_default() += 1;
                                for anc in s.attributes.ancestry(leaf_id) {
                                    if &anc != leaf_id {
                                        *counts.entry(anc).or_default() += 1;
                                    }
                                }
                            }
                        }

                        if counts.is_empty() {
                            return None;
                        }

                        let selected_val = tags
                            .iter()
                            .find(|(k, _)| k == &root.id)
                            .map(|(_, v)| v.clone());

                        let mut ordered: Vec<(Id<Attribute>, String, usize, usize)> = Vec::new();
                        s.attributes.dfs_collect_counts(&root.id, 0, &counts, &mut ordered);

                        if ordered.is_empty() {
                            return None;
                        }

                        let root_id = root.id.clone();
                        let root_label = root.label.clone();

                        let rid_toggle = root_id.clone();
                        let rid_icon = root_id.clone();
                        let chevron = view! {
                            <button
                                class="facet-panel-chevron"
                                title="折りたたむ"
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
                        };

                        let title = match zoom_dim {
                            Some(zd) => {
                                let did = root_id.clone();
                                let did_eq = root_id.clone();
                                view! {
                                    <button
                                        class="facet-panel-title facet-panel-title-btn"
                                        class:active=move || zd.get() == did_eq
                                        title="ズーム軸にする"
                                        on:click=move |_| zd.set(did.clone())
                                    >
                                        {root_label}
                                    </button>
                                }
                                .into_view()
                            }
                            None => view! {
                                <div class="facet-panel-title">{root_label}</div>
                            }
                            .into_view(),
                        };

                        let rid_vis = root_id.clone();

                        Some(view! {
                            <div class="facet-panel">
                                <div class="facet-panel-header">
                                    {chevron}
                                    {title}
                                </div>
                                {move || {
                                    if collapsed.with(|c| c.contains(&rid_vis)) {
                                        return None;
                                    }
                                    Some(
                                        ordered
                                            .iter()
                                            .map(|(node_id, node_label, depth, count)| {
                                                let is_sel = selected_val.as_deref()
                                                    == Some(node_id.as_str());
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
                                                                let k = rid.clone();
                                                                let v = nid.clone();
                                                                selected_tags.update(|t| {
                                                                    let already = t.iter().any(|(tk, tv)| tk == &k && tv == &v);
                                                                    t.retain(|(tk, _)| tk != &k);
                                                                    if !already {
                                                                        t.push((k, v));
                                                                    }
                                                                });
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
