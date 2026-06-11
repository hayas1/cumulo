use crate::model::{AppStore, DimensionNode};

use icondata as icon;
use leptos::html::{Div, Input};
use leptos::*;
use leptos_icons::Icon;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Copy, Clone)]
struct DimTabActions(RwSignal<AppStore>);

impl DimTabActions {
    fn reparent(self, dragged: String, new_parent: Option<String>) {
        self.0.update(|s| s.dimensions.reparent(&dragged, new_parent));
        self.0.get_untracked().save_to_storage();
    }

    fn move_relative(self, dragged: String, target: String, after: bool) {
        self.0.update(|s| s.dimensions.move_relative(&dragged, &target, after));
        self.0.get_untracked().save_to_storage();
    }

    fn delete_promote(self, node_id: String) {
        self.0.update(|s| s.dimensions.delete_promote(&node_id));
        self.0.get_untracked().save_to_storage();
    }

    fn delete_subtree(self, node_id: String) {
        self.0.update(|s| s.dimensions.delete_subtree(&node_id));
        self.0.get_untracked().save_to_storage();
    }

    fn commit_node_edit(
        self,
        editing_id: RwSignal<Option<String>>,
        id_ref: NodeRef<Input>,
        label_ref: NodeRef<Input>,
        color_ref: NodeRef<Input>,
    ) {
        let Some(old_id) = editing_id.get_untracked() else {
            return;
        };
        let new_id = id_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
        let new_label = label_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
        let new_color = color_ref.get_untracked().map(|el| el.value()).unwrap_or_default();
        if new_id.trim().is_empty() {
            return;
        }
        self.0.update(|s| s.dimensions.rename_node(&old_id, &new_id, &new_label, &new_color));
        self.0.get_untracked().save_to_storage();
        editing_id.set(None);
    }
}

#[derive(Copy, Clone)]
struct ConfirmState {
    msg: RwSignal<Option<&'static str>>,
    action: RwSignal<Option<Rc<dyn Fn()>>>,
}

impl ConfirmState {
    fn prompt(self, msg: &'static str, action: impl Fn() + 'static) {
        self.msg.set(Some(msg));
        self.action.set(Some(Rc::new(action)));
    }

    fn close(self) {
        self.msg.set(None);
        self.action.set(None);
    }
}

struct UiHelper;

impl UiHelper {
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
                UiHelper::zone_from(r.top(), r.height(), ev.client_y())
            })
            .unwrap_or(1)
    }
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

    let confirm = ConfirmState {
        msg: create_rw_signal(None),
        action: create_rw_signal(None),
    };
    let acts = DimTabActions(store);

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
                        let sentinel = format!("\x00root:{}", root.id);

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
                                <div class="dim-row-header">
                                    {if is_root_editing {
                                        let rid_cancel = root_id_header.clone();
                                        view! {
                                            <div
                                                class="dim-name-editor"
                                                on:focusout=move |ev: web_sys::FocusEvent| {
                                                    if UiHelper::focus_left(&ev, ".dim-name-editor") {
                                                        acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
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
                                                        store.get_untracked().save_to_storage();
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
                                                    acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
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
                                                        acts.reparent(dval, Some(rid_drop.clone()));
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
                                                    confirm.prompt(
                                                        "このディメンションを削除しますか？",
                                                        move || acts.delete_subtree(id.clone()),
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
                                                            if UiHelper::focus_left(&ev, ".chip-editor") {
                                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
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
                                                                let color = DimensionNode::random_color();
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
                                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
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
                                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
                                                            }
                                                            let parent = nid_add.clone();
                                                            let new_id = DimensionNode::new_id();
                                                            let new_id2 = new_id.clone();
                                                            store.update(|s| {
                                                                s.dimensions.push(DimensionNode {
                                                                    id: new_id.clone(),
                                                                    label: String::new(),
                                                                    color: DimensionNode::random_color(),
                                                                    parent: Some(parent.clone()),
                                                                });
                                                            });
                                                            store.get_untracked().save_to_storage();
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
                                                        let zone = UiHelper::zone_of(row_ref, &ev);
                                                        drag_over.set(Some((v_over.clone(), zone)));
                                                    }
                                                    on:drop=move |ev: web_sys::DragEvent| {
                                                        ev.prevent_default();
                                                        let zone = UiHelper::zone_of(row_ref, &ev);
                                                        if let Some(dval) = dragging.get_untracked()
                                                        {
                                                            match zone {
                                                                0 => acts.move_relative(dval, v_drop.clone(), false),
                                                                2 => acts.move_relative(dval, v_drop.clone(), true),
                                                                _ => acts.reparent(dval, Some(v_drop.clone())),
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
                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
                                            }
                                            let new_id = DimensionNode::new_id();
                                            let new_id2 = new_id.clone();
                                            store.update(|s| {
                                                s.dimensions.push(DimensionNode {
                                                    id: new_id.clone(),
                                                    label: String::new(),
                                                    color: DimensionNode::random_color(),
                                                    parent: Some(root_id_add.clone()),
                                                });
                                            });
                                            store.get_untracked().save_to_storage();
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
                    acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref);
                    let new_id = DimensionNode::new_id();
                    let new_id2 = new_id.clone();
                    store.update(|s| {
                        s.dimensions.push(DimensionNode {
                            id: new_id.clone(),
                            label: String::new(),
                            color: "#8899AA".to_string(),
                            parent: None,
                        });
                    });
                    store.get_untracked().save_to_storage();
                    editing_id.set(Some(new_id2));
                }
            >
                "+ 軸を追加"
            </button>
        </div>

        {move || {
            confirm.msg.get().map(|msg| {
                view! {
                    <div class="confirm-overlay" on:click=move |_| confirm.close()>
                        <div class="confirm-dialog" on:click=|ev| ev.stop_propagation()>
                            <p class="confirm-text">{msg}</p>
                            <div class="confirm-btns">
                                <button class="confirm-cancel" on:click=move |_| confirm.close()>
                                    "キャンセル"
                                </button>
                                <button
                                    class="confirm-ok"
                                    on:click=move |_| {
                                        if let Some(action) = confirm.action.get_untracked() {
                                            action();
                                        }
                                        confirm.close();
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
                                                acts.delete_promote(v_promote.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "子を繰り上げ"
                                        </button>
                                        <button
                                            class="confirm-ok confirm-danger"
                                            on:click=move |_| {
                                                editing_id.set(None);
                                                acts.delete_subtree(v_subtree.clone());
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
                                                acts.delete_subtree(v_simple.clone());
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
