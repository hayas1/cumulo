use crate::model::{children_of, roots, AppStore, Resource};
use crate::storage::save_to_storage;
use leptos::html::Input;
use leptos::*;

fn gen_id() -> String {
    let n = (js_sys::Math::random() * 1e15) as u64;
    format!("r{n:x}")
}

/// 指定した根の下にある全ノードを DFS 順で (id, label, color, depth) として返す。
fn descendants_dfs(
    store: &AppStore,
    root_id: &str,
) -> Vec<(String, String, String, usize)> {
    let mut out = Vec::new();
    fn dfs(
        nodes: &[crate::model::DimensionNode],
        parent_id: &str,
        depth: usize,
        out: &mut Vec<(String, String, String, usize)>,
    ) {
        for n in children_of(nodes, parent_id) {
            out.push((n.id.clone(), n.label.clone(), n.color.clone(), depth));
            dfs(nodes, &n.id, depth + 1, out);
        }
    }
    dfs(&store.dimensions, root_id, 0, &mut out);
    out
}

#[component]
pub fn ResourceForm(
    store: RwSignal<AppStore>,
    editing: RwSignal<Option<Resource>>,
) -> impl IntoView {
    let form_name = create_rw_signal(String::new());
    let form_url = create_rw_signal(String::new());
    let form_freq = create_rw_signal(1u32);

    // Attrs: stable id + key + value
    let next_id = create_rw_signal(0u32);
    let form_attrs = create_rw_signal(Vec::<(u32, String, String)>::new());

    let name_ref = create_node_ref::<Input>();
    let url_ref = create_node_ref::<Input>();
    let freq_ref = create_node_ref::<Input>();

    create_effect(move |_| {
        let Some(r) = editing.get() else { return };

        form_name.set(r.name.clone());
        form_url.set(r.console_url.clone());
        form_freq.set(r.freq.max(1));

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
                        placeholder="例: auth / BigQuery (prod)"
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

                    // ── 軸ごとのディメンションチップ ──────────────────────────
                    <label class="form-label">"属性"</label>
                    {move || {
                        let s = store.get();
                        roots(&s.dimensions)
                            .into_iter()
                            .map(|root| {
                                let root_id = root.id.clone();
                                let root_label = if root.label.is_empty() {
                                    root.id.clone()
                                } else {
                                    root.label.clone()
                                };
                                let chips = descendants_dfs(&s, &root.id);
                                view! {
                                    <div class="form-dim-row">
                                        <span class="form-dim-label">{root_label}</span>
                                        <div class="form-dim-chips">
                                            {chips
                                                .into_iter()
                                                .map(|(node_id, node_label, color, _depth)| {
                                                    let k_sel = root_id.clone();
                                                    let v_sel = node_id.clone();
                                                    let k_clk = root_id.clone();
                                                    let v_clk = node_id.clone();
                                                    let style = if !color.is_empty() {
                                                        format!(
                                                            "border-color:{color};background:{color}1a"
                                                        )
                                                    } else {
                                                        String::new()
                                                    };
                                                    view! {
                                                        <span
                                                            class="attr-chip"
                                                            class:selected=move || {
                                                                form_attrs
                                                                    .get()
                                                                    .iter()
                                                                    .any(|(_, k, v)| k == &k_sel && v == &v_sel)
                                                            }
                                                            style=style
                                                            on:click=move |_| {
                                                                let already = form_attrs
                                                                    .get_untracked()
                                                                    .iter()
                                                                    .any(|(_, k, v)| k == &k_clk && v == &v_clk);
                                                                if already {
                                                                    form_attrs.update(|a| {
                                                                        a.retain(|(_, k, _)| k != &k_clk)
                                                                    });
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
                                                            {node_label}
                                                        </span>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}

                    // ── ディメンションに含まれないフリーな属性 ────────────────
                    <For
                        each=move || {
                            let root_ids = store.with(|s| {
                                roots(&s.dimensions)
                                    .into_iter()
                                    .map(|r| r.id.clone())
                                    .collect::<Vec<_>>()
                            });
                            form_attrs
                                .get()
                                .into_iter()
                                .filter(|(_, k, _)| !root_ids.contains(k))
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
                            form_attrs
                                .update(|a| a.push((id, String::new(), String::new())));
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

#[component]
fn AttrRow(
    row_id: u32,
    initial_key: String,
    initial_val: String,
    form_attrs: RwSignal<Vec<(u32, String, String)>>,
) -> impl IntoView {
    let key_ref = create_node_ref::<Input>();
    let val_ref = create_node_ref::<Input>();

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
                    form_attrs.update(|a| a.retain(|(id, _, _)| *id != row_id));
                }
            >
                "×"
            </button>
        </div>
    }
}
