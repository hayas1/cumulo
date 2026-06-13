use super::facet_sidebar::FacetSidebar;
use crate::platform::{DimAttrs, Platform};
use cumulo_model::model::{Bipartite, Resource};
use icondata as icon;
use leptos::*;
use leptos_icons::Icon;

#[component]
pub fn FacetView(
    bipartite: ReadSignal<Bipartite<DimAttrs>>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let filtered_ids = create_memo(move |_| {
        let s = bipartite.get();
        let tags = selected_tags.get();
        s.filter_resources(&tags)
            .into_iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    });

    view! {
        <div class="facet-view">
            <div class="facet-body">
                <FacetSidebar bipartite=bipartite selected_tags=selected_tags />

                <main class="facet-results">
                    {move || {
                        let s = bipartite.get();
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

                                        let mut dims_sorted: Vec<_> = r.dimensions.iter()
                                            .map(|(k, v)| (k.clone(), v.clone()))
                                            .collect();
                                        dims_sorted.sort_by_key(|(k, _)| k.clone());

                                        let chips: Vec<(String, String, String)> = dims_sorted
                                            .iter()
                                            .map(|(k, v)| {
                                                let color = s.dimensions.node(v)
                                                    .map(|n| n.attrs.color.clone())
                                                    .unwrap_or_default();
                                                let label = s.dimensions.node(v)
                                                    .map(|n| n.label.clone())
                                                    .unwrap_or_else(|| v.clone());
                                                (k.clone(), label, color)
                                            })
                                            .collect();

                                        let r_for_edit = r.clone();
                                        view! {
                                            <div class="result-card">
                                                <div class="result-card-header">
                                                    <span
                                                        class="result-name result-name-link"
                                                        on:click=move |_| Platform::open_url(&url)
                                                    >
                                                        {r.display_label(&s.dimensions)}
                                                    </span>
                                                    <div class="result-card-actions">
                                                        <button
                                                            class="result-edit-btn"
                                                            on:click=move |_| {
                                                                editing.set(Some(r_for_edit.clone()))
                                                            }
                                                            title="編集"
                                                        >
                                                            <Icon icon=icon::HiPencilOutlineLg width="14" height="14" />
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
