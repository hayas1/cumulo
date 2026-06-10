use super::facet_sidebar::FacetSidebar;
use crate::logic::facet::filter_resources;
use crate::model::{node, AppStore, Resource};
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
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let filtered_ids = create_memo(move |_| {
        let s = store.get();
        let tags = selected_tags.get();
        filter_resources(&s.resources, &tags, &s.dimensions)
            .into_iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    });

    view! {
        <div class="facet-view">
            <div class="facet-body">
                <FacetSidebar store=store selected_tags=selected_tags />

                <main class="facet-results">
                    {move || {
                        let s = store.get();
                        let ids = filtered_ids.get();

                        let resources: Vec<_> = s
                            .resources
                            .iter()
                            .filter(|r| ids.contains(&r.id))
                            .cloned()
                            .collect();

                        if resources.is_empty() {
                            return view! {
                                <div class="facet-empty">
                                    "マッチするリソースがありません"
                                </div>
                            }
                            .into_view();
                        }

                        view! {
                            <div class="results-header-row">
                                <span class="results-count">{resources.len()} " 件"</span>
                                <button
                                    class="add-resource-btn"
                                    on:click=move |_| editing.set(Some(Resource::default()))
                                >
                                    "+ 追加"
                                </button>
                            </div>
                            <div class="results-list">
                                {resources
                                    .into_iter()
                                    .map(|r| {
                                        let url = r.console_url.clone();

                                        // 全 attrs をチップ表示（ノードの label と color を使う）
                                        let mut attrs_sorted: Vec<_> =
                                            r.attrs.iter().collect::<Vec<_>>().into_iter()
                                            .map(|(k, v)| (k.clone(), v.clone()))
                                            .collect();
                                        attrs_sorted.sort_by_key(|(k, _)| k.clone());

                                        let chips: Vec<(String, String, String)> = attrs_sorted
                                            .iter()
                                            .map(|(k, v)| {
                                                let color = node(&s.dimensions, v)
                                                    .map(|n| n.color.clone())
                                                    .unwrap_or_default();
                                                let label = node(&s.dimensions, v)
                                                    .map(|n| n.label.clone())
                                                    .unwrap_or_else(|| v.clone());
                                                (k.clone(), label, color)
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
                                                        .map(|(k, label, color)| {
                                                            let style = if !color.is_empty() {
                                                                format!("border-color:{color};background:{color}1a")
                                                            } else {
                                                                String::new()
                                                            };
                                                            view! {
                                                                <span class="result-chip" style=style>
                                                                    <span class="chip-k">{k}</span>
                                                                    <span class="chip-sep">":"</span>
                                                                    <span class="chip-v">{label}</span>
                                                                </span>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
                                                </div>
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
