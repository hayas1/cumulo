use crate::logic::facet::filter_resources;
use crate::model::{AppStore, Resource};
use leptos::*;
use super::facet_sidebar::FacetSidebar;
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
    editing: RwSignal<Option<Resource>>,
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
                <FacetSidebar store=store selected_tags=selected_tags />

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
                            <div class="results-header-row">
                                <span class="results-count">{parents.len()} " 件"</span>
                                <button
                                    class="add-resource-btn"
                                    on:click=move |_| editing.set(Some(Resource::default()))
                                >
                                    "+ 追加"
                                </button>
                            </div>
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

                                        let r_for_edit = r.clone();
                                        view! {
                                            <div class="result-card">
                                                <div class="result-card-header">
                                                    <span class="result-name">{r.name.clone()}</span>
                                                    <div class="result-card-actions">
                                                        <button
                                                            class="result-edit-btn"
                                                            on:click=move |_| {
                                                                editing.set(Some(r_for_edit.clone()))
                                                            }
                                                        >
                                                            "編集"
                                                        </button>
                                                        <button
                                                            class="result-open-btn"
                                                            on:click=move |_| open_url(&url)
                                                        >
                                                            "コンソールへ →"
                                                        </button>
                                                    </div>
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
