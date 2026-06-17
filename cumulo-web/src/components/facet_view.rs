use super::facet_sidebar::FacetSidebar;
use crate::platform::{CategoryAttribute, Filters, Platform, ResourceAttribute};
use cumulo_model::{Bipartite, Forest, Resource};
use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn FacetView(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let filtered_ids = Memo::new(move |_| {
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

                        let entities: Vec<_> = s
                            .catalog
                            .iter()
                            .filter(|r| ids.contains(&r.id))
                            .cloned()
                            .collect();

                        if entities.is_empty() {
                            return view! {
                                <div class="facet-empty">
                                    "マッチするリソースがありません"
                                </div>
                            }
                            .into_any();
                        }

                        view! {
                            <div class="results-header-row">
                                <span class="results-count">{entities.len()} " 件"</span>
                                <button
                                    class="add-resource-btn"
                                    on:click=move |_| editing.set(Some(Platform::new_resource()))
                                >
                                    "+ 追加"
                                </button>
                            </div>
                            <div class="results-list">
                                {entities
                                    .into_iter()
                                    .map(|r| {
                                        let url = r.attribute.console_url.clone();

                                        // 軸（根）は root_of で導出する
                                        let mut dims_sorted: Vec<_> = r.categories.iter()
                                            .map(|v| {
                                                let k = s.taxonomy.root_of(v).unwrap_or_else(|| v.clone());
                                                (k, v.clone())
                                            })
                                            .collect();
                                        dims_sorted.sort_by_key(|(k, _)| k.clone());

                                        let chips: Vec<(String, String, String)> = dims_sorted
                                            .iter()
                                            .map(|(k, v)| {
                                                let color = s.taxonomy.node(v)
                                                    .map(|n| n.attribute.color.clone())
                                                    .unwrap_or_default();
                                                let label = s.taxonomy.node(v)
                                                    .map(|n| n.label.clone())
                                                    .unwrap_or_else(|| v.to_string());
                                                (k.to_string(), label, color)
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
                                                        {r.display_label(&s.taxonomy)}
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
                                                <div class="result-value">
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
                        .into_any()
                    }}
                </main>
            </div>
        </div>
    }
}
