use crate::logic::facet::{filter_resources, resolve_dimension};
use crate::model::{AppStore, Dimension};
use leptos::*;
use std::collections::HashMap;

/// 階層dimensionを定義順でDFSし、(value, depth, count) を出力する。
/// count==0 のノード（現在の絞り込みで該当なし）はスキップする。
fn dfs_collect(
    dim: &Dimension,
    parent: Option<&str>,
    depth: usize,
    counts: &HashMap<String, usize>,
    out: &mut Vec<(String, usize, usize)>,
) {
    for child in dim.children_of(parent) {
        let cnt = counts.get(&child.value).copied().unwrap_or(0);
        if cnt == 0 {
            continue;
        }
        out.push((child.value.clone(), depth, cnt));
        dfs_collect(dim, Some(&child.value), depth + 1, counts, out);
    }
}

#[component]
pub fn FacetSidebar(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
) -> impl IntoView {
    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = store.get();
                let tags = selected_tags.get();

                s.dimensions
                    .clone()
                    .into_iter()
                    .filter_map(|dim| {
                        let tags_minus: Vec<_> = tags
                            .iter()
                            .filter(|(k, _)| k != &dim.id)
                            .cloned()
                            .collect();
                        let base = filter_resources(&s.resources, &tags_minus, &s.dimensions);

                        // 祖先まで展開してロールアップ集計（中間ノードにも件数が乗る）
                        let mut counts: HashMap<String, usize> = HashMap::new();
                        for r in &base {
                            if r.parent_id.is_some() {
                                continue;
                            }
                            if let Some(leaf) = resolve_dimension(r, &dim) {
                                for anc in dim.ancestry(&leaf) {
                                    *counts.entry(anc).or_default() += 1;
                                }
                            }
                        }

                        if counts.is_empty() {
                            return None;
                        }

                        let selected_val = tags
                            .iter()
                            .find(|(k, _)| k == &dim.id)
                            .map(|(_, v)| v.clone());

                        // 表示順を (value, depth, count) に正規化
                        let ordered: Vec<(String, usize, usize)> = if dim.is_hierarchical() {
                            let mut out = Vec::new();
                            dfs_collect(&dim, None, 0, &counts, &mut out);
                            out
                        } else {
                            let mut vals: Vec<(String, usize)> =
                                counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
                            if !dim.values.is_empty() {
                                vals.sort_by_key(|(v, _)| {
                                    dim.values
                                        .iter()
                                        .position(|dv| &dv.value == v)
                                        .unwrap_or(usize::MAX)
                                });
                            } else {
                                vals.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                            }
                            vals.into_iter().map(|(v, c)| (v, 0, c)).collect()
                        };

                        let dim_id = dim.id.clone();
                        let dim_label = dim.label.clone();

                        Some(view! {
                            <div class="facet-panel">
                                <div class="facet-panel-title">{dim_label}</div>
                                {ordered
                                    .into_iter()
                                    .map(|(val, depth, count)| {
                                        let is_sel =
                                            selected_val.as_deref() == Some(val.as_str());
                                        let did = dim_id.clone();
                                        let v_click = val.clone();
                                        let indent =
                                            format!("padding-left:{}rem", 0.5 + depth as f32 * 0.85);
                                        view! {
                                            <button
                                                class=if is_sel {
                                                    "facet-value selected"
                                                } else {
                                                    "facet-value"
                                                }
                                                style=indent
                                                on:click=move |_| {
                                                    let d = did.clone();
                                                    let vv = v_click.clone();
                                                    selected_tags.update(|t| {
                                                        let already = t
                                                            .iter()
                                                            .any(|(k, tv)| k == &d && tv == &vv);
                                                        t.retain(|(k, _)| k != &d);
                                                        if !already {
                                                            t.push((d, vv));
                                                        }
                                                    });
                                                }
                                            >
                                                <span class="fv-dot">
                                                    {if is_sel { "●" } else { "○" }}
                                                </span>
                                                <span class="fv-label">{val}</span>
                                                <span class="fv-count">{count}</span>
                                            </button>
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
