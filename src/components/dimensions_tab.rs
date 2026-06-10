use crate::model::{AppStore, Dimension, DimensionValue};
use crate::storage::save_to_storage;
use icondata as icon;
use leptos::html::{Div, Input};
use leptos::*;
use leptos_icons::Icon;
use std::collections::HashSet;
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
    ev.related_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok())
        .flatten()
        .is_none()
}

// Returns true when focus moved to an element matching `target_sel`.
fn focus_going_to(ev: &web_sys::FocusEvent, target_sel: &str) -> bool {
    ev.related_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(target_sel).ok())
        .flatten()
        .is_some()
}

fn collapse_key(di: usize, value: &str) -> String {
    format!("{di}\u{0}{value}")
}

// 森を定義順にDFSして (index, depth, has_children) を返す。折りたたみは子孫をスキップ。
fn dfs_indices(
    values: &[DimensionValue],
    di: usize,
    parent: Option<&str>,
    depth: usize,
    collapsed: &HashSet<String>,
    out: &mut Vec<(usize, usize, bool)>,
) {
    for (i, v) in values.iter().enumerate() {
        if v.parent.as_deref() == parent {
            let has_children = values
                .iter()
                .any(|c| c.parent.as_deref() == Some(v.value.as_str()));
            out.push((i, depth, has_children));
            if has_children && !collapsed.contains(&collapse_key(di, &v.value)) {
                dfs_indices(values, di, Some(&v.value), depth + 1, collapsed, out);
            }
        }
    }
}

// start の祖先チェーン（自身含む）に target が含まれるか。
fn ancestry_contains(values: &[DimensionValue], start: &str, target: &str) -> bool {
    let mut cur = Some(start.to_string());
    let mut seen = HashSet::new();
    while let Some(c) = cur {
        if c == target {
            return true;
        }
        if !seen.insert(c.clone()) {
            break;
        }
        cur = values
            .iter()
            .find(|v| v.value == c)
            .and_then(|v| v.parent.clone());
    }
    false
}

// root とその全子孫の value を out に集める（root 自身を含む）。
fn collect_descendants(values: &[DimensionValue], root: &str, out: &mut HashSet<String>) {
    if !out.insert(root.to_string()) {
        return;
    }
    for v in values {
        if v.parent.as_deref() == Some(root) {
            collect_descendants(values, &v.value, out);
        }
    }
}

// ドラッグした値の親を付け替える（循環は防止）。
fn reparent(store: RwSignal<AppStore>, di: usize, dragged: String, new_parent: Option<String>) {
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            if let Some(np) = &new_parent {
                // 自分自身・自分の子孫の下には付け替えできない
                if np == &dragged || ancestry_contains(&dim.values, np, &dragged) {
                    return;
                }
            }
            if let Some(v) = dim.values.iter_mut().find(|v| v.value == dragged) {
                v.parent = new_parent;
            }
        }
    });
    save_to_storage(&store.get_untracked());
}

// 根ドロップゾーンを表す番兵（実値と衝突しないよう制御文字を含める）。
const ROOT_SENTINEL: &str = "\u{0}root";

// dragged を target の兄弟として、前(after=false)/後(after=true)に挿入し、
// 親を target の親に合わせる（並び替え）。循環は防止。
fn move_relative(
    store: RwSignal<AppStore>,
    di: usize,
    dragged: String,
    target: String,
    after: bool,
) {
    if dragged == target {
        return;
    }
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            let new_parent = dim
                .values
                .iter()
                .find(|v| v.value == target)
                .and_then(|v| v.parent.clone());
            if let Some(np) = &new_parent {
                if np == &dragged || ancestry_contains(&dim.values, np, &dragged) {
                    return;
                }
            }
            let Some(dpos) = dim.values.iter().position(|v| v.value == dragged) else {
                return;
            };
            let mut node = dim.values.remove(dpos);
            node.parent = new_parent;
            let tpos = dim
                .values
                .iter()
                .position(|v| v.value == target)
                .unwrap_or(dim.values.len());
            let insert_at = if after { tpos + 1 } else { tpos };
            dim.values.insert(insert_at.min(dim.values.len()), node);
        }
    });
    save_to_storage(&store.get_untracked());
}

// 行の上端・高さ・ポインタY からドロップ位置を判定: 0=前, 1=中(子にする), 2=後。
fn zone_from(top: f64, height: f64, client_y: i32) -> u8 {
    if height > 0.0 {
        let rel = (client_y as f64 - top) / height;
        if rel < 0.3 {
            return 0;
        }
        if rel > 0.7 {
            return 2;
        }
    }
    1
}

// 行要素（NodeRef）とイベントからゾーンを求める。
fn zone_of(row_ref: NodeRef<Div>, ev: &web_sys::DragEvent) -> u8 {
    row_ref
        .get_untracked()
        .map(|el| {
            let r = el.get_bounding_client_rect();
            zone_from(r.top(), r.height(), ev.client_y())
        })
        .unwrap_or(1)
}

// 子を親（削除ノードの親）に繰り上げてからノードを削除。
fn delete_node_promote(store: RwSignal<AppStore>, di: usize, value: String) {
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            let parent = dim
                .values
                .iter()
                .find(|v| v.value == value)
                .and_then(|v| v.parent.clone());
            for c in dim.values.iter_mut() {
                if c.parent.as_deref() == Some(value.as_str()) {
                    c.parent = parent.clone();
                }
            }
            dim.values.retain(|v| v.value != value);
        }
    });
    save_to_storage(&store.get_untracked());
}

// サブツリー（ノード＋全子孫）を削除。
fn delete_node_subtree(store: RwSignal<AppStore>, di: usize, value: String) {
    store.update(|s| {
        if let Some(dim) = s.dimensions.get_mut(di) {
            let mut doomed = HashSet::new();
            collect_descendants(&dim.values, &value, &mut doomed);
            dim.values.retain(|v| !doomed.contains(&v.value));
        }
    });
    save_to_storage(&store.get_untracked());
}

fn commit_node(
    editing_node: RwSignal<Option<(usize, usize)>>,
    val_ref: NodeRef<Input>,
    color_ref: NodeRef<Input>,
    store: RwSignal<AppStore>,
) {
    let Some((di, vi)) = editing_node.get_untracked() else {
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
    editing_node.set(None);
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
    let editing_node = create_rw_signal(Option::<(usize, usize)>::None);
    let editing_dim = create_rw_signal(Option::<usize>::None);

    let val_ref = create_node_ref::<Input>();
    let color_ref = create_node_ref::<Input>();
    let label_ref = create_node_ref::<Input>();
    let id_ref = create_node_ref::<Input>();

    let preview_color = create_rw_signal(Option::<String>::None);
    let collapsed = create_rw_signal(HashSet::<String>::new());

    // ドラッグ中の (dim index, value) と、ドロップ先ハイライト用。
    let dragging = create_rw_signal(Option::<(usize, String)>::None);
    // (dim index, value or ROOT_SENTINEL, zone: 0=前 1=中 2=後)
    let drag_over = create_rw_signal(Option::<(usize, String, u8)>::None);

    // ディメンション削除用の汎用確認ダイアログ。
    let confirm_msg = create_rw_signal(Option::<&'static str>::None);
    let confirm_action: RwSignal<Option<Rc<dyn Fn()>>> = create_rw_signal(None);
    let close_confirm = move || {
        confirm_msg.set(None);
        confirm_action.set(None);
    };

    // ノード削除ダイアログ: (dim index, value, has_children)
    let delete_target = create_rw_signal(Option::<(usize, String, bool)>::None);

    create_effect(move |_| {
        let Some((di, vi)) = editing_node.get() else {
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
                    let dim_id_del = dim.id.clone();
                    let collapsed_set = collapsed.get();

                    let mut order = Vec::new();
                    dfs_indices(&dim.values, di, None, 0, &collapsed_set, &mut order);

                    view! {
                        <div
                            class="dim-row"
                            class:active=move || {
                                editing_dim.get() == Some(di)
                                    || editing_node.get().map(|(d, _)| d) == Some(di)
                            }
                        >
                            // ── Dimension header ──────────────────────────────
                            <div class="dim-row-header">
                                {move || {
                                    if editing_dim.get() == Some(di) {
                                        view! {
                                            <div class="dim-name-editor"
                                                on:focusout=move |ev: web_sys::FocusEvent| {
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
                                                    commit_node(editing_node, val_ref, color_ref, store);
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

                                // ドラッグ中だけ、タイトル横に「根へ」ドロップ枠を出す
                                {move || {
                                    if matches!(dragging.get(), Some((d, _)) if d == di) {
                                        view! {
                                            <div
                                                class="dim-root-drop"
                                                class:over=move || {
                                                    drag_over.get()
                                                        == Some((di, ROOT_SENTINEL.to_string(), 1))
                                                }
                                                on:dragover=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some(dt) = ev.data_transfer() {
                                                        dt.set_drop_effect("move");
                                                    }
                                                    drag_over.set(Some((di, ROOT_SENTINEL.to_string(), 1)));
                                                }
                                                on:drop=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some((ddi, dval)) = dragging.get_untracked() {
                                                        if ddi == di { reparent(store, di, dval, None); }
                                                    }
                                                    dragging.set(None);
                                                    drag_over.set(None);
                                                }
                                            >
                                                "⬚ 根へ"
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! { <span /> }.into_view()
                                    }
                                }}

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
                                                            editing_node.set(None);
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

                            // ── ツリー本体 ─────────────────────────────────────
                            <div class="dim-tree">
                                {order.into_iter().map(|(vi, depth, has_children)| {
                                    let row_ref = create_node_ref::<Div>();
                                    let val = dim.values[vi].clone();
                                    let value = val.value.clone();
                                    let val_color = val.color.clone();
                                    let indent = format!("padding-left:{}rem", 0.1 + depth as f32 * 1.0);

                                    // caret
                                    let caret = if has_children {
                                        let v = value.clone();
                                        let is_collapsed = collapsed_set.contains(&collapse_key(di, &value));
                                        view! {
                                            <button class="dim-caret"
                                                on:click=move |_| {
                                                    let key = collapse_key(di, &v);
                                                    collapsed.update(|c| { if !c.remove(&key) { c.insert(key.clone()); } });
                                                }
                                            >{if is_collapsed { "▶" } else { "▼" }}</button>
                                        }.into_view()
                                    } else {
                                        view! { <span class="dim-caret-spacer" /> }.into_view()
                                    };

                                    // ドラッグ元の値（dragstart 用）
                                    let v_drag = value.clone();
                                    // ドロップ先の値（drop 用）
                                    let v_drop = value.clone();
                                    let v_over2 = value.clone();
                                    let v_in = value.clone();
                                    let v_bf = value.clone();
                                    let v_af = value.clone();

                                    let row_body = if editing_node.get() == Some((di, vi)) {
                                        view! {
                                            <div class="chip-editor dim-node-editor"
                                                on:focusout=move |ev: web_sys::FocusEvent| {
                                                    if focus_left(&ev, ".chip-editor") {
                                                        commit_node(editing_node, val_ref, color_ref, store);
                                                    }
                                                }
                                            >
                                                <input node_ref=val_ref class="chip-editor-val" type="text" placeholder="値" />
                                                <input node_ref=color_ref class="chip-editor-color" type="color"
                                                    on:input=move |ev: web_sys::Event| {
                                                        let el: web_sys::HtmlInputElement = ev.target().unwrap().dyn_into().unwrap();
                                                        preview_color.set(Some(el.value()));
                                                    }
                                                />
                                                <button class="chip-editor-randomize"
                                                    on:click=move |_| {
                                                        let color = random_nice_color();
                                                        if let Some(el) = color_ref.get_untracked() { el.set_value(&color); }
                                                        preview_color.set(Some(color));
                                                    }
                                                >
                                                    <Icon icon=icon::HiArrowPathOutlineLg width="14" height="14" />
                                                </button>
                                                <button class="chip-editor-cancel" on:click=move |_| editing_node.set(None)>
                                                    "キャンセル"
                                                </button>
                                            </div>
                                        }.into_view()
                                    } else {
                                        let parent_value = value.clone();
                                        let del_value = value.clone();
                                        view! {
                                            <button
                                                class="dim-node-label"
                                                style=move || {
                                                    let color = if editing_node.get() == Some((di, vi)) {
                                                        preview_color.get().or_else(|| val_color.clone())
                                                    } else {
                                                        val_color.clone()
                                                    };
                                                    match color {
                                                        Some(c) if !c.is_empty() =>
                                                            format!("border-color:{c};background:{c}1a"),
                                                        _ => String::new(),
                                                    }
                                                }
                                                on:click=move |_| {
                                                    editing_dim.set(None);
                                                    let cur = editing_node.get_untracked();
                                                    if cur != Some((di, vi)) && cur.is_some() {
                                                        commit_node(editing_node, val_ref, color_ref, store);
                                                    }
                                                    editing_node.set(Some((di, vi)));
                                                }
                                            >
                                                {if value.is_empty() { "（空）".to_string() } else { value.clone() }}
                                            </button>
                                            <button
                                                class="dim-node-add-child"
                                                title="子を追加"
                                                on:click=move |_| {
                                                    if editing_node.get_untracked().is_some() {
                                                        commit_node(editing_node, val_ref, color_ref, store);
                                                    }
                                                    let parent = parent_value.clone();
                                                    let new_vi = {
                                                        let mut vi = 0;
                                                        store.update(|s| {
                                                            if let Some(d) = s.dimensions.get_mut(di) {
                                                                vi = d.values.len();
                                                                d.values.push(DimensionValue {
                                                                    value: String::new(),
                                                                    color: None,
                                                                    parent: Some(parent.clone()),
                                                                });
                                                            }
                                                        });
                                                        vi
                                                    };
                                                    // 親が折りたたまれていたら開く
                                                    collapsed.update(|c| { c.remove(&collapse_key(di, &parent_value)); });
                                                    editing_node.set(Some((di, new_vi)));
                                                }
                                            >
                                                "＋"
                                            </button>
                                            <button
                                                class="dim-node-delete"
                                                title="削除"
                                                on:click=move |_| {
                                                    delete_target.set(Some((di, del_value.clone(), has_children)));
                                                }
                                            >
                                                "×"
                                            </button>
                                        }.into_view()
                                    };

                                    view! {
                                        <div
                                            node_ref=row_ref
                                            class="dim-node-row"
                                            class:over-inside=move || drag_over.get() == Some((di, v_in.clone(), 1))
                                            class:over-before=move || drag_over.get() == Some((di, v_bf.clone(), 0))
                                            class:over-after=move || drag_over.get() == Some((di, v_af.clone(), 2))
                                            style=indent
                                            on:dragover=move |ev: web_sys::DragEvent| {
                                                ev.prevent_default();
                                                if let Some(dt) = ev.data_transfer() {
                                                    dt.set_drop_effect("move");
                                                }
                                                let zone = zone_of(row_ref, &ev);
                                                drag_over.set(Some((di, v_over2.clone(), zone)));
                                            }
                                            on:drop=move |ev: web_sys::DragEvent| {
                                                ev.prevent_default();
                                                let zone = zone_of(row_ref, &ev);
                                                if let Some((ddi, dval)) = dragging.get_untracked() {
                                                    if ddi == di {
                                                        match zone {
                                                            0 => move_relative(store, di, dval, v_drop.clone(), false),
                                                            2 => move_relative(store, di, dval, v_drop.clone(), true),
                                                            _ => reparent(store, di, dval, Some(v_drop.clone())),
                                                        }
                                                    }
                                                }
                                                dragging.set(None);
                                                drag_over.set(None);
                                            }
                                        >
                                            {caret}
                                            <span
                                                class="dim-drag-handle"
                                                draggable="true"
                                                title="ドラッグで親を付け替え"
                                                on:dragstart=move |ev: web_sys::DragEvent| {
                                                    dragging.set(Some((di, v_drag.clone())));
                                                    if let Some(dt) = ev.data_transfer() {
                                                        let _ = dt.set_data("text/plain", &v_drag);
                                                        dt.set_effect_allowed("move");
                                                    }
                                                }
                                                on:dragend=move |_| {
                                                    dragging.set(None);
                                                    drag_over.set(None);
                                                }
                                            >
                                                "⠿"
                                            </span>
                                            {row_body}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}

                                <button
                                    class="dim-add-root"
                                    on:click=move |_| {
                                        if editing_node.get_untracked().is_some() {
                                            commit_node(editing_node, val_ref, color_ref, store);
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
                                        editing_node.set(Some((di, new_vi)));
                                    }
                                >
                                    "+ 根を追加"
                                </button>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}

            <button
                class="dim-add-btn"
                on:click=move |_| {
                    editing_node.set(None);
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

        // ── ディメンション削除の確認ダイアログ ────────────────────
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
                                if let Some(action) = confirm_action.get_untracked() { action(); }
                                close_confirm();
                            }
                        >
                            "削除"
                        </button>
                    </div>
                </div>
            </div>
        })}

        // ── ノード削除ダイアログ（子があれば選択式）────────────────
        {move || delete_target.get().map(|(di, value, has_children)| {
            let label = if value.is_empty() { "（空）".to_string() } else { value.clone() };
            let v_promote = value.clone();
            let v_subtree = value.clone();
            let v_simple = value.clone();
            view! {
                <div class="confirm-overlay" on:click=move |_| delete_target.set(None)>
                    <div class="confirm-dialog" on:click=|ev| ev.stop_propagation()>
                        <p class="confirm-text">{format!("「{label}」を削除します")}</p>
                        <div class="confirm-btns">
                            <button class="confirm-cancel" on:click=move |_| delete_target.set(None)>
                                "キャンセル"
                            </button>
                            {if has_children {
                                view! {
                                    <button
                                        class="confirm-ok"
                                        on:click=move |_| {
                                            editing_node.set(None);
                                            delete_node_promote(store, di, v_promote.clone());
                                            delete_target.set(None);
                                        }
                                    >
                                        "子を繰り上げ"
                                    </button>
                                    <button
                                        class="confirm-ok confirm-danger"
                                        on:click=move |_| {
                                            editing_node.set(None);
                                            delete_node_subtree(store, di, v_subtree.clone());
                                            delete_target.set(None);
                                        }
                                    >
                                        "サブツリーごと"
                                    </button>
                                }.into_view()
                            } else {
                                view! {
                                    <button
                                        class="confirm-ok"
                                        on:click=move |_| {
                                            editing_node.set(None);
                                            delete_node_subtree(store, di, v_simple.clone());
                                            delete_target.set(None);
                                        }
                                    >
                                        "削除"
                                    </button>
                                }.into_view()
                            }}
                        </div>
                    </div>
                </div>
            }
        })}
    }
}
