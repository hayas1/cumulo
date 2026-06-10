use crate::model::{AppStore, Resource};
use leptos::*;
use web_sys::window;

fn open_url(url: String) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(&url, "_blank");
    }
}

#[component]
pub fn DetailPanel(
    store: ReadSignal<AppStore>,
    selected_id: RwSignal<Option<String>>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let resource = create_memo(move |_| {
        let id = selected_id.get()?;
        let s = store.get();
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

                            let mut attrs_sorted: Vec<_> =
                                r.attrs.into_iter().collect();
                            attrs_sorted.sort_by_key(|(k, _)| k.clone());
                            view! {
                                <div class="detail-header">
                                    <div class="detail-name">{r.name.clone()}</div>
                                    <div class="detail-header-actions">
                                        <button
                                            class="detail-edit-btn"
                                            on:click=move |_| editing.set(Some(r_for_edit.clone()))
                                        >
                                            "編集"
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
                                    <div class="detail-section-title">"属性"</div>
                                    <div class="detail-attrs">
                                        {attrs_sorted
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
                                    <button
                                        class="console-jump-btn"
                                        on:click=move |_| open_url(url.clone())
                                    >
                                        "コンソールへ →"
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
        </Show>
    }
}
