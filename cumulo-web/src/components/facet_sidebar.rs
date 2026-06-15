use crate::platform::{CategoryAttribute, CategoryId, ResourceAttribute};
use cumulo_model::{Bipartite, Forest};
use leptos::*;
use std::collections::{HashMap, HashSet};

#[component]
pub fn FacetSidebar(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Vec<(CategoryId, CategoryId)>>,
    /// マップビューでのみ渡す。渡されたときはディメンション軸タイトルをクリックで
    /// ズーム軸に設定できるようにする。
    #[prop(optional)]
    zoom_dim: Option<RwSignal<CategoryId>>,
) -> impl IntoView {
    // 折りたたまれているパネルの根id を管理（ノード単位ではなくパネル単位）
    let collapsed = create_rw_signal(HashSet::<CategoryId>::new());

    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = bipartite.get();
                let tags = selected_tags.get();

                s.taxonomy.roots()
                    .into_iter()
                    .filter_map(|root| {
                        let tags_minus: Vec<_> = tags
                            .iter()
                            .filter(|(k, _)| k != &root.id)
                            .cloned()
                            .collect();
                        let base = s.filter_resources(&tags_minus);

                        let mut counts: HashMap<CategoryId, usize> = HashMap::new();
                        for r in &base {
                            if let Some(leaf_id) = r.category(&s.taxonomy, &root.id) {
                                *counts.entry(leaf_id.clone()).or_default() += 1;
                                for anc in s.taxonomy.ancestry(leaf_id) {
                                    if &anc != leaf_id {
                                        *counts.entry(anc).or_default() += 1;
                                    }
                                }
                            }
                        }

                        if counts.is_empty() {
                            return None;
                        }

                        // 1軸1フィルタなので、その軸で選択中の値は高々1つ
                        let selected_val = tags
                            .iter()
                            .find(|(k, _)| k == &root.id)
                            .map(|(_, v)| v.clone());

                        // 軸（根）はヘッダの見出し兼フィルタ要素に集約する。配下の値だけを行に並べる。
                        let root_count = counts.get(root.id.as_str()).copied().unwrap_or(0);
                        let mut ordered: Vec<(CategoryId, String, usize, usize)> = Vec::new();
                        s.taxonomy.dfs_collect_counts(&root.id, 0, &counts, &mut ordered);

                        // collapse は折りたためる子がある軸でのみ可能にする
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
                            }
                        });

                        // 軸の見出し＝根。マップではクリックでズーム軸、ファセットでは
                        // 根フィルタ（その軸の部分木全体にマッチ）。見出しと根を1要素に統合する。
                        let axis_btn = match zoom_dim {
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
                                        <span class="fv-label">{root_label}</span>
                                        <span class="fv-count">{root_count}</span>
                                    </button>
                                }
                                .into_view()
                            }
                            None => {
                                let rid = root_id.clone();
                                let is_sel = selected_val.as_deref() == Some(root_id.as_str());
                                view! {
                                    <button
                                        class=if is_sel {
                                            "facet-panel-title facet-panel-title-btn selected"
                                        } else {
                                            "facet-panel-title facet-panel-title-btn"
                                        }
                                        title="この軸全体で絞り込む"
                                        on:click=move |_| {
                                            // 1軸1フィルタ: 同軸の既存値を外して根に入れ替える（同値なら解除）
                                            selected_tags.update(|t| {
                                                let already = t.iter().any(|(tk, tv)| tk == &rid && tv == &rid);
                                                t.retain(|(tk, _)| tk != &rid);
                                                if !already {
                                                    t.push((rid.clone(), rid.clone()));
                                                }
                                            });
                                        }
                                    >
                                        <span class="fv-label">{root_label}</span>
                                        <span class="fv-count">{root_count}</span>
                                    </button>
                                }
                                .into_view()
                            }
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
                                                                let k = rid.clone();
                                                                let v = nid.clone();
                                                                // 1軸1フィルタ: 同軸の既存値を外して入れ替える（同値なら解除）
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
