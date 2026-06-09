use crate::model::{AppStore, Resource};
use crate::storage::save_to_storage;
use leptos::html::Input;
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
    // Local form state – plain RwSignals for static fields
    let form_name = create_rw_signal(String::new());
    let form_url = create_rw_signal(String::new());
    let form_freq = create_rw_signal(1u32);
    let form_parent = create_rw_signal(Option::<String>::None);

    // Attrs carry a stable u32 ID so <For> can diff without recreating rows
    let next_id = create_rw_signal(0u32);
    let form_attrs = create_rw_signal(Vec::<(u32, String, String)>::new());

    // NodeRefs let us set input values imperatively (avoids prop:value re-render)
    let name_ref = create_node_ref::<Input>();
    let url_ref = create_node_ref::<Input>();
    let freq_ref = create_node_ref::<Input>();

    // Populate form whenever `editing` changes
    create_effect(move |_| {
        let Some(r) = editing.get() else { return };

        form_name.set(r.name.clone());
        form_url.set(r.console_url.clone());
        form_freq.set(r.freq.max(1));
        form_parent.set(r.parent_id.clone());

        // Set DOM values imperatively so no prop:value re-render is needed
        if let Some(el) = name_ref.get() {
            el.set_value(&r.name);
        }
        if let Some(el) = url_ref.get() {
            el.set_value(&r.console_url);
        }
        if let Some(el) = freq_ref.get() {
            el.set_value(&r.freq.max(1).to_string());
        }

        let mut attrs: Vec<_> = r.attrs.into_iter().collect();
        attrs.sort_by_key(|(k, _)| k.clone());

        // Assign stable IDs and reset counter
        let mut id = 0u32;
        let rows: Vec<(u32, String, String)> = attrs
            .into_iter()
            .map(|(k, v)| {
                let cur = id;
                id += 1;
                (cur, k, v)
            })
            .collect();
        next_id.set(id);
        form_attrs.set(rows);
    });

    let is_new = move || editing.with(|e| e.as_ref().map(|r| r.id.is_empty()).unwrap_or(false));

    let save = move || {
        let name = form_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }

        let id = editing
            .with_untracked(|e| {
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
            attrs: form_attrs
                .get_untracked()
                .into_iter()
                .map(|(_, k, v)| (k, v))
                .collect(),
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
                        node_ref=name_ref
                        class="form-input"
                        type="text"
                        placeholder="例: auth-bigquery-prod"
                        on:input=move |ev| form_name.set(event_target_value(&ev))
                    />

                    <label class="form-label">"コンソール URL"</label>
                    <input
                        node_ref=url_ref
                        class="form-input"
                        type="text"
                        placeholder="https://..."
                        on:input=move |ev| form_url.set(event_target_value(&ev))
                    />

                    <label class="form-label">"アクセス頻度"</label>
                    <input
                        node_ref=freq_ref
                        class="form-input form-input-sm"
                        type="number"
                        min="0"
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
                            let cur_id = editing.with(|e| {
                                e.as_ref().map(|r| r.id.clone()).unwrap_or_default()
                            });
                            s.resources
                                .iter()
                                .filter(|r| r.parent_id.is_none() && r.id != cur_id)
                                .map(|r| view! {
                                    <option value={r.id.clone()}>{r.name.clone()}</option>
                                })
                                .collect::<Vec<_>>()
                        }}
                    </select>

                    // ── Dimension chip selectors ─────────────────────
                    <label class="form-label">"属性"</label>
                    {move || {
                        store.get().dimensions.into_iter().map(|dim| {
                            let dim_id = dim.id.clone();
                            let label = if dim.label.is_empty() { dim.id.clone() } else { dim.label.clone() };
                            view! {
                                <div class="form-dim-row">
                                    <span class="form-dim-label">{label}</span>
                                    <div class="form-dim-chips">
                                        {dim.values.into_iter().map(|dv| {
                                            let val   = dv.value.clone();
                                            let color = dv.color.clone();
                                            let k_sel = dim_id.clone();
                                            let v_sel = val.clone();
                                            let k_clk = dim_id.clone();
                                            let v_clk = val.clone();
                                            let style = color.as_deref()
                                                .filter(|c| !c.is_empty())
                                                .map(|c| format!("border-color:{c};background:{c}1a"))
                                                .unwrap_or_default();
                                            view! {
                                                <span
                                                    class="attr-chip"
                                                    class:selected=move || form_attrs.get().iter()
                                                        .any(|(_, k, v)| k == &k_sel && v == &v_sel)
                                                    style=style
                                                    on:click=move |_| {
                                                        let already = form_attrs.get_untracked().iter()
                                                            .any(|(_, k, v)| k == &k_clk && v == &v_clk);
                                                        if already {
                                                            form_attrs.update(|a| a.retain(|(_, k, _)| k != &k_clk));
                                                        } else {
                                                            let nid = next_id.get_untracked();
                                                            next_id.set(nid + 1);
                                                            form_attrs.update(|a| {
                                                                a.retain(|(_, k, _)| k != &k_clk);
                                                                a.push((nid, k_clk.clone(), v_clk.clone()));
                                                            });
                                                        }
                                                    }
                                                >
                                                    {val}
                                                </span>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}

                    // ── Free-form attrs (keys not covered by any dimension) ───
                    <For
                        each=move || {
                            let dim_ids = store.with(|s| {
                                s.dimensions.iter().map(|d| d.id.clone()).collect::<Vec<_>>()
                            });
                            form_attrs.get().into_iter()
                                .filter(|(_, k, _)| !dim_ids.contains(k))
                                .collect::<Vec<_>>()
                        }
                        key=|(id, _, _)| *id
                        children=move |(row_id, k, v)| {
                            view! {
                                <AttrRow
                                    row_id=row_id
                                    initial_key=k
                                    initial_val=v
                                    form_attrs=form_attrs
                                />
                            }
                        }
                    />
                    <button
                        class="form-add-attr-btn"
                        on:click=move |_| {
                            let id = next_id.get_untracked();
                            next_id.set(id + 1);
                            form_attrs.update(|a| a.push((id, String::new(), String::new())));
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

/// A single attribute row that manages its own inputs without causing list re-renders.
#[component]
fn AttrRow(
    row_id: u32,
    initial_key: String,
    initial_val: String,
    form_attrs: RwSignal<Vec<(u32, String, String)>>,
) -> impl IntoView {
    let key_ref = create_node_ref::<Input>();
    let val_ref = create_node_ref::<Input>();

    // Set DOM values once on mount (initial_key/val are owned strings, not reactive)
    let ik = initial_key.clone();
    let iv = initial_val.clone();
    create_effect(move |_| {
        if let Some(el) = key_ref.get() {
            el.set_value(&ik);
        }
        if let Some(el) = val_ref.get() {
            el.set_value(&iv);
        }
    });

    view! {
        <div class="form-attr-row">
            <input
                node_ref=key_ref
                class="form-input form-attr-key"
                type="text"
                placeholder="キー"
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    // update_untracked: stores the new value without triggering <For> re-diff
                    form_attrs.update_untracked(|a| {
                        if let Some(row) = a.iter_mut().find(|(id, _, _)| *id == row_id) {
                            row.1 = val;
                        }
                    });
                }
            />
            <input
                node_ref=val_ref
                class="form-input form-attr-val"
                type="text"
                placeholder="値"
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    form_attrs.update_untracked(|a| {
                        if let Some(row) = a.iter_mut().find(|(id, _, _)| *id == row_id) {
                            row.2 = val;
                        }
                    });
                }
            />
            <button
                class="form-attr-remove"
                on:click=move |_| {
                    // Structural change: use tracked update so <For> re-diffs
                    form_attrs.update(|a| a.retain(|(id, _, _)| *id != row_id));
                }
            >
                "×"
            </button>
        </div>
    }
}
