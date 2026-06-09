use crate::model::{AppStore, Dimension, DimensionValue};
use crate::storage::save_to_storage;
use icondata as icon;
use leptos::html::Input;
use leptos::*;
use leptos_icons::Icon;
use std::rc::Rc;
use wasm_bindgen::JsCast;

fn new_dim_id() -> String {
    let n = (js_sys::Math::random() * 1e15) as u64;
    format!("dim{n:x}")
}

fn random_nice_color() -> String {
    const PALETTE: &[&str] = &[
        "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
        "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
    ];
    let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
    PALETTE[idx.min(PALETTE.len() - 1)].to_string()
}

// Returns true when focus moved outside `selector` (or to null).
fn focus_left(ev: &web_sys::FocusEvent, selector: &str) -> bool {
    !ev.related_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok())
        .flatten()
        .is_some()
}

// Returns true when focus moved to an element matching `target_sel`.
fn focus_going_to(ev: &web_sys::FocusEvent, target_sel: &str) -> bool {
    ev.related_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(target_sel).ok())
        .flatten()
        .is_some()
}

fn commit_chip(
    editing_chip: RwSignal<Option<(usize, usize)>>,
    val_ref: NodeRef<Input>,
    color_ref: NodeRef<Input>,
    store: RwSignal<AppStore>,
) {
    let Some((di, vi)) = editing_chip.get_untracked() else {
        return;
    };
    let new_val = val_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    let new_color = color_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            if let Some(v) = dim.values.get_mut(vi) {
                v.value = new_val;
                v.color = if new_color.is_empty() {
                    None
                } else {
                    Some(new_color)
                };
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
    let Some(di) = editing_dim.get_untracked() else {
        return;
    };
    let new_label = label_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    let new_id = id_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
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

fn ask_confirm(
    msg: &'static str,
    action: impl Fn() + 'static,
    confirm_msg: RwSignal<Option<&'static str>>,
    confirm_action: RwSignal<Option<Rc<dyn Fn()>>>,
) {
    confirm_msg.set(Some(msg));
    confirm_action.set(Some(Rc::new(action)));
}

#[component]
pub fn DimensionsTab(store: RwSignal<AppStore>) -> impl IntoView {
    let editing_chip = create_rw_signal(Option::<(usize, usize)>::None);
    let editing_dim = create_rw_signal(Option::<usize>::None);

    let val_ref = create_node_ref::<Input>();
    let color_ref = create_node_ref::<Input>();
    let label_ref = create_node_ref::<Input>();
    let id_ref = create_node_ref::<Input>();

    let preview_color = create_rw_signal(Option::<String>::None);

    let confirm_msg = create_rw_signal(Option::<&'static str>::None);
    let confirm_action: RwSignal<Option<Rc<dyn Fn()>>> = create_rw_signal(None);

    let close_confirm = move || {
        confirm_msg.set(None);
        confirm_action.set(None);
    };

    create_effect(move |_| {
        let Some((di, vi)) = editing_chip.get() else {
            preview_color.set(None);
            return;
        };
        let s = store.get_untracked();
        let Some(dim) = s.dimensions.get(di) else {
            return;
        };
        let Some(val) = dim.values.get(vi) else {
            return;
        };
        preview_color.set(val.color.clone());
        if let Some(el) = val_ref.get() {
            el.set_value(&val.value);
        }
        if let Some(el) = color_ref.get() {
            el.set_value(val.color.as_deref().unwrap_or("#888888"));
        }
    });

    create_effect(move |_| {
        let Some(di) = editing_dim.get() else { return };
        let s = store.get_untracked();
        let Some(dim) = s.dimensions.get(di) else {
            return;
        };
        if let Some(el) = label_ref.get() {
            el.set_value(&dim.label);
        }
        if let Some(el) = id_ref.get() {
            el.set_value(&dim.id);
        }
    });

    view! {
        <div class="dim-tab">
            {move || {
                let s = store.get();
                s.dimensions.clone().into_iter().enumerate().map(|(di, dim)| {
                    // dim_id_del is captured by the delete-button reactive closure below.
                    let dim_id_del = dim.id.clone();

                    view! {
                        <div class="dim-row" class:active=move || editing_dim.get() == Some(di) || editing_chip.get().map(|(d,_)| d) == Some(di)>

                            // ── Dimension header ──────────────────────────────
                            <div class="dim-row-header">
                                // Name button / editor
                                {move || {
                                    if editing_dim.get() == Some(di) {
                                        view! {
                                            <div class="dim-name-editor"
                                                on:focusout=move |ev: web_sys::FocusEvent| {
                                                    // Don't commit when focus moves to the cancel button.
                                                    if !focus_going_to(&ev, ".dim-row-cancel")
                                                        && focus_left(&ev, ".dim-name-editor")
                                                    {
                                                        commit_dim(editing_dim, label_ref, id_ref, store);
                                                    }
                                                }
                                            >
                                                <input node_ref=label_ref class="dim-input" type="text" placeholder="ラベル" />
                                                <span class="dim-name-sep">"/"</span>
                                                <input node_ref=id_ref class="dim-input dim-input-id" type="text" placeholder="id" />
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
                                                    {if dim.label.is_empty() {
                                                        "（ラベルなし）".into()
                                                    } else {
                                                        dim.label.clone()
                                                    }}
                                                </span>
                                                <span class="dim-id-text">{dim.id.clone()}</span>
                                            </button>
                                        }.into_view()
                                    }
                                }}

                                // × delete  ↔  キャンセル (while editing name)
                                {move || {
                                    if editing_dim.get() == Some(di) {
                                        view! {
                                            <button
                                                class="dim-row-cancel"
                                                on:click=move |_| editing_dim.set(None)
                                            >
                                                "キャンセル"
                                            </button>
                                        }.into_view()
                                    } else {
                                        let dim_id = dim_id_del.clone();
                                        view! {
                                            <button
                                                class="dim-row-delete"
                                                on:click=move |_| {
                                                    let id = dim_id.clone();
                                                    ask_confirm(
                                                        "このディメンションを削除しますか？",
                                                        move || {
                                                            editing_chip.set(None);
                                                            editing_dim.set(None);
                                                            store.update(|s| s.dimensions.retain(|d| d.id != id));
                                                            save_to_storage(&store.get_untracked());
                                                        },
                                                        confirm_msg,
                                                        confirm_action,
                                                    );
                                                }
                                            >
                                                "×"
                                            </button>
                                        }.into_view()
                                    }
                                }}
                            </div>

                            // ── Value chips ───────────────────────────────────
                            <div class="dim-chips">
                                {dim.values.iter().enumerate().map(|(vi, val)| {
                                    let val_color = val.color.clone();
                                    view! {
                                        <button
                                            class="val-chip"
                                            class:editing=move || editing_chip.get() == Some((di, vi))
                                            style=move || {
                                                let color = if editing_chip.get() == Some((di, vi)) {
                                                    preview_color.get()
                                                        .or_else(|| val_color.clone())
                                                        .unwrap_or_default()
                                                } else {
                                                    val_color.clone().unwrap_or_default()
                                                };
                                                if color.is_empty() {
                                                    String::new()
                                                } else {
                                                    format!("border-color:{color};background:{color}1a")
                                                }
                                            }
                                            on:click=move |_| {
                                                editing_dim.set(None);
                                                let cur = editing_chip.get_untracked();
                                                if cur == Some((di, vi)) {
                                                    commit_chip(editing_chip, val_ref, color_ref, store);
                                                } else {
                                                    if cur.is_some() {
                                                        commit_chip(editing_chip, val_ref, color_ref, store);
                                                    }
                                                    editing_chip.set(Some((di, vi)));
                                                }
                                            }
                                        >
                                            <span class="val-label">
                                                {if val.value.is_empty() { "（空）".into() } else { val.value.clone() }}
                                            </span>
                                            // × inside the chip — stop_propagation prevents opening editor
                                            <span
                                                class="val-chip-delete"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    ask_confirm(
                                                        "この値を削除しますか？",
                                                        move || {
                                                            store.update(|s| {
                                                                if let Some(dim) = s.dimensions.get_mut(di) {
                                                                    if vi < dim.values.len() { dim.values.remove(vi); }
                                                                }
                                                            });
                                                            save_to_storage(&store.get_untracked());
                                                            editing_chip.set(None);
                                                        },
                                                        confirm_msg,
                                                        confirm_action,
                                                    );
                                                }
                                            >
                                                "×"
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
                                                        parent: None,
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

                            // ── Chip editor ───────────────────────────────────
                            {move || {
                                if editing_chip.get().map(|(d, _)| d) != Some(di) {
                                    return None;
                                }
                                Some(view! {
                                    <div
                                        class="chip-editor"
                                        on:focusout=move |ev: web_sys::FocusEvent| {
                                            if focus_left(&ev, ".chip-editor") {
                                                commit_chip(editing_chip, val_ref, color_ref, store);
                                            }
                                        }
                                    >
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
                                            on:input=move |ev: web_sys::Event| {
                                                let el: web_sys::HtmlInputElement =
                                                    ev.target().unwrap().dyn_into().unwrap();
                                                preview_color.set(Some(el.value()));
                                            }
                                        />
                                        <button
                                            class="chip-editor-randomize"
                                            on:click=move |_| {
                                                let color = random_nice_color();
                                                if let Some(el) = color_ref.get_untracked() {
                                                    el.set_value(&color);
                                                }
                                                preview_color.set(Some(color));
                                            }
                                        >
                                            <Icon icon=icon::HiArrowPathOutlineLg width="14" height="14" />
                                        </button>
                                        // Cancel is inside .chip-editor so focusout won't commit.
                                        <button
                                            class="chip-editor-cancel"
                                            on:click=move |_| editing_chip.set(None)
                                        >
                                            "キャンセル"
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

        // ── 確認ダイアログ ────────────────────────────────────────
        {move || confirm_msg.get().map(|msg| view! {
            <div class="confirm-overlay" on:click=move |_| close_confirm()>
                <div class="confirm-dialog" on:click=|ev| ev.stop_propagation()>
                    <p class="confirm-text">{msg}</p>
                    <div class="confirm-btns">
                        <button class="confirm-cancel" on:click=move |_| close_confirm()>
                            "キャンセル"
                        </button>
                        <button
                            class="confirm-ok"
                            on:click=move |_| {
                                if let Some(action) = confirm_action.get_untracked() {
                                    action();
                                }
                                close_confirm();
                            }
                        >
                            "削除"
                        </button>
                    </div>
                </div>
            </div>
        })}
    }
}
