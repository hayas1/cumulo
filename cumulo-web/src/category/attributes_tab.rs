use crate::category::{CategoryAttribute, CategoryId, DEFAULT_COLOR};
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::shared::{
    CategoryDeleteConfirm, CategoryRename, CategoryRenameConfirm, Color, ConfirmDialog,
};
use cumulo_model::{Category, Forest, ForestMut};

use icondata as icon;
use leptos::html::{Div, Input};
use leptos::prelude::*;
use leptos_icons::Icon;
use std::collections::HashSet;
use std::sync::Arc;
use wasm_bindgen::JsCast;

#[derive(Copy, Clone)]
struct CategoryTabActions(Client);

impl CategoryTabActions {
    fn reparent(self, dragged: CategoryId, new_parent: Option<CategoryId>) {
        let moved = self
            .0
            .signal()
            .try_update(|s| s.taxonomy.reparent(&dragged, new_parent).is_ok())
            .unwrap_or(false);
        if moved {
            self.0.save();
        }
    }

    fn move_relative(self, dragged: CategoryId, target: CategoryId, after: bool) {
        let moved = self
            .0
            .signal()
            .try_update(|s| s.taxonomy.move_relative(&dragged, &target, after).is_ok())
            .unwrap_or(false);
        if moved {
            self.0.save();
        }
    }

    fn delete_subtree(self, node_id: CategoryId) {
        self.0.update(|s| s.delete_category(&node_id, true));
    }

    fn commit_node_edit(
        self,
        editing_id: RwSignal<Option<CategoryId>>,
        id_ref: NodeRef<Input>,
        label_ref: NodeRef<Input>,
        color_ref: NodeRef<Input>,
        rename_confirm: RwSignal<Option<CategoryRename>>,
    ) -> bool {
        let Some(old_id) = editing_id.get_untracked() else {
            return true;
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
            return true;
        }
        let new_id: CategoryId = new_id.try_into().unwrap();
        let attribute = CategoryAttribute {
            color: Color::from_hex(&new_color),
        };
        let changed_id = new_id != old_id;
        let (id_taken, affected) = self.0.signal().with_untracked(|s| {
            (
                changed_id && s.taxonomy.node(&new_id).is_some(),
                if changed_id {
                    s.resources_with_category(&old_id).len()
                } else {
                    0
                },
            )
        });
        if id_taken {
            let i18n = use_i18n();
            self.0
                .notify(t_string!(i18n, category_id_taken).to_string());
            return false;
        }
        if affected > 0 {
            rename_confirm.set(Some(CategoryRename {
                old_id,
                new_id,
                label: new_label,
                attribute,
            }));
            return false;
        }
        let applied = self
            .0
            .signal()
            .try_update(|s| {
                s.rename_category(&old_id, new_id, &new_label, attribute)
                    .is_ok()
            })
            .unwrap_or(false);
        if applied {
            self.0.save();
            editing_id.set(None);
        }
        applied
    }
}

#[derive(Copy, Clone)]
struct ConfirmState {
    msg: RwSignal<Option<String>>,
    action: RwSignal<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl ConfirmState {
    fn prompt(self, msg: String, action: impl Fn() + Send + Sync + 'static) {
        self.msg.set(Some(msg));
        self.action.set(Some(Arc::new(action)));
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
pub fn AttributesTab(client: Client) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.signal();
    let editing_id = RwSignal::new(Option::<CategoryId>::None);
    let id_ref = NodeRef::<Input>::new();
    let label_ref = NodeRef::<Input>::new();
    let color_ref = NodeRef::<Input>::new();
    let preview_color = RwSignal::new(String::new());

    let collapsed = RwSignal::new(HashSet::<CategoryId>::new());
    let dragging = RwSignal::new(Option::<CategoryId>::None);
    let drag_over = RwSignal::new(Option::<(CategoryId, u8)>::None);

    let confirm = ConfirmState {
        msg: RwSignal::new(None),
        action: RwSignal::new(None),
    };
    let acts = CategoryTabActions(client);

    let delete_target = RwSignal::new(Option::<(CategoryId, bool)>::None);
    let rename_confirm = RwSignal::new(Option::<CategoryRename>::None);

    Effect::new(move |_| {
        let Some(eid) = editing_id.get() else {
            preview_color.set(String::new());
            return;
        };
        let s = bipartite.get_untracked();
        let Some(n) = s.taxonomy.node(&eid) else {
            return;
        };
        preview_color.set(n.attribute.color.map(|c| c.to_hex()).unwrap_or_default());
        if let Some(el) = id_ref.get() {
            el.set_value(&n.id);
        }
        if let Some(el) = label_ref.get() {
            el.set_value(&n.label);
        }
        if let Some(el) = color_ref.get() {
            el.set_value(&n.attribute.color.map(|c| c.to_hex()).unwrap_or_default());
        }
    });

    view! {
        <div class="category-tab" tabindex="-1">
            {move || {
                let s = bipartite.get();
                let collapsed_set = collapsed.get();
                let current_editing = editing_id.get();
                let root_nodes: Vec<Category<CategoryAttribute>> = s
                    .taxonomy
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

                        let order = s.taxonomy.dfs_order(&root.id, &collapsed_set);
                        let sentinel: CategoryId = format!("\x00root:{}", root.id).try_into().unwrap();

                        let is_root_editing = current_editing.as_deref() == Some(root.id.as_str());

                        view! {
                            <div
                                class="category-row"
                                class:active=move || {
                                    if let Some(ref eid) = editing_id.get() {
                                        *eid == root_id_active
                                            || bipartite.with(|s| {
                                                s.taxonomy.root_of(eid)
                                                    .map(|r| r == root_id_active)
                                                    .unwrap_or(false)
                                            })
                                    } else {
                                        false
                                    }
                                }
                            >
                                <div class="category-row-header">
                                    {if is_root_editing {
                                        let rid_cancel = root_id_header.clone();
                                        view! {
                                            <form
                                                class="category-name-editor"
                                                on:submit=move |ev: web_sys::SubmitEvent| {
                                                    ev.prevent_default();
                                                    acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm);
                                                }
                                                on:focusout=move |ev: web_sys::FocusEvent| {
                                                    if UiHelper::focus_left(&ev, ".category-name-editor") {
                                                        acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm);
                                                    }
                                                }
                                            >
                                                <input
                                                    node_ref=label_ref
                                                    class="category-input"
                                                    type="text"
                                                    placeholder=move || t_string!(i18n, category_placeholder_label)
                                                />
                                                <span class="category-name-sep">"/"</span>
                                                <input
                                                    node_ref=id_ref
                                                    class="category-input category-input-id"
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
                                                    type="button"
                                                    class="category-row-cancel"
                                                    on:click=move |_| {
                                                        editing_id.set(None);
                                                        bipartite.update(|s| {
                                                            if let Some(n) = s
                                                                .taxonomy
                                                                .iter()
                                                                .find(|n| n.id == rid_cancel)
                                                            {
                                                                if n.label.is_empty() && n.id.starts_with("node") {
                                                                    let id = n.id.clone();
                                                                    s.taxonomy.retain(|n| n.id != id);
                                                                }
                                                            }
                                                        });
                                                        client.save();
                                                    }
                                                >
                                                    {t!(i18n, action_cancel)}
                                                </button>
                                                <button type="submit" class="editor-submit" tabindex="-1" aria-hidden="true" />
                                            </form>
                                        }
                                        .into_any()
                                    } else {
                                        let rid_click = root_id_header.clone();
                                        view! {
                                            <button
                                                class="category-name-btn"
                                                on:click=move |_| {
                                                    if acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm) {
                                                        editing_id.set(Some(rid_click.clone()));
                                                    }
                                                }
                                            >
                                                <span class="category-label-text">
                                                    {if root.label.is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        root.label.clone()
                                                    }}
                                                </span>
                                                <span class="category-id-text">{root.id.to_string()}</span>
                                            </button>
                                        }
                                        .into_any()
                                    }}

                                    {
                                        let s_drag_z = sentinel.clone();
                                        let rid_drop_z = root_id_drop.clone();
                                        move || {
                                            if dragging.get().is_some() {
                                                let s_drag = s_drag_z.clone();
                                                let s_over = s_drag_z.clone();
                                                let rid_drop = rid_drop_z.clone();
                                                view! {
                                                    <div
                                                        class="category-root-drop"
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
                                                        {t!(i18n, category_drop_here)}
                                                    </div>
                                                }
                                                .into_any()
                                            } else {
                                                view! { <span /> }.into_any()
                                            }
                                        }
                                    }

                                    {if !is_root_editing {
                                        let rid_d = root_id_del.clone();
                                        view! {
                                            <button
                                                class="category-row-delete"
                                                on:click=move |_| {
                                                    let id = rid_d.clone();
                                                    editing_id.set(None);
                                                    confirm.prompt(
                                                        t_string!(i18n, category_delete_confirm).to_string(),
                                                        move || acts.delete_subtree(id.clone()),
                                                    );
                                                }
                                            >
                                                "×"
                                            </button>
                                        }
                                        .into_any()
                                    } else {
                                        view! { <span /> }.into_any()
                                    }}
                                </div>

                                <div class="category-tree">
                                    {order
                                        .into_iter()
                                        .map(|(node_id, depth, has_children)| {
                                            let row_ref = NodeRef::<Div>::new();
                                            let n = s
                                                .taxonomy
                                                .iter()
                                                .find(|n| n.id == node_id)
                                                .cloned()
                                                .unwrap();
                                            let node_color =
                                                n.attribute.color.map(|c| c.to_hex()).unwrap_or_default();
                                            let node_label_text = if n.label.is_empty() {
                                                n.id.to_string()
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
                                                        class="category-caret"
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
                                                .into_any()
                                            } else {
                                                view! { <span class="category-caret-spacer" /> }
                                                    .into_any()
                                            };

                                            let v_drag = node_id.clone();
                                            let v_drop = node_id.clone();
                                            let v_over = node_id.clone();
                                            let v_in = node_id.clone();
                                            let v_bf = node_id.clone();
                                            let v_af = node_id.clone();

                                            let row_body = if is_node_editing {
                                                view! {
                                                    <form
                                                        class="chip-editor category-node-editor"
                                                        on:submit=move |ev: web_sys::SubmitEvent| {
                                                            ev.prevent_default();
                                                            acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm);
                                                        }
                                                        on:focusout=move |ev: web_sys::FocusEvent| {
                                                            if UiHelper::focus_left(&ev, ".chip-editor") {
                                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm);
                                                            }
                                                        }
                                                    >
                                                        <input
                                                            node_ref=label_ref
                                                            class="chip-editor-val"
                                                            type="text"
                                                            placeholder=move || t_string!(i18n, category_placeholder_label)
                                                        />
                                                        <span class="category-name-sep">"/"</span>
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
                                                            type="button"
                                                            class="chip-editor-randomize"
                                                            on:click=move |_| {
                                                                let color = Platform::random_color();
                                                                if let Some(el) = color_ref.get_untracked()
                                                                {
                                                                    el.set_value(&color.to_hex());
                                                                }
                                                                preview_color.set(color.to_hex());
                                                            }
                                                        >
                                                            <Icon
                                                                icon=icon::HiArrowPathOutlineLg
                                                                width="14"
                                                                height="14"
                                                            />
                                                        </button>
                                                        <button
                                                            type="button"
                                                            class="chip-editor-cancel"
                                                            on:click=move |_| editing_id.set(None)
                                                        >
                                                            {t!(i18n, action_cancel)}
                                                        </button>
                                                        <button type="submit" class="editor-submit" tabindex="-1" aria-hidden="true" />
                                                    </form>
                                                }
                                                .into_any()
                                            } else {
                                                let nid_style = node_id.clone();
                                                let nid_click = node_id.clone();
                                                let nid_add = node_id.clone();
                                                let nid_del = node_id.clone();
                                                view! {
                                                    <button
                                                        class="category-node-label"
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
                                                            let proceed = if cur.as_deref() != Some(nid_click.as_str())
                                                                && cur.is_some()
                                                            {
                                                                acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm)
                                                            } else {
                                                                true
                                                            };
                                                            if proceed {
                                                                editing_id.set(Some(nid_click.clone()));
                                                            }
                                                        }
                                                    >
                                                        {node_label_text}
                                                    </button>
                                                    <button
                                                        class="category-node-add-child"
                                                        title=move || t_string!(i18n, category_add_child)
                                                        on:click=move |_| {
                                                            if editing_id.get_untracked().is_some()
                                                                && !acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm)
                                                            {
                                                                return;
                                                            }
                                                            let parent = nid_add.clone();
                                                            let new_id = Platform::new_node_id();
                                                            let new_id2 = new_id.clone();
                                                            bipartite.update(|s| {
                                                                s.taxonomy.push(Category {
                                                                    id: new_id.clone(),
                                                                    label: String::new(),
                                                                    attribute: CategoryAttribute {
                                                                        color: Some(Platform::random_color()),
                                                                    },
                                                                    parent: Some(parent.clone()),
                                                                });
                                                            });
                                                            client.save();
                                                            collapsed.update(|c| {
                                                                c.remove(&nid_add);
                                                            });
                                                            editing_id.set(Some(new_id2));
                                                        }
                                                    >
                                                        "＋"
                                                    </button>
                                                    <button
                                                        class="category-node-delete"
                                                        title=move || t_string!(i18n, action_delete)
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
                                                .into_any()
                                            };

                                            view! {
                                                <div
                                                    node_ref=row_ref
                                                    class="category-node-row"
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
                                                        class="category-drag-handle"
                                                        draggable="true"
                                                        title=move || t_string!(i18n, category_drag_reparent)
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
                                        class="category-add-root"
                                        on:click=move |_| {
                                            if editing_id.get_untracked().is_some()
                                                && !acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm)
                                            {
                                                return;
                                            }
                                            let new_id = Platform::new_node_id();
                                            let new_id2 = new_id.clone();
                                            bipartite.update(|s| {
                                                s.taxonomy.push(Category {
                                                    id: new_id.clone(),
                                                    label: String::new(),
                                                    attribute: CategoryAttribute {
                                                        color: Some(Platform::random_color()),
                                                    },
                                                    parent: Some(root_id_add.clone()),
                                                });
                                            });
                                            client.save();
                                            editing_id.set(Some(new_id2));
                                        }
                                    >
                                        {t!(i18n, category_add_value)}
                                    </button>
                                </div>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

            <button
                class="category-add-btn"
                on:click=move |_| {
                    if !acts.commit_node_edit(editing_id, id_ref, label_ref, color_ref, rename_confirm) {
                        return;
                    }
                    let new_id = Platform::new_node_id();
                    let new_id2 = new_id.clone();
                    bipartite.update(|s| {
                        s.taxonomy.push(Category {
                            id: new_id.clone(),
                            label: String::new(),
                            attribute: CategoryAttribute { color: Some(DEFAULT_COLOR) },
                            parent: None,
                        });
                    });
                    client.save();
                    editing_id.set(Some(new_id2));
                }
            >
                {t!(i18n, category_add_axis)}
            </button>
        </div>

        {move || {
            confirm.msg.get().map(|msg| {
                view! {
                    <ConfirmDialog
                        message=msg
                        confirm_label=t_string!(i18n, action_delete)
                        on_confirm=Callback::new(move |_| {
                            if let Some(action) = confirm.action.get_untracked() {
                                action();
                            }
                            confirm.close();
                        })
                        on_cancel=Callback::new(move |_| confirm.close())
                    />
                }
            })
        }}

        <CategoryDeleteConfirm
            client=client
            target=delete_target
            on_after=Callback::new(move |_| editing_id.set(None))
        />

        <CategoryRenameConfirm
            client=client
            pending=rename_confirm
            on_after=Callback::new(move |_| editing_id.set(None))
        />
    }
}
