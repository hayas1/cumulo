use crate::model::{AppStore, DimensionNode};
use crate::storage::save_to_storage;
use icondata as icon;
use leptos::html::{Div, Input};
use leptos::*;
use leptos_icons::Icon;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::JsCast;

fn new_node_id() -> String {
    let n = (js_sys::Math::random() * 1e15) as u64;
    format!("node{n:x}")
}

fn random_nice_color() -> String {
    const PALETTE: &[&str] = &[
        "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
        "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
    ];
    let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
    PALETTE[idx.min(PALETTE.len() - 1)].to_string()
}

fn focus_left(ev: &web_sys::FocusEvent, selector: &str) -> bool {
    ev.related_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest(selector).ok())
        .flatten()
        .is_none()
}

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

fn zone_of(row_ref: NodeRef<Div>, ev: &web_sys::DragEvent) -> u8 {
    row_ref
        .get_untracked()
        .map(|el| {
            let r = el.get_bounding_client_rect();
            zone_from(r.top(), r.height(), ev.client_y())
        })
        .unwrap_or(1)
}

fn root_sentinel(root_id: &str) -> String {
    format!("\x00root:{}", root_id)
}


fn reparent_flat(store: RwSignal<AppStore>, dragged: String, new_parent: Option<String>) {
    store.update(|s| {
        if let Some(np) = &new_parent {
            if np == &dragged || s.dimensions.ancestry_contains(np, &dragged) {
                return;
            }
        }
        if let Some(n) = s.dimensions.iter_mut().find(|n| n.id == dragged) {
            n.parent = new_parent;
        }
    });
    save_to_storage(&store.get_untracked());
}

fn move_relative_flat(store: RwSignal<AppStore>, dragged: String, target: String, after: bool) {
    if dragged == target {
        return;
    }
    store.update(|s| {
        let new_parent = s
            .dimensions
            .iter()
            .find(|n| n.id == target)
            .and_then(|n| n.parent.clone());
        if let Some(np) = &new_parent {
            if s.dimensions.ancestry_contains(np, &dragged) {
                return;
            }
        }
        let Some(dpos) = s.dimensions.iter().position(|n| n.id == dragged) else {
            return;
        };
        let mut node = s.dimensions.remove(dpos);
        node.parent = new_parent;
        let tpos = s
            .dimensions
            .iter()
            .position(|n| n.id == target)
            .unwrap_or(s.dimensions.len());
        let insert_at = if after { tpos + 1 } else { tpos };
        let len = s.dimensions.len();
        s.dimensions.insert(insert_at.min(len), node);
    });
    save_to_storage(&store.get_untracked());
}

fn delete_promote_flat(store: RwSignal<AppStore>, node_id: String) {
    store.update(|s| {
        let parent = s
            .dimensions
            .iter()
            .find(|n| n.id == node_id)
            .and_then(|n| n.parent.clone());
        for child in s.dimensions.iter_mut() {
            if child.parent.as_deref() == Some(node_id.as_str()) {
                child.parent = parent.clone();
            }
        }
        s.dimensions.retain(|n| n.id != node_id);
    });
    save_to_storage(&store.get_untracked());
}

fn delete_subtree_flat(store: RwSignal<AppStore>, node_id: String) {
    store.update(|s| {
        let doomed = s.dimensions.collect_descendants(&node_id);
        s.dimensions.retain(|n| !doomed.contains(&n.id));
    });
    save_to_storage(&store.get_untracked());
}


fn commit_node_edit(
    editing_id: RwSignal<Option<String>>,
    id_ref: NodeRef<Input>,
    label_ref: NodeRef<Input>,
    color_ref: NodeRef<Input>,
    store: RwSignal<AppStore>,
) {
    let Some(old_id) = editing_id.get_untracked() else {
        return;
    };
    let new_id = id_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    let new_label = label_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    let new_color = color_ref
        .get_untracked()
        .map(|el| el.value())
        .unwrap_or_default();
    if new_id.trim().is_empty() {
        return;
    }
    store.update(|s| {
        if old_id != new_id {
            for other in s.dimensions.iter_mut() {
                if other.parent.as_deref() == Some(old_id.as_str()) {
                    other.parent = Some(new_id.clone());
                }
            }
        }
        if let Some(n) = s.dimensions.iter_mut().find(|n| n.id == old_id) {
            n.id = new_id;
            n.label = new_label;
            n.color = new_color;
        }
    });
    save_to_storage(&store.get_untracked());
    editing_id.set(None);
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
    let editing_id = create_rw_signal(Option::<String>::None);
    let id_ref = create_node_ref::<Input>();
    let label_ref = create_node_ref::<Input>();
    let color_ref = create_node_ref::<Input>();
    let preview_color = create_rw_signal(String::new());

    let collapsed = create_rw_signal(HashSet::<String>::new());
    let dragging = create_rw_signal(Option::<String>::None);
    let drag_over = create_rw_signal(Option::<(String, u8)>::None);

    let confirm_msg = create_rw_signal(Option::<&'static str>::None);
    let confirm_action: RwSignal<Option<Rc<dyn Fn()>>> = create_rw_signal(None);
    let close_confirm = move || {
        confirm_msg.set(None);
        confirm_action.set(None);
    };

    let delete_target = create_rw_signal(Option::<(String, bool)>::None);

    create_effect(move |_| {
        let Some(eid) = editing_id.get() else {
            preview_color.set(String::new());
            return;
        };
        let s = store.get_untracked();
        let Some(n) = s.dimensions.iter().find(|n| n.id == eid) else {
            return;
        };
        preview_color.set(n.color.clone());
        if let Some(el) = id_ref.get() {
            el.set_value(&n.id);
        }
        if let Some(el) = label_ref.get() {
            el.set_value(&n.label);
        }
        if let Some(el) = color_ref.get() {
            el.set_value(&n.color);
        }
    });

    view! {
        <div class="dim-tab">
            {move || {
                let s = store.get();
                let collapsed_set = collapsed.get();
                let current_editing = editing_id.get();
                let is_dragging = dragging.get().is_some();

                let root_nodes: Vec<DimensionNode> = s
                    .dimensions
                    .iter()
                    .filter(|n| n.parent.is_none())
                    .cloned()
                    .collect();

                root_nodes
                    .into_iter()
                    .map(|root| {
                        let root_id_active = root.id.clone();
                        let root_id_header = root.id.clone();
                        let root_id_drop = root.id.clone();
                        let root_id_del = root.id.clone();
                        let root_id_add = root.id.clone();

                        let order = s.dimensions.dfs_order(&root.id, &collapsed_set);
                        let sentinel = root_sentinel(&root.id);

                        let is_root_editing = current_editing.as_deref() == Some(root.id.as_str());

                        view! {
                            <div
                                class="dim-row"
                                class:active=move || {
                                    if let Some(ref eid) = editing_id.get() {
                                        *eid == root_id_active
                                            || store.with(|s| {
                                                s.dimensions.root_of(eid)
                                                    .map(|r| r == root_id_active)
                                                    .unwrap_or(false)
                                            })
                                    } else {
                                        false
                                    }
                                }
                            >
                                // ── Root header ──────────────────────────────────
                                <div class="dim-row-header">
                                    {if is_root_editing {
                                        let rid_cancel = root_id_header.clone();
                                        view! {
                                            <div
                                                class="dim-name-editor"
                                                on:focusout=move |ev: web_sys::FocusEvent| {
                                                    if focus_left(&ev, ".dim-name-editor") {
                                                        commit_node_edit(
                                                            editing_id,
                                                            id_ref,
                                                            label_ref,
                                                            color_ref,
                                                            store,
                                                        );
                                                    }
                                                }
                                            >
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
                                                <input
                                                    node_ref=color_ref
                                                    class="chip-editor-color"
                                                    type="color"
                                                    on:input=move |ev: web_sys::Event| {
                                                        let el: web_sys::HtmlInputElement =
                                                            ev.target().unwrap().dyn_into().unwrap();
                                                        preview_color.set(el.value());
                                                    }
                                                />
                                                <button
                                                    class="dim-row-cancel"
                                                    on:click=move |_| {
                                                        editing_id.set(None);
                                                        // If was a new (empty) axis, delete it
                                                        store.update(|s| {
                                                            if let Some(n) = s
                                                                .dimensions
                                                                .iter()
                                                                .find(|n| n.id == rid_cancel)
                                                            {
                                                                if n.label.is_empty() && n.id.starts_with("node") {
                                                                    let id = n.id.clone();
                                                                    s.dimensions.retain(|n| n.id != id);
                                                                }
                                                            }
                                                        });
                                                        save_to_storage(&store.get_untracked());
                                                    }
                                                >
                                                    "キャンセル"
                                                </button>
                                            </div>
                                        }
                                        .into_view()
                                    } else {
                                        let rid_click = root_id_header.clone();
                                        view! {
                                            <button
                                                class="dim-name-btn"
                                                on:click=move |_| {
                                                    commit_node_edit(
                                                        editing_id,
                                                        id_ref,
                                                        label_ref,
                                                        color_ref,
                                                        store,
                                                    );
                                                    editing_id.set(Some(rid_click.clone()));
                                                }
                                            >
                                                <span class="dim-label-text">
                                                    {if root.label.is_empty() {
                                                        "（ラベルなし）".to_string()
                                                    } else {
                                                        root.label.clone()
                                                    }}
                                                </span>
                                                <span class="dim-id-text">{root.id.clone()}</span>
                                            </button>
                                        }
                                        .into_view()
                                    }}

                                    // Root drop zone (visible while dragging)
                                    {if is_dragging {
                                        let s_drag = sentinel.clone();
                                        let s_over = sentinel.clone();
                                        let rid_drop = root_id_drop.clone();
                                        view! {
                                            <div
                                                class="dim-root-drop"
                                                class:over=move || {
                                                    drag_over
                                                        .get()
                                                        .as_ref()
                                                        .map(|(id, _)| *id == s_over)
                                                        .unwrap_or(false)
                                                }
                                                on:dragover=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some(dt) = ev.data_transfer() {
                                                        dt.set_drop_effect("move");
                                                    }
                                                    drag_over.set(Some((s_drag.clone(), 1)));
                                                }
                                                on:drop=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some(dval) = dragging.get_untracked() {
                                                        reparent_flat(store, dval, Some(rid_drop.clone()));
                                                    }
                                                    dragging.set(None);
                                                    drag_over.set(None);
                                                }
                                            >
                                                "⬚ 直下へ"
                                            </div>
                                        }
                                        .into_view()
                                    } else {
                                        view! { <span /> }.into_view()
                                    }}

                                    {if !is_root_editing {
                                        let rid_d = root_id_del.clone();
                                        view! {
                                            <button
                                                class="dim-row-delete"
                                                on:click=move |_| {
                                                    let id = rid_d.clone();
                                                    editing_id.set(None);
                                                    ask_confirm(
                                                        "このディメンションを削除しますか？",
                                                        move || {
                                                            let id2 = id.clone();
                                                            let doomed = store.with_untracked(|s| {
                                                                s.dimensions.collect_descendants(&id2)
                                                            });
                                                            store.update(|s| {
                                                                s.dimensions
                                                                    .retain(|n| !doomed.contains(&n.id));
                                                            });
                                                            save_to_storage(&store.get_untracked());
                                                        },
                                                        confirm_msg,
                                                        confirm_action,
                                                    );
                                                }
                                            >
                                                "×"
                                            </button>
                                        }
                                        .into_view()
                                    } else {
                                        view! { <span /> }.into_view()
                                    }}
                                </div>

                                // ── Tree body ──────────────────────────────────────
                                <div class="dim-tree">
                                    {order
                                        .into_iter()
                                        .map(|(node_id, depth, has_children)| {
                                            let row_ref = create_node_ref::<Div>();
                                            let n = s
                                                .dimensions
                                                .iter()
                                                .find(|n| n.id == node_id)
                                                .cloned()
                                                .unwrap();
                                            let node_color = n.color.clone();
                                            let node_label_text = if n.label.is_empty() {
                                                n.id.clone()
                                            } else {
                                                n.label.clone()
                                            };
                                            let indent = format!(
                                                "padding-left:{}rem",
                                                0.1 + depth as f32 * 1.0
                                            );
                                            let is_node_editing = current_editing.as_deref()
                                                == Some(node_id.as_str());
                                            let is_collapsed =
                                                collapsed_set.contains(node_id.as_str());

                                            let caret = if has_children {
                                                let nid = node_id.clone();
                                                view! {
                                                    <button
                                                        class="dim-caret"
                                                        on:click=move |_| {
                                                            collapsed.update(|c| {
                                                                if !c.remove(&nid) {
                                                                    c.insert(nid.clone());
                                                                }
                                                            });
                                                        }
                                                    >
                                                        {if is_collapsed { "▶" } else { "▼" }}
                                                    </button>
                                                }
                                                .into_view()
                                            } else {
                                                view! { <span class="dim-caret-spacer" /> }
                                                    .into_view()
                                            };

                                            let v_drag = node_id.clone();
                                            let v_drop = node_id.clone();
                                            let v_over = node_id.clone();
                                            let v_in = node_id.clone();
                                            let v_bf = node_id.clone();
                                            let v_af = node_id.clone();

                                            let row_body = if is_node_editing {
                                                view! {
                                                    <div
                                                        class="chip-editor dim-node-editor"
                                                        on:focusout=move |ev: web_sys::FocusEvent| {
                                                            if focus_left(&ev, ".chip-editor") {
                                                                commit_node_edit(
                                                                    editing_id,
                                                                    id_ref,
                                                                    label_ref,
                                                                    color_ref,
                                                                    store,
                                                                );
                                                            }
                                                        }
                                                    >
                                                        <input
                                                            node_ref=label_ref
                                                            class="chip-editor-val"
                                                            type="text"
                                                            placeholder="ラベル"
                                                        />
                                                        <span class="dim-name-sep">"/"</span>
                                                        <input
                                                            node_ref=id_ref
                                                            class="chip-editor-val"
                                                            type="text"
                                                            placeholder="id"
                                                        />
                                                        <input
                                                            node_ref=color_ref
                                                            class="chip-editor-color"
                                                            type="color"
                                                            on:input=move |ev: web_sys::Event| {
                                                                let el: web_sys::HtmlInputElement =
                                                                    ev.target().unwrap().dyn_into().unwrap();
                                                                preview_color.set(el.value());
                                                            }
                                                        />
                                                        <button
                                                            class="chip-editor-randomize"
                                                            on:click=move |_| {
                                                                let color = random_nice_color();
                                                                if let Some(el) = color_ref.get_untracked()
                                                                {
                                                                    el.set_value(&color);
                                                                }
                                                                preview_color.set(color);
                                                            }
                                                        >
                                                            <Icon
                                                                icon=icon::HiArrowPathOutlineLg
                                                                width="14"
                                                                height="14"
                                                            />
                                                        </button>
                                                        <button
                                                            class="chip-editor-cancel"
                                                            on:click=move |_| editing_id.set(None)
                                                        >
                                                            "キャンセル"
                                                        </button>
                                                    </div>
                                                }
                                                .into_view()
                                            } else {
                                                let nid_style = node_id.clone();
                                                let nid_click = node_id.clone();
                                                let nid_add = node_id.clone();
                                                let nid_del = node_id.clone();
                                                view! {
                                                    <button
                                                        class="dim-node-label"
                                                        style=move || {
                                                            let color = if editing_id.get().as_deref()
                                                                == Some(nid_style.as_str())
                                                            {
                                                                let pc = preview_color.get();
                                                                if pc.is_empty() {
                                                                    node_color.clone()
                                                                } else {
                                                                    pc
                                                                }
                                                            } else {
                                                                node_color.clone()
                                                            };
                                                            if !color.is_empty() {
                                                                format!(
                                                                    "border-color:{color};background:{color}1a"
                                                                )
                                                            } else {
                                                                String::new()
                                                            }
                                                        }
                                                        on:click=move |_| {
                                                            let cur = editing_id.get_untracked();
                                                            if cur.as_deref() != Some(&nid_click)
                                                                && cur.is_some()
                                                            {
                                                                commit_node_edit(
                                                                    editing_id,
                                                                    id_ref,
                                                                    label_ref,
                                                                    color_ref,
                                                                    store,
                                                                );
                                                            }
                                                            editing_id.set(Some(nid_click.clone()));
                                                        }
                                                    >
                                                        {node_label_text}
                                                    </button>
                                                    <button
                                                        class="dim-node-add-child"
                                                        title="子を追加"
                                                        on:click=move |_| {
                                                            if editing_id.get_untracked().is_some() {
                                                                commit_node_edit(
                                                                    editing_id,
                                                                    id_ref,
                                                                    label_ref,
                                                                    color_ref,
                                                                    store,
                                                                );
                                                            }
                                                            let parent = nid_add.clone();
                                                            let new_id = new_node_id();
                                                            let new_id2 = new_id.clone();
                                                            store.update(|s| {
                                                                s.dimensions.push(DimensionNode {
                                                                    id: new_id.clone(),
                                                                    label: String::new(),
                                                                    color: random_nice_color(),
                                                                    parent: Some(parent.clone()),
                                                                });
                                                            });
                                                            save_to_storage(
                                                                &store.get_untracked(),
                                                            );
                                                            collapsed.update(|c| {
                                                                c.remove(&nid_add);
                                                            });
                                                            editing_id.set(Some(new_id2));
                                                        }
                                                    >
                                                        "＋"
                                                    </button>
                                                    <button
                                                        class="dim-node-delete"
                                                        title="削除"
                                                        on:click=move |_| {
                                                            delete_target
                                                                .set(Some((
                                                                    nid_del.clone(),
                                                                    has_children,
                                                                )));
                                                        }
                                                    >
                                                        "×"
                                                    </button>
                                                }
                                                .into_view()
                                            };

                                            view! {
                                                <div
                                                    node_ref=row_ref
                                                    class="dim-node-row"
                                                    class:over-inside=move || {
                                                        drag_over
                                                            .get()
                                                            .as_ref()
                                                            .map(|(id, z)| id == &v_in && *z == 1)
                                                            .unwrap_or(false)
                                                    }
                                                    class:over-before=move || {
                                                        drag_over
                                                            .get()
                                                            .as_ref()
                                                            .map(|(id, z)| id == &v_bf && *z == 0)
                                                            .unwrap_or(false)
                                                    }
                                                    class:over-after=move || {
                                                        drag_over
                                                            .get()
                                                            .as_ref()
                                                            .map(|(id, z)| id == &v_af && *z == 2)
                                                            .unwrap_or(false)
                                                    }
                                                    style=indent
                                                    on:dragover=move |ev: web_sys::DragEvent| {
                                                        ev.prevent_default();
                                                        if let Some(dt) = ev.data_transfer() {
                                                            dt.set_drop_effect("move");
                                                        }
                                                        let zone = zone_of(row_ref, &ev);
                                                        drag_over.set(Some((v_over.clone(), zone)));
                                                    }
                                                    on:drop=move |ev: web_sys::DragEvent| {
                                                        ev.prevent_default();
                                                        let zone = zone_of(row_ref, &ev);
                                                        if let Some(dval) = dragging.get_untracked()
                                                        {
                                                            match zone {
                                                                0 => move_relative_flat(
                                                                    store,
                                                                    dval,
                                                                    v_drop.clone(),
                                                                    false,
                                                                ),
                                                                2 => move_relative_flat(
                                                                    store,
                                                                    dval,
                                                                    v_drop.clone(),
                                                                    true,
                                                                ),
                                                                _ => reparent_flat(
                                                                    store,
                                                                    dval,
                                                                    Some(v_drop.clone()),
                                                                ),
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
                                                            dragging.set(Some(v_drag.clone()));
                                                            if let Some(dt) = ev.data_transfer() {
                                                                let _ = dt.set_data(
                                                                    "text/plain",
                                                                    &v_drag,
                                                                );
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
                                        })
                                        .collect::<Vec<_>>()}

                                    <button
                                        class="dim-add-root"
                                        on:click=move |_| {
                                            if editing_id.get_untracked().is_some() {
                                                commit_node_edit(
                                                    editing_id,
                                                    id_ref,
                                                    label_ref,
                                                    color_ref,
                                                    store,
                                                );
                                            }
                                            let new_id = new_node_id();
                                            let new_id2 = new_id.clone();
                                            store.update(|s| {
                                                s.dimensions.push(DimensionNode {
                                                    id: new_id.clone(),
                                                    label: String::new(),
                                                    color: random_nice_color(),
                                                    parent: Some(root_id_add.clone()),
                                                });
                                            });
                                            save_to_storage(&store.get_untracked());
                                            editing_id.set(Some(new_id2));
                                        }
                                    >
                                        "+ 値を追加"
                                    </button>
                                </div>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

            <button
                class="dim-add-btn"
                on:click=move |_| {
                    commit_node_edit(editing_id, id_ref, label_ref, color_ref, store);
                    let new_id = new_node_id();
                    let new_id2 = new_id.clone();
                    store.update(|s| {
                        s.dimensions.push(DimensionNode {
                            id: new_id.clone(),
                            label: String::new(),
                            color: "#8899AA".to_string(),
                            parent: None,
                        });
                    });
                    save_to_storage(&store.get_untracked());
                    editing_id.set(Some(new_id2));
                }
            >
                "+ 軸を追加"
            </button>
        </div>

        // ── 軸削除の確認ダイアログ ──────────────────────────────────────
        {move || {
            confirm_msg.get().map(|msg| {
                view! {
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
                }
            })
        }}

        // ── ノード削除ダイアログ（子があれば選択式）────────────────────
        {move || {
            delete_target.get().map(|(node_id, has_children)| {
                let label = node_id.clone();
                let v_promote = node_id.clone();
                let v_subtree = node_id.clone();
                let v_simple = node_id.clone();
                view! {
                    <div class="confirm-overlay" on:click=move |_| delete_target.set(None)>
                        <div class="confirm-dialog" on:click=|ev| ev.stop_propagation()>
                            <p class="confirm-text">{format!("「{label}」を削除します")}</p>
                            <div class="confirm-btns">
                                <button
                                    class="confirm-cancel"
                                    on:click=move |_| delete_target.set(None)
                                >
                                    "キャンセル"
                                </button>
                                {if has_children {
                                    view! {
                                        <button
                                            class="confirm-ok"
                                            on:click=move |_| {
                                                editing_id.set(None);
                                                delete_promote_flat(store, v_promote.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "子を繰り上げ"
                                        </button>
                                        <button
                                            class="confirm-ok confirm-danger"
                                            on:click=move |_| {
                                                editing_id.set(None);
                                                delete_subtree_flat(store, v_subtree.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "サブツリーごと"
                                        </button>
                                    }
                                    .into_view()
                                } else {
                                    view! {
                                        <button
                                            class="confirm-ok"
                                            on:click=move |_| {
                                                editing_id.set(None);
                                                delete_subtree_flat(store, v_simple.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "削除"
                                        </button>
                                    }
                                    .into_view()
                                }}
                            </div>
                        </div>
                    </div>
                }
            })
        }}
    }
}
