use crate::model::{AppStore, Dimension, DimensionValue};
use crate::storage::save_to_storage;
use leptos::html::Input;
use leptos::*;

fn confirm(msg: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(msg).ok())
        .unwrap_or(false)
}

fn new_dim_id() -> String {
    let n = (js_sys::Math::random() * 1e15) as u64;
    format!("dim{n:x}")
}

fn commit_chip(
    editing_chip: RwSignal<Option<(usize, usize)>>,
    val_ref: NodeRef<Input>,
    color_ref: NodeRef<Input>,
    store: RwSignal<AppStore>,
) {
    let Some((di, vi)) = editing_chip.get_untracked() else { return };
    let new_val = val_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
    let new_color = color_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            if let Some(v) = dim.values.get_mut(vi) {
                v.value = new_val;
                v.color = if new_color.is_empty() { None } else { Some(new_color) };
            }
        }
    });
    save_to_storage(&store.get_untracked());
    editing_chip.set(None);
}

fn remove_chip(editing_chip: RwSignal<Option<(usize, usize)>>, store: RwSignal<AppStore>) {
    let Some((di, vi)) = editing_chip.get_untracked() else { return };
    if !confirm("この値を削除しますか？") { return; }
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            if vi < dim.values.len() {
                dim.values.remove(vi);
            }
        }
    });
    save_to_storage(&store.get_untracked());
    editing_chip.set(None);
}

fn commit_dim(
    editing_dim: RwSignal<Option<usize>>,
    label_ref: NodeRef<Input>,
    id_ref: NodeRef<Input>,
    store: RwSignal<AppStore>,
) {
    let Some(di) = editing_dim.get_untracked() else { return };
    let new_label = label_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
    let new_id = id_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
    if !new_id.trim().is_empty() {
        store.update(|s| {
            if let Some(dim) = s.dimensions.get_mut(di) {
                dim.label = new_label;
                dim.id = new_id;
            }
        });
        save_to_storage(&store.get_untracked());
    }
    editing_dim.set(None);
}

#[component]
pub fn DimensionsTab(store: RwSignal<AppStore>) -> impl IntoView {
    let editing_chip = create_rw_signal(Option::<(usize, usize)>::None);
    let editing_dim = create_rw_signal(Option::<usize>::None);

    let val_ref = create_node_ref::<Input>();
    let color_ref = create_node_ref::<Input>();
    let label_ref = create_node_ref::<Input>();
    let id_ref = create_node_ref::<Input>();

    // Populate chip editor inputs when editing_chip changes
    create_effect(move |_| {
        let Some((di, vi)) = editing_chip.get() else { return };
        let s = store.get_untracked();
        let Some(dim) = s.dimensions.get(di) else { return };
        let Some(val) = dim.values.get(vi) else { return };
        if let Some(el) = val_ref.get() { el.set_value(&val.value); }
        if let Some(el) = color_ref.get() {
            el.set_value(val.color.as_deref().unwrap_or("#888888"));
        }
    });

    // Populate dim name editor inputs when editing_dim changes
    create_effect(move |_| {
        let Some(di) = editing_dim.get() else { return };
        let s = store.get_untracked();
        let Some(dim) = s.dimensions.get(di) else { return };
        if let Some(el) = label_ref.get() { el.set_value(&dim.label); }
        if let Some(el) = id_ref.get() { el.set_value(&dim.id); }
    });

    view! {
        <div class="dim-tab">
            {move || {
                let s = store.get();
                s.dimensions.clone().into_iter().enumerate().map(|(di, dim)| {
                    let dim_id_del = dim.id.clone();

                    view! {
                        <div class="dim-row">
                            // ── Dimension header ─────────────────────────
                            <div class="dim-row-header">
                                {move || {
                                    if editing_dim.get() == Some(di) {
                                        view! {
                                            <div class="dim-name-editor">
                                                <input
                                                    node_ref=label_ref
                                                    class="dim-input"
                                                    type="text"
                                                    placeholder="ラベル"
                                                />
                                                <span class="dim-name-sep">"/"</span>
                                                <input
                                                    node_ref=id_ref
                                                    class="dim-input dim-input-id"
                                                    type="text"
                                                    placeholder="id"
                                                />
                                                <button
                                                    class="dim-commit-btn"
                                                    on:click=move |_| commit_dim(editing_dim, label_ref, id_ref, store)
                                                >
                                                    "完了"
                                                </button>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <button
                                                class="dim-name-btn"
                                                on:click=move |_| {
                                                    commit_chip(editing_chip, val_ref, color_ref, store);
                                                    editing_dim.set(Some(di));
                                                }
                                            >
                                                <span class="dim-label-text">
                                                    {if dim.label.is_empty() { "（ラベルなし）".into() } else { dim.label.clone() }}
                                                </span>
                                                <span class="dim-id-text">{dim.id.clone()}</span>
                                            </button>
                                        }.into_view()
                                    }
                                }}
                                <button
                                    class="dim-row-delete"
                                    on:click=move |_| {
                                        if !confirm("このディメンションを削除しますか？") { return; }
                                        editing_chip.set(None);
                                        editing_dim.set(None);
                                        store.update(|s| s.dimensions.retain(|d| d.id != dim_id_del));
                                        save_to_storage(&store.get_untracked());
                                    }
                                >
                                    "×"
                                </button>
                            </div>

                            // ── Value chips ───────────────────────────────
                            <div class="dim-chips">
                                {dim.values.iter().enumerate().map(|(vi, val)| {
                                    let swatch = val.color.clone().unwrap_or_else(|| "#6e7681".into());
                                    view! {
                                        <button
                                            class="val-chip"
                                            class:editing=move || editing_chip.get() == Some((di, vi))
                                            on:click=move |_| {
                                                editing_dim.set(None);
                                                let cur = editing_chip.get_untracked();
                                                if cur == Some((di, vi)) {
                                                    // toggle closed → save
                                                    commit_chip(editing_chip, val_ref, color_ref, store);
                                                } else {
                                                    if cur.is_some() {
                                                        commit_chip(editing_chip, val_ref, color_ref, store);
                                                    }
                                                    editing_chip.set(Some((di, vi)));
                                                }
                                            }
                                        >
                                            <span class="val-swatch" style=format!("background:{swatch}") />
                                            <span class="val-label">
                                                {if val.value.is_empty() { "（空）".into() } else { val.value.clone() }}
                                            </span>
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}

                                <button
                                    class="val-add-chip"
                                    on:click=move |_| {
                                        editing_dim.set(None);
                                        if editing_chip.get_untracked().is_some() {
                                            commit_chip(editing_chip, val_ref, color_ref, store);
                                        }
                                        let new_vi = {
                                            let mut vi = 0;
                                            store.update(|s| {
                                                if let Some(d) = s.dimensions.get_mut(di) {
                                                    vi = d.values.len();
                                                    d.values.push(DimensionValue {
                                                        value: String::new(),
                                                        color: None,
                                                    });
                                                }
                                            });
                                            vi
                                        };
                                        editing_chip.set(Some((di, new_vi)));
                                    }
                                >
                                    "+"
                                </button>
                            </div>

                            // ── Chip editor (shown below chips) ───────────
                            {move || {
                                if editing_chip.get().map(|(d, _)| d) != Some(di) {
                                    return None;
                                }
                                Some(view! {
                                    <div class="chip-editor">
                                        <input
                                            node_ref=val_ref
                                            class="chip-editor-val"
                                            type="text"
                                            placeholder="値"
                                        />
                                        <input
                                            node_ref=color_ref
                                            class="chip-editor-color"
                                            type="color"
                                        />
                                        <button
                                            class="chip-editor-delete"
                                            on:click=move |_| remove_chip(editing_chip, store)
                                        >
                                            "削除"
                                        </button>
                                        <button
                                            class="chip-editor-done"
                                            on:click=move |_| commit_chip(editing_chip, val_ref, color_ref, store)
                                        >
                                            "完了"
                                        </button>
                                    </div>
                                })
                            }}
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}

            <button
                class="dim-add-btn"
                on:click=move |_| {
                    editing_chip.set(None);
                    commit_dim(editing_dim, label_ref, id_ref, store);
                    let new_di = {
                        let mut di = 0;
                        store.update(|s| {
                            di = s.dimensions.len();
                            s.dimensions.push(Dimension {
                                id: new_dim_id(),
                                label: String::new(),
                                values: vec![],
                            });
                        });
                        di
                    };
                    save_to_storage(&store.get_untracked());
                    editing_dim.set(Some(new_di));
                }
            >
                "+ ディメンションを追加"
            </button>
        </div>
    }
}
