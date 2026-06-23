use crate::category::CategoryAttribute;
use crate::platform::Platform;
use crate::resource::{ResourceAttribute, ResourceId};
use crate::storage::AppStorage;
use cumulo_model::{Bipartite, Forest, Resource};

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn EntitiesTab(
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    settings_open: RwSignal<bool>,
    return_to_settings: RwSignal<bool>,
) -> impl IntoView {
    // 削除対象 (id, 子を持つか)。子を持つ場合は繰り上げ / サブツリーを popup で選ばせる。
    let delete_target = RwSignal::new(Option::<(ResourceId, bool)>::None);

    // 削除はリソースも is-a 森なので、モデルの delete_promote / delete_subtree に委譲する。
    let delete_promote = move |id: ResourceId| {
        bipartite.update(|s| s.catalog.delete_promote(&id));
        AppStorage::save(&bipartite.get_untracked());
    };
    let delete_subtree = move |id: ResourceId| {
        bipartite.update(|s| s.catalog.delete_subtree(&id));
        AppStorage::save(&bipartite.get_untracked());
    };

    view! {
        <div class="resource-tab">
            <button
                class="resource-add-btn"
                on:click=move |_| {
                    return_to_settings.set(true);
                    editing.set(Some(Platform::new_resource()));
                    settings_open.set(false);
                }
            >
                "+ リソースを追加"
            </button>

            {move || {
                let s = bipartite.get();

                if s.catalog.is_empty() {
                    return view! {
                        <p class="resource-tab-empty">"リソースがありません"</p>
                    }
                    .into_any();
                }

                s.catalog
                    .iter()
                    .map(|r| {
                        let r_id = r.id.clone();
                        let r_edit = r.clone();
                        let display = r.display_label(&s.taxonomy);
                        let has_children = !s.catalog.children_of(&r.id).is_empty();
                        view! {
                            <div class="resource-row">
                                <span class="resource-row-name">{display}</span>
                                <div class="resource-row-actions">
                                    <button
                                        class="resource-row-edit"
                                        on:click=move |_| {
                                            return_to_settings.set(true);
                                            editing.set(Some(r_edit.clone()));
                                            settings_open.set(false);
                                        }
                                        title="編集"
                                    >
                                        <Icon icon=icon::HiPencilOutlineLg width="14" height="14" />
                                    </button>
                                    <button
                                        class="resource-row-delete"
                                        on:click=move |_| {
                                            delete_target.set(Some((r_id.clone(), has_children)));
                                        }
                                    >
                                        "×"
                                    </button>
                                </div>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_any()
            }}
        </div>

        {move || {
            delete_target.get().map(|(id, has_children)| {
                let label = bipartite
                    .with(|s| s.catalog.node(&id).map(|r| r.display_label(&s.taxonomy)))
                    .unwrap_or_else(|| id.to_string());
                let v_promote = id.clone();
                let v_subtree = id.clone();
                let v_simple = id.clone();
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
                                                delete_promote(v_promote.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "子を繰り上げ"
                                        </button>
                                        <button
                                            class="confirm-ok confirm-danger"
                                            on:click=move |_| {
                                                delete_subtree(v_subtree.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "サブツリーごと"
                                        </button>
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <button
                                            class="confirm-ok"
                                            on:click=move |_| {
                                                delete_promote(v_simple.clone());
                                                delete_target.set(None);
                                            }
                                        >
                                            "削除"
                                        </button>
                                    }
                                    .into_any()
                                }}
                            </div>
                        </div>
                    </div>
                }
            })
        }}
    }
}
