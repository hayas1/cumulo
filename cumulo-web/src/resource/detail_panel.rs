use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::resource::{ResourceAttribute, ResourceId};
use cumulo_model::{Forest, Resource};
use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn DetailPanel(
    client: Client,
    selected_id: RwSignal<Option<ResourceId>>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let bipartite = client.read();
    let resource = Memo::new(move |_| {
        let id = selected_id.get()?;
        let s = bipartite.get();
        s.catalog.node(&id).cloned()
    });

    view! {
        <Show when=move || resource.get().is_some()>
            <div class="detail-panel">
                {move || {
                    resource
                        .get()
                        .map(|r| {
                            let url = r.attribute.console_url.clone();
                            let freq = r.attribute.freq;
                            let r_for_edit = r.clone();
                            let s = bipartite.get();
                            let display = r.display_label(&s.taxonomy);

                            let dims_sorted: Vec<(String, String)> = r.rooted_nodes(&s.taxonomy)
                                .into_iter()
                                .map(|(k, v)| {
                                    let k_label = s.taxonomy.node(&k)
                                        .map(|n| n.label.clone())
                                        .filter(|l| !l.is_empty())
                                        .unwrap_or_else(|| k.to_string());
                                    let v_label = s.taxonomy.node(&v)
                                        .map(|n| n.label.clone())
                                        .filter(|l| !l.is_empty())
                                        .unwrap_or_else(|| v.to_string());
                                    (k_label, v_label)
                                })
                                .collect();
                            view! {
                                <div class="detail-header">
                                    <div
                                        class="detail-name detail-name-link"
                                        on:click=move |_| Platform::open_url(&url)
                                    >
                                        {display}
                                    </div>
                                    <div class="detail-header-actions">
                                        <button
                                            class="detail-edit-btn"
                                            on:click=move |_| editing.set(Some(r_for_edit.clone()))
                                            title="編集"
                                        >
                                            <Icon icon=icon::HiPencilOutlineLg width="14" height="14" />
                                        </button>
                                        <button
                                            class="detail-close"
                                            on:click=move |_| selected_id.set(None)
                                        >
                                            "×"
                                        </button>
                                    </div>
                                </div>

                                <div class="detail-body">
                                    <div class="detail-section-title">"カテゴリ"</div>
                                    <div class="detail-value">
                                        {dims_sorted
                                            .into_iter()
                                            .map(|(k, v)| {
                                                view! {
                                                    <div class="detail-attr-row">
                                                        <span class="detail-attr-key">{k}</span>
                                                        <span class="detail-attr-val">{v}</span>
                                                    </div>
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </div>
                                </div>

                                <div class="detail-footer">
                                    <span class="detail-freq">
                                        "アクセス頻度: " {freq}
                                    </span>
                                </div>
                            }
                        })
                }}
            </div>
        </Show>
    }
}
