use crate::logic::facet::{filter_resources, resolve_dimension};
use crate::model::AppStore;
use leptos::*;

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

                        let mut vals: Vec<(String, usize)> = counts.into_iter().collect();
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
                                        let is_sel =
                                            selected_val.as_deref() == Some(val.as_str());
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
    }
}
