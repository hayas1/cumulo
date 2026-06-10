use crate::logic::facet::{filter_resources, resolve_dimension};
use crate::model::{ancestry, children_of, roots, AppStore, DimensionNode};
use leptos::*;
use std::collections::{HashMap, HashSet};

/// 折りたたみキー（ノードidをそのまま使う）
fn collapse_key(node_id: &str) -> String {
    node_id.to_string()
}

/// 軸の根の直下から DFS し、(node_id, node_label, depth, count, has_children, is_collapsed) を出力。
/// count == 0 のノードはスキップ。折りたたまれたノードの子孫は出力しない。
fn dfs_collect(
    all_nodes: &[DimensionNode],
    parent_id: &str,
    depth: usize,
    counts: &HashMap<String, usize>,
    collapsed: &HashSet<String>,
    out: &mut Vec<(String, String, usize, usize, bool, bool)>,
) {
    for child in children_of(all_nodes, parent_id) {
        let cnt = counts.get(&child.id).copied().unwrap_or(0);
        if cnt == 0 {
            continue;
        }
        let has_children = children_of(all_nodes, &child.id)
            .iter()
            .any(|k| counts.get(&k.id).copied().unwrap_or(0) > 0);
        let is_collapsed = collapsed.contains(&collapse_key(&child.id));
        out.push((
            child.id.clone(),
            child.label.clone(),
            depth,
            cnt,
            has_children,
            is_collapsed,
        ));
        if has_children && !is_collapsed {
            dfs_collect(all_nodes, &child.id, depth + 1, counts, collapsed, out);
        }
    }
}

#[component]
pub fn FacetSidebar(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    /// マップビューでのみ渡す。渡されたときはディメンション軸タイトルをクリックで
    /// ズーム軸に設定できるようにする。
    #[prop(optional)]
    zoom_dim: Option<RwSignal<String>>,
) -> impl IntoView {
    let collapsed = create_rw_signal(HashSet::<String>::new());

    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = store.get();
                let tags = selected_tags.get();
                let collapsed_set = collapsed.get();

                roots(&s.dimensions)
                    .into_iter()
                    .filter_map(|root| {
                        // この軸を除いた絞り込みでベースを計算
                        let tags_minus: Vec<_> = tags
                            .iter()
                            .filter(|(k, _)| k != &root.id)
                            .cloned()
                            .collect();
                        let base = filter_resources(&s.resources, &tags_minus, &s.dimensions);

                        // 根の直下ノードから集計（祖先ロールアップ）
                        let mut counts: HashMap<String, usize> = HashMap::new();
                        for r in &base {
                            if let Some(leaf_id) = resolve_dimension(r, &root.id) {
                                // leaf 自身をカウント
                                *counts.entry(leaf_id.clone()).or_default() += 1;
                                // 祖先ノードもカウント（根を除く）
                                for anc in ancestry(&s.dimensions, &leaf_id) {
                                    if anc != leaf_id {
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

                        let mut ordered: Vec<(String, String, usize, usize, bool, bool)> =
                            Vec::new();
                        dfs_collect(
                            &s.dimensions,
                            &root.id,
                            0,
                            &counts,
                            &collapsed_set,
                            &mut ordered,
                        );

                        if ordered.is_empty() {
                            return None;
                        }

                        let root_id = root.id.clone();
                        let root_label = root.label.clone();

                        // ── ディメンション軸タイトル ──────────────────────────
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

                        Some(view! {
                            <div class="facet-panel">
                                {title}
                                {ordered
                                    .into_iter()
                                    .map(|(node_id, node_label, depth, count, has_children, is_collapsed)| {
                                        let is_sel =
                                            selected_val.as_deref() == Some(node_id.as_str());
                                        let indent = format!(
                                            "padding-left:{}rem",
                                            0.25 + depth as f32 * 0.85
                                        );

                                        let caret = if has_children {
                                            let nid = node_id.clone();
                                            view! {
                                                <button
                                                    class="facet-caret"
                                                    on:click=move |_| {
                                                        let key = collapse_key(&nid);
                                                        collapsed.update(|c| {
                                                            if !c.remove(&key) {
                                                                c.insert(key.clone());
                                                            }
                                                        });
                                                    }
                                                >
                                                    {if is_collapsed { "▶" } else { "▼" }}
                                                </button>
                                            }
                                            .into_view()
                                        } else {
                                            view! { <span class="facet-caret-spacer" /> }
                                                .into_view()
                                        };

                                        let rid = root_id.clone();
                                        let nid_click = node_id.clone();
                                        view! {
                                            <div class="facet-row" style=indent>
                                                {caret}
                                                <button
                                                    class=if is_sel {
                                                        "facet-value selected"
                                                    } else {
                                                        "facet-value"
                                                    }
                                                    on:click=move |_| {
                                                        let k = rid.clone();
                                                        let v = nid_click.clone();
                                                        selected_tags.update(|t| {
                                                            let already = t.iter().any(|(tk, tv)| tk == &k && tv == &v);
                                                            t.retain(|(tk, _)| tk != &k);
                                                            if !already {
                                                                t.push((k, v));
                                                            }
                                                        });
                                                    }
                                                >
                                                    <span class="fv-label">{node_label}</span>
                                                    <span class="fv-count">{count}</span>
                                                </button>
                                            </div>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        })
                    })
                    .collect::<Vec<_>>()
            }}
        </aside>
    }
}
