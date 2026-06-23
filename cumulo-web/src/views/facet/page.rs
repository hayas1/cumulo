//! ファセット画面（ルート `/` `/facet`）。サイドバーで絞り込み、一致リソースを一覧表示する枠。

use super::sidebar::FacetSidebar;
use crate::category::{CategoryAttribute, Filters};
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use cumulo_model::{Bipartite, Forest, Resource, Selection};
use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn FacetView(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    view! {
        <div class="facet-view">
            <div class="facet-body">
                <FacetSidebar bipartite=bipartite selected_tags=selected_tags />

                <main class="facet-results">
                    {move || {
                        let s = bipartite.get();
                        let tags = selected_tags.get();

                        // 一致リソースは filtered() の結果をそのまま使う（id 集合は作らない）
                        let entities: Vec<_> =
                            s.filtered(&tags).items().iter().map(|r| (*r).clone()).collect();

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

                                        // 森射影・並べ替えはモデル（rooted_nodes）に委譲し、ここはラベル/色の解決のみ
                                        let chips: Vec<(String, String, String)> = r.rooted_nodes(&s.taxonomy)
                                            .into_iter()
                                            .map(|(k, v)| {
                                                let color = s.taxonomy.node(&v)
                                                    .and_then(|n| n.attribute.color)
                                                    .map(|c| c.to_hex())
                                                    .unwrap_or_default();
                                                let label = s.taxonomy.node(&v)
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
