use crate::platform::{CategoryAttribute, CategoryId, Platform, ResourceAttribute};
use crate::storage::AppStorage;
use cumulo_model::{Bipartite, Forest, Resource, Taxonomy};

use leptos::html::Input;
use leptos::*;
use std::collections::HashMap;

enum DimTreeItem {
    Branch {
        id: CategoryId,
        label: String,
        color: String,
        depth: usize,
    },
    /// 同じ親を持つ葉ノードをまとめた行 (id, label, color)
    Leaves {
        depth: usize,
        nodes: Vec<(CategoryId, String, String)>,
    },
}

fn descendants_dfs(forest: &Taxonomy<CategoryAttribute>, root_id: &CategoryId) -> Vec<DimTreeItem> {
    let mut flat: Vec<(CategoryId, String, String, usize, bool, CategoryId)> = Vec::new();
    fn dfs(
        forest: &Taxonomy<CategoryAttribute>,
        parent_id: &CategoryId,
        depth: usize,
        flat: &mut Vec<(CategoryId, String, String, usize, bool, CategoryId)>,
    ) {
        for n in forest.children_of(parent_id) {
            let has_children = !forest.children_of(&n.id).is_empty();
            flat.push((
                n.id.clone(),
                n.label.clone(),
                n.attribute.color.clone(),
                depth,
                has_children,
                parent_id.clone(),
            ));
            dfs(forest, &n.id, depth + 1, flat);
        }
    }
    dfs(forest, root_id, 0, &mut flat);

    // 連続する同一親の葉ノードをまとめる
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
            result.push(DimTreeItem::Branch {
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
            result.push(DimTreeItem::Leaves {
                depth,
                nodes: leaves,
            });
        }
    }
    result
}

#[component]
pub fn EntityForm(
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let form_label = create_rw_signal(String::new());
    let form_url = create_rw_signal(String::new());
    let form_freq = create_rw_signal(1u32);
    let form_dims = create_rw_signal(HashMap::<CategoryId, CategoryId>::new());

    let label_ref = create_node_ref::<Input>();
    let url_ref = create_node_ref::<Input>();
    let freq_ref = create_node_ref::<Input>();

    create_effect(move |_| {
        let Some(r) = editing.get() else { return };

        form_label.set(r.label.clone().unwrap_or_default());
        form_url.set(r.attribute.console_url.clone());
        form_freq.set(r.attribute.freq.max(1));
        form_dims.set(r.categories.clone());

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

    let is_new = move || editing.with(|e| e.as_ref().map(|r| r.id.is_empty()).unwrap_or(false));

    let save = move || {
        let id = editing
            .with_untracked(|e| {
                e.as_ref()
                    .filter(|r| !r.id.is_empty())
                    .map(|r| r.id.clone())
            })
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
            categories: form_dims.get_untracked(),
            attribute: ResourceAttribute {
                console_url: form_url.get_untracked(),
                freq: form_freq.get_untracked(),
                created_at: None,
            },
        };

        bipartite.update(|s| {
            if let Some(pos) = s.catalog.iter().position(|x| x.id == id) {
                s.catalog[pos] = r;
            } else {
                s.catalog.push(r);
            }
        });
        AppStorage::save(&bipartite.get_untracked());
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
                        placeholder="空欄でディメンション値から自動生成"
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

                    // ── 軸ごとのディメンションチップ ──────────────────────────
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
                                let chips = descendants_dfs(&s.taxonomy, &root.id);
                                view! {
                                    <div class="form-dim-row">
                                        <span class="form-dim-label">{root_label}</span>
                                        <div class="form-dim-tree">
                                            {chips
                                                .into_iter()
                                                .map(|item| match item {
                                                    DimTreeItem::Branch { id, label, color, depth } => {
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
                                                            <div class="form-dim-node" style=row_style>
                                                                <span
                                                                    class="attr-chip dim-branch"
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
                                                        }.into_view()
                                                    }
                                                    DimTreeItem::Leaves { depth, nodes } => {
                                                        let row_style = format!(
                                                            "padding-left:{}rem",
                                                            depth as f32 * 0.9
                                                        );
                                                        view! {
                                                            <div class="form-dim-node form-dim-leaf-row" style=row_style>
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
                                                        }.into_view()
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
