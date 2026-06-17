use crate::platform::{CategoryAttribute, Platform, ResourceAttribute};
use crate::storage::AppStorage;
use cumulo_model::{Bipartite, Resource};

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::sync::Arc;

fn ask_confirm(
    msg: &'static str,
    action: impl Fn() + Send + Sync + 'static,
    confirm_msg: RwSignal<Option<&'static str>>,
    confirm_action: RwSignal<Option<Arc<dyn Fn() + Send + Sync>>>,
) {
    confirm_msg.set(Some(msg));
    confirm_action.set(Some(Arc::new(action)));
}

#[component]
pub fn EntitiesTab(
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
    settings_open: RwSignal<bool>,
    return_to_settings: RwSignal<bool>,
) -> impl IntoView {
    let confirm_msg = RwSignal::new(Option::<&'static str>::None);
    let confirm_action: RwSignal<Option<Arc<dyn Fn() + Send + Sync>>> = RwSignal::new(None);

    let close_confirm = move || {
        confirm_msg.set(None);
        confirm_action.set(None);
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
                                            let id = r_id.clone();
                                            ask_confirm(
                                                "このリソースを削除しますか？",
                                                move || {
                                                    bipartite.update(|s| {
                                                        s.catalog.retain(|r| r.id != id)
                                                    });
                                                    AppStorage::save(&bipartite.get_untracked());
                                                },
                                                confirm_msg,
                                                confirm_action,
                                            );
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
    }
}
