use crate::model::{AppStore, Resource};
use crate::storage::save_to_storage;
use leptos::*;

fn gen_id() -> String {
    let n = (js_sys::Math::random() * 1e15) as u64;
    format!("r{n:x}")
}

#[component]
pub fn ResourceForm(
    store: RwSignal<AppStore>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let form_name = create_rw_signal(String::new());
    let form_url = create_rw_signal(String::new());
    let form_freq = create_rw_signal(1u32);
    let form_parent = create_rw_signal(Option::<String>::None);
    let form_attrs = create_rw_signal(Vec::<(String, String)>::new());

    // editing が変わるたびにフォームを初期化
    create_effect(move |_| {
        if let Some(r) = editing.get() {
            form_name.set(r.name.clone());
            form_url.set(r.console_url.clone());
            form_freq.set(r.freq.max(1));
            form_parent.set(r.parent_id.clone());
            let mut attrs: Vec<_> = r.attrs.into_iter().collect();
            attrs.sort_by_key(|(k, _)| k.clone());
            form_attrs.set(attrs);
        }
    });

    let is_new = move || {
        editing.with(|e| e.as_ref().map(|r| r.id.is_empty()).unwrap_or(false))
    };

    let save = move || {
        let name = form_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        let id = editing.with_untracked(|e| {
            e.as_ref()
                .filter(|r| !r.id.is_empty())
                .map(|r| r.id.clone())
        })
        .unwrap_or_else(gen_id);

        let r = Resource {
            id: id.clone(),
            name,
            console_url: form_url.get_untracked(),
            freq: form_freq.get_untracked(),
            parent_id: form_parent.get_untracked(),
            attrs: form_attrs.get_untracked().into_iter().collect(),
            created_at: None,
        };

        store.update(|s| {
            if let Some(pos) = s.resources.iter().position(|x| x.id == id) {
                s.resources[pos] = r;
            } else {
                s.resources.push(r);
            }
        });
        save_to_storage(&store.get_untracked());
        editing.set(None);
    };

    view! {
        <Show when=move || editing.with(|e| e.is_some())>
            <div class="form-backdrop" on:click=move |_| editing.set(None) />
            <div class="form-panel">
                <div class="form-header">
                    <span class="form-title">
                        {move || if is_new() { "リソースを追加" } else { "リソースを編集" }}
                    </span>
                    <button class="form-close" on:click=move |_| editing.set(None)>
                        "×"
                    </button>
                </div>

                <div class="form-body">
                    <label class="form-label">"名前"</label>
                    <input
                        class="form-input"
                        type="text"
                        placeholder="例: auth-bigquery-prod"
                        prop:value=move || form_name.get()
                        on:input=move |ev| form_name.set(event_target_value(&ev))
                    />

                    <label class="form-label">"コンソール URL"</label>
                    <input
                        class="form-input"
                        type="text"
                        placeholder="https://..."
                        prop:value=move || form_url.get()
                        on:input=move |ev| form_url.set(event_target_value(&ev))
                    />

                    <label class="form-label">"アクセス頻度"</label>
                    <input
                        class="form-input form-input-sm"
                        type="number"
                        min="0"
                        prop:value=move || form_freq.get().to_string()
                        on:input=move |ev| {
                            if let Ok(n) = event_target_value(&ev).parse::<u32>() {
                                form_freq.set(n);
                            }
                        }
                    />

                    <label class="form-label">"親リソース"</label>
                    <select
                        class="form-input"
                        prop:value=move || form_parent.get().unwrap_or_default()
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            form_parent.set(if v.is_empty() { None } else { Some(v) });
                        }
                    >
                        <option value="">"なし"</option>
                        {move || {
                            let s = store.get();
                            let cur_id = editing
                                .with(|e| {
                                    e.as_ref().map(|r| r.id.clone()).unwrap_or_default()
                                });
                            s.resources
                                .iter()
                                .filter(|r| r.parent_id.is_none() && r.id != cur_id)
                                .map(|r| {
                                    view! {
                                        <option value={r.id.clone()}>{r.name.clone()}</option>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </select>

                    <label class="form-label">"属性"</label>
                    {move || {
                        form_attrs
                            .get()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (k, v))| {
                                view! {
                                    <div class="form-attr-row">
                                        <input
                                            class="form-input form-attr-key"
                                            type="text"
                                            placeholder="キー"
                                            prop:value=k
                                            on:input=move |ev| {
                                                let val = event_target_value(&ev);
                                                form_attrs
                                                    .update(|a| {
                                                        if let Some(row) = a.get_mut(i) {
                                                            row.0 = val;
                                                        }
                                                    });
                                            }
                                        />
                                        <input
                                            class="form-input form-attr-val"
                                            type="text"
                                            placeholder="値"
                                            prop:value=v
                                            on:input=move |ev| {
                                                let val = event_target_value(&ev);
                                                form_attrs
                                                    .update(|a| {
                                                        if let Some(row) = a.get_mut(i) {
                                                            row.1 = val;
                                                        }
                                                    });
                                            }
                                        />
                                        <button
                                            class="form-attr-remove"
                                            on:click=move |_| {
                                                form_attrs.update(|a| { a.remove(i); });
                                            }
                                        >
                                            "×"
                                        </button>
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                    <button
                        class="form-add-attr-btn"
                        on:click=move |_| {
                            form_attrs
                                .update(|a| a.push((String::new(), String::new())));
                        }
                    >
                        "+ 属性を追加"
                    </button>
                </div>

                <div class="form-footer">
                    <button class="form-cancel-btn" on:click=move |_| editing.set(None)>
                        "キャンセル"
                    </button>
                    <button class="form-save-btn" on:click=move |_| save()>
                        "保存"
                    </button>
                </div>
            </div>
        </Show>
    }
}
