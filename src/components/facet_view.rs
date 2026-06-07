use crate::logic::facet::{filter_resources, resolve_dimension};
use crate::model::AppStore;
use leptos::*;
use web_sys::window;

fn open_url(url: &str) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(url, "_blank");
    }
}

#[component]
pub fn FacetView(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
) -> impl IntoView {
    let filtered_parent_ids = create_memo(move |_| {
        let s = store.get();
        let tags = selected_tags.get();
        filter_resources(&s.resources, &tags)
            .into_iter()
            .filter(|r| r.parent_id.is_none())
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    });

    view! {
        <div class="facet-view">
            <div class="facet-body">

                // ── ファセットサイドバー ─────────────────────────────────
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
                                let base = filter_resources(&s.resources, &tags_minus);

                                let mut counts: std::collections::HashMap<String, usize> =
                                    std::collections::HashMap::new();
                                for r in &base {
                                    if r.parent_id.is_some() {
                                        continue;
                                    }
                                    if let Some(val) = resolve_dimension(r, &dim) {
                                        *counts.entry(val).or_default() += 1;
                                    }
                                }

                                if counts.is_empty() {
                                    return None;
                                }

                                let selected_val = tags
                                    .iter()
                                    .find(|(k, _)| k == &dim.id)
                                    .map(|(_, v)| v.clone());

                                let mut vals: Vec<(String, usize)> =
                                    counts.into_iter().collect();
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

                                let dim_id = dim.id.clone();
                                let dim_label = dim.label.clone();

                                Some(view! {
                                    <div class="facet-panel">
                                        <div class="facet-panel-title">{dim_label}</div>
                                        {vals
                                            .into_iter()
                                            .map(|(val, count)| {
                                                let is_sel = selected_val
                                                    .as_deref()
                                                    == Some(val.as_str());
                                                let did = dim_id.clone();
                                                let v_click = val.clone();
                                                view! {
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
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </div>
                                })
                            })
                            .collect::<Vec<_>>()
                    }}
                </aside>

                // ── 結果リスト ───────────────────────────────────────────
                <main class="facet-results">
                    {move || {
                        let s = store.get();
                        let ids = filtered_parent_ids.get();

                        let parents: Vec<_> = s
                            .resources
                            .iter()
                            .filter(|r| r.parent_id.is_none() && ids.contains(&r.id))
                            .cloned()
                            .collect();

                        if parents.is_empty() {
                            return view! {
                                <div class="facet-empty">
                                    "マッチするリソースがありません"
                                </div>
                            }
                            .into_view();
                        }

                        view! {
                            <div class="results-count">{parents.len()} " 件"</div>
                            <div class="results-list">
                                {parents
                                    .into_iter()
                                    .map(|r| {
                                        let url = r.console_url.clone();
                                        let children: Vec<_> = s
                                            .resources
                                            .iter()
                                            .filter(|c| {
                                                c.parent_id.as_deref() == Some(r.id.as_str())
                                            })
                                            .cloned()
                                            .collect();

                                        let eff = r.effective_attrs(&s.resources);
                                        let key_attrs =
                                            ["vendor", "env", "service", "resource_type"];
                                        let chips: Vec<(String, String)> = key_attrs
                                            .iter()
                                            .filter_map(|k| {
                                                eff.get(*k).map(|v| (k.to_string(), v.clone()))
                                            })
                                            .collect();

                                        view! {
                                            <div class="result-card">
                                                <div class="result-card-header">
                                                    <span class="result-name">{r.name.clone()}</span>
                                                    <button
                                                        class="result-open-btn"
                                                        on:click=move |_| open_url(&url)
                                                    >
                                                        "コンソールへ →"
                                                    </button>
                                                </div>
                                                <div class="result-attrs">
                                                    {chips
                                                        .into_iter()
                                                        .map(|(k, v)| {
                                                            view! {
                                                                <span class="result-chip">
                                                                    <span class="chip-k">{k}</span>
                                                                    <span class="chip-sep">":"</span>
                                                                    <span class="chip-v">{v}</span>
                                                                </span>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
                                                </div>
                                                {if !children.is_empty() {
                                                    Some(view! {
                                                        <div class="result-children">
                                                            {children
                                                                .into_iter()
                                                                .map(|c| {
                                                                    let curl = c.console_url.clone();
                                                                    view! {
                                                                        <div class="result-child">
                                                                            <span class="child-indent">"↳"</span>
                                                                            <span class="result-child-name">
                                                                                {c.name.clone()}
                                                                            </span>
                                                                            <button
                                                                                class="result-child-btn"
                                                                                on:click=move |_| open_url(&curl)
                                                                            >
                                                                                "→"
                                                                            </button>
                                                                        </div>
                                                                    }
                                                                })
                                                                .collect::<Vec<_>>()}
                                                        </div>
                                                    })
                                                } else {
                                                    None
                                                }}
                                            </div>
                                        }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }
                        .into_view()
                    }}
                </main>
            </div>
        </div>
    }
}
