use crate::platform::{DimAttrs, Platform};
use cumulo_model::model::{Bipartite, Resource};
use icondata as icon;
use leptos::*;
use leptos_icons::Icon;

#[component]
pub fn DetailPanel(
    bipartite: ReadSignal<Bipartite<DimAttrs>>,
    selected_id: RwSignal<Option<String>>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let resource = create_memo(move |_| {
        let id = selected_id.get()?;
        let s = bipartite.get();
        s.resources.iter().find(|r| r.id == id).cloned()
    });

    view! {
        <Show when=move || resource.get().is_some()>
            <div class="detail-panel">
                {move || {
                    resource
                        .get()
                        .map(|r| {
                            let url = r.console_url.clone();
                            let freq = r.freq;
                            let r_for_edit = r.clone();
                            let s = bipartite.get();
                            let display = r.display_label(&s.dimensions);

                            let mut dims_sorted: Vec<_> = r.dimensions.into_iter()
                                .map(|(k, v)| {
                                    let k_label = s.dimensions.node(&k)
                                        .map(|n| n.label.clone())
                                        .filter(|l| !l.is_empty())
                                        .unwrap_or_else(|| k.clone());
                                    let v_label = s.dimensions.node(&v)
                                        .map(|n| n.label.clone())
                                        .filter(|l| !l.is_empty())
                                        .unwrap_or_else(|| v.clone());
                                    (k_label, v_label)
                                })
                                .collect();
                            dims_sorted.sort_by_key(|(k, _)| k.clone());
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
                                    <div class="detail-section-title">"ディメンション"</div>
                                    <div class="detail-attrs">
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
