use crate::logic::facet::{filter_resources, resolve_dimension};
use crate::model::{AppStore, Dimension};
use leptos::*;
use std::collections::{HashMap, HashSet};

/// 折りたたみ状態のキー（dim_id と value を区切り文字で連結）
fn collapse_key(dim_id: &str, value: &str) -> String {
    format!("{dim_id}\u{0}{value}")
}

/// 階層dimensionを定義順でDFSし、(value, depth, count, has_children, collapsed) を出力する。
/// count==0 のノードはスキップ。折りたたまれたノードの子孫は出力しない。
fn dfs_collect(
    dim: &Dimension,
    parent: Option<&str>,
    depth: usize,
    counts: &HashMap<String, usize>,
    collapsed: &HashSet<String>,
    out: &mut Vec<(String, usize, usize, bool, bool)>,
) {
    for child in dim.children_of(parent) {
        let cnt = counts.get(&child.value).copied().unwrap_or(0);
        if cnt == 0 {
            continue;
        }
        let has_children = dim
            .children_of(Some(&child.value))
            .iter()
            .any(|k| counts.get(&k.value).copied().unwrap_or(0) > 0);
        let is_collapsed = collapsed.contains(&collapse_key(&dim.id, &child.value));
        out.push((child.value.clone(), depth, cnt, has_children, is_collapsed));
        if has_children && !is_collapsed {
            dfs_collect(dim, Some(&child.value), depth + 1, counts, collapsed, out);
        }
    }
}

#[component]
pub fn FacetSidebar(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    /// マップビューでのみ渡す。渡されたときはディメンションのタイトルをクリックで
    /// ズーム軸に設定できるようにする。
    #[prop(optional)]
    zoom_dim: Option<RwSignal<String>>,
) -> impl IntoView {
    // 折りたたみ済みノード（collapse_key の集合）。ビュー内で永続。
    let collapsed = create_rw_signal(HashSet::<String>::new());

    view! {
        <aside class="facet-sidebar">
            {move || {
                let s = store.get();
                let tags = selected_tags.get();
                let collapsed_set = collapsed.get();

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

                        let hierarchical = dim.is_hierarchical();

                        // (value, depth, count, has_children, collapsed)
                        let ordered: Vec<(String, usize, usize, bool, bool)> = if hierarchical {
                            let mut out = Vec::new();
                            dfs_collect(&dim, None, 0, &counts, &collapsed_set, &mut out);
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
                            vals.into_iter()
                                .map(|(v, c)| (v, 0, c, false, false))
                                .collect()
                        };

                        let dim_id = dim.id.clone();
                        let dim_label = dim.label.clone();

                        // ── ディメンションタイトル（マップ時はズーム軸選択ボタン）──
                        let title = match zoom_dim {
                            Some(zd) => {
                                let did = dim_id.clone();
                                let did_eq = dim_id.clone();
                                view! {
                                    <button
                                        class="facet-panel-title facet-panel-title-btn"
                                        class:active=move || zd.get() == did_eq
                                        title="ズーム軸にする"
                                        on:click=move |_| zd.set(did.clone())
                                    >
                                        {dim_label}
                                    </button>
                                }
                                .into_view()
                            }
                            None => {
                                view! { <div class="facet-panel-title">{dim_label}</div> }
                                    .into_view()
                            }
                        };

                        Some(view! {
                            <div class="facet-panel">
                                {title}
                                {ordered
                                    .into_iter()
                                    .map(|(val, depth, count, has_children, is_collapsed)| {
                                        let is_sel =
                                            selected_val.as_deref() == Some(val.as_str());
                                        let indent = format!(
                                            "padding-left:{}rem",
                                            0.25 + depth as f32 * 0.85
                                        );

                                        // ── 折りたたみキャレット ──
                                        let caret = if has_children {
                                            let did = dim_id.clone();
                                            let vv = val.clone();
                                            view! {
                                                <button
                                                    class="facet-caret"
                                                    on:click=move |_| {
                                                        let key = collapse_key(&did, &vv);
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

                                        let did = dim_id.clone();
                                        let v_click = val.clone();
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
