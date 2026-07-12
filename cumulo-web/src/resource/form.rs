use crate::category::{CategoryAttribute, CategoryId};
use crate::client::Client;
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use cumulo_model::{Forest, Resource, Taxonomy};

use leptos::html::Input;
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};

enum CategoryTreeItem {
    Branch {
        id: CategoryId,
        label: String,
        color: String,
        depth: usize,
    },
    Leaves {
        depth: usize,
        nodes: Vec<(CategoryId, String, String)>,
    },
}

impl CategoryTreeItem {
    fn rows(forest: &Taxonomy<CategoryAttribute>, root_id: &CategoryId) -> Vec<CategoryTreeItem> {
        let flat: Vec<(CategoryId, String, String, usize, bool, CategoryId)> = forest
            .dfs_order(root_id, &HashSet::new())
            .into_iter()
            .filter_map(|(id, depth, has_children)| {
                let n = forest.node(&id)?;
                let parent = n.parent.clone().unwrap_or_else(|| root_id.clone());
                let color = n.attribute.color.map(|c| c.to_hex()).unwrap_or_default();
                Some((id, n.label.clone(), color, depth, has_children, parent))
            })
            .collect();

        let mut result = Vec::new();
        let mut i = 0;
        while i < flat.len() {
            let (id, label, color, depth, has_children, ref parent_id) = &flat[i];
            let (id, label, color, depth, has_children) = (
                id.clone(),
                label.clone(),
                color.clone(),
                *depth,
                *has_children,
            );
            if has_children {
                result.push(CategoryTreeItem::Branch {
                    id: id.clone(),
                    label: label.clone(),
                    color: color.clone(),
                    depth,
                });
                i += 1;
            } else {
                let parent_id = parent_id.clone();
                let mut leaves = vec![(id.clone(), label.clone(), color.clone())];
                i += 1;
                while i < flat.len() {
                    let (id2, label2, color2, _, has2, ref p2) = &flat[i];
                    let (id2, label2, color2, has2) =
                        (id2.clone(), label2.clone(), color2.clone(), *has2);
                    if !has2 && *p2 == parent_id {
                        leaves.push((id2.clone(), label2.clone(), color2.clone()));
                        i += 1;
                    } else {
                        break;
                    }
                }
                result.push(CategoryTreeItem::Leaves {
                    depth,
                    nodes: leaves,
                });
            }
        }
        result
    }
}

#[component]
pub fn ResourceForm(
    client: Client,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let bipartite = client.read();
    let form_label = RwSignal::new(String::new());
    let form_url = RwSignal::new(String::new());
    let form_freq = RwSignal::new(1u32);
    let form_dims = RwSignal::new(HashMap::<CategoryId, CategoryId>::new());

    let label_ref = NodeRef::<Input>::new();
    let url_ref = NodeRef::<Input>::new();
    let freq_ref = NodeRef::<Input>::new();

    Effect::new(move |_| {
        let Some(r) = editing.get() else { return };

        form_label.set(r.label.clone().unwrap_or_default());
        form_url.set(r.attribute.console_url.clone());
        form_freq.set(r.attribute.freq.max(1));
        form_dims.set(bipartite.with_untracked(|s| {
            r.categories
                .iter()
                .filter_map(|v| s.taxonomy.root_of(v).map(|k| (k, v.clone())))
                .collect()
        }));

        if let Some(el) = label_ref.get() {
            el.set_value(&r.label.unwrap_or_default());
        }
        if let Some(el) = url_ref.get() {
            el.set_value(&r.attribute.console_url);
        }
        if let Some(el) = freq_ref.get() {
            el.set_value(&r.attribute.freq.max(1).to_string());
        }
    });

    let is_new = move || {
        editing.with(|e| {
            e.as_ref()
                .map(|r| bipartite.with(|s| s.catalog.node(&r.id).is_none()))
                .unwrap_or(false)
        })
    };

    let save = move || {
        let id = editing
            .with_untracked(|e| e.as_ref().map(|r| r.id.clone()))
            .unwrap_or_else(Platform::new_resource_id);

        let lbl = form_label.get_untracked();
        let r = Resource {
            id: id.clone(),
            label: if lbl.trim().is_empty() {
                None
            } else {
                Some(lbl)
            },
            parent: None,
            categories: form_dims.get_untracked().into_values().collect(),
            attribute: ResourceAttribute {
                console_url: form_url.get_untracked(),
                freq: form_freq.get_untracked(),
                created_at: None,
            },
        };

        client.update(|s| {
            if let Some(pos) = s.catalog.iter().position(|x| x.id == id) {
                s.catalog[pos] = r;
            } else {
                s.catalog.push(r);
            }
        });
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
                    <label class="form-label">"ラベル（省略可）"</label>
                    <input
                        node_ref=label_ref
                        class="form-input"
                        type="text"
                        placeholder="空欄でカテゴリ値から自動生成"
                        on:input=move |ev| form_label.set(event_target_value(&ev))
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

                    <label class="form-label">"カテゴリ"</label>
                    {move || {
                        let s = bipartite.get();
                        s.taxonomy.roots()
                            .into_iter()
                            .map(|root| {
                                let root_id = root.id.clone();
                                let root_label = if root.label.is_empty() {
                                    root.id.to_string()
                                } else {
                                    root.label.clone()
                                };
                                let chips = CategoryTreeItem::rows(&s.taxonomy, &root.id);
                                view! {
                                    <div class="form-category-row">
                                        <span class="form-category-label">{root_label}</span>
                                        <div class="form-category-tree">
                                            {chips
                                                .into_iter()
                                                .map(|item| match item {
                                                    CategoryTreeItem::Branch { id, label, color, depth } => {
                                                        let row_style = format!(
                                                            "padding-left:{}rem",
                                                            depth as f32 * 0.9
                                                        );
                                                        let k_sel = root_id.clone();
                                                        let v_sel = id.clone();
                                                        let k_clk = root_id.clone();
                                                        let v_clk = id.clone();
                                                        let chip_style = if !color.is_empty() {
                                                            format!("border-color:{color};background:{color}1a")
                                                        } else {
                                                            String::new()
                                                        };
                                                        view! {
                                                            <div class="form-category-node" style=row_style>
                                                                <span
                                                                    class="attr-chip category-branch"
                                                                    class:selected=move || {
                                                                        form_dims.get().get(&k_sel)
                                                                            .map(|v| v == &v_sel)
                                                                            .unwrap_or(false)
                                                                    }
                                                                    style=chip_style
                                                                    on:click=move |_| {
                                                                        let already = form_dims.get_untracked()
                                                                            .get(&k_clk).map(|v| v == &v_clk)
                                                                            .unwrap_or(false);
                                                                        if already {
                                                                            form_dims.update(|d| { d.remove(&k_clk); });
                                                                        } else {
                                                                            form_dims.update(|d| { d.insert(k_clk.clone(), v_clk.clone()); });
                                                                        }
                                                                    }
                                                                >
                                                                    {label}
                                                                </span>
                                                            </div>
                                                        }.into_any()
                                                    }
                                                    CategoryTreeItem::Leaves { depth, nodes } => {
                                                        let row_style = format!(
                                                            "padding-left:{}rem",
                                                            depth as f32 * 0.9
                                                        );
                                                        view! {
                                                            <div class="form-category-node form-category-leaf-row" style=row_style>
                                                                {nodes.into_iter().map(|(node_id, node_label, color)| {
                                                                    let k_sel = root_id.clone();
                                                                    let v_sel = node_id.clone();
                                                                    let k_clk = root_id.clone();
                                                                    let v_clk = node_id.clone();
                                                                    let chip_style = if !color.is_empty() {
                                                                        format!("border-color:{color};background:{color}1a")
                                                                    } else {
                                                                        String::new()
                                                                    };
                                                                    view! {
                                                                        <span
                                                                            class="attr-chip"
                                                                            class:selected=move || {
                                                                                form_dims.get().get(&k_sel)
                                                                                    .map(|v| v == &v_sel)
                                                                                    .unwrap_or(false)
                                                                            }
                                                                            style=chip_style
                                                                            on:click=move |_| {
                                                                                let already = form_dims.get_untracked()
                                                                                    .get(&k_clk).map(|v| v == &v_clk)
                                                                                    .unwrap_or(false);
                                                                                if already {
                                                                                    form_dims.update(|d| { d.remove(&k_clk); });
                                                                                } else {
                                                                                    form_dims.update(|d| { d.insert(k_clk.clone(), v_clk.clone()); });
                                                                                }
                                                                            }
                                                                        >
                                                                            {node_label}
                                                                        </span>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        }.into_any()
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
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
