use crate::category::{CategoryAttribute, CategoryId};
use crate::client::Client;
use crate::resource::ResourceAttribute;
use cumulo_model::{Bipartite, ForestMut, Id};
use leptos::prelude::*;

#[component]
fn ConfirmShell(on_cancel: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <div class="confirm-overlay" on:click=move |_| on_cancel.run(())>
            <div class="confirm-dialog" on:click=|ev| ev.stop_propagation()>
                {children()}
            </div>
        </div>
    }
}

#[component]
pub fn ConfirmDialog(
    #[prop(into)] message: String,
    #[prop(into)] confirm_label: String,
    #[prop(optional)] danger: bool,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let ok_class = if danger {
        "confirm-ok confirm-danger"
    } else {
        "confirm-ok"
    };
    view! {
        <ConfirmShell on_cancel=on_cancel>
            <p class="confirm-text">{message}</p>
            <div class="confirm-btns">
                <button class="confirm-cancel" on:click=move |_| on_cancel.run(())>
                    "キャンセル"
                </button>
                <button class=ok_class on:click=move |_| on_confirm.run(())>
                    {confirm_label}
                </button>
            </div>
        </ConfirmShell>
    }
}

#[component]
fn DeleteShell(
    #[prop(into)] label: String,
    on_cancel: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <ConfirmShell on_cancel=on_cancel>
            <p class="confirm-text">{format!("「{label}」を削除します")}</p>
            <div class="confirm-btns">
                <button class="confirm-cancel" on:click=move |_| on_cancel.run(())>
                    "キャンセル"
                </button>
                {children()}
            </div>
        </ConfirmShell>
    }
}

type App = Bipartite<ResourceAttribute, CategoryAttribute>;

#[derive(Clone)]
pub struct CategoryRename {
    pub old_id: CategoryId,
    pub new_id: CategoryId,
    pub label: String,
    pub attribute: CategoryAttribute,
}

const AFFECTED_PREVIEW_CAP: usize = 8;

#[component]
pub fn CategoryRenameConfirm(
    client: Client,
    pending: RwSignal<Option<CategoryRename>>,
    on_after: Callback<()>,
) -> impl IntoView {
    move || {
        pending.get().map(|p| {
            let names = client.read().with(|b| {
                b.resources_with_category(&p.old_id)
                    .iter()
                    .map(|r| {
                        r.resolved_label(&b.taxonomy)
                            .unwrap_or_else(|| r.id.to_string())
                    })
                    .collect::<Vec<_>>()
            });
            let total = names.len();
            let shown: Vec<String> = names.into_iter().take(AFFECTED_PREVIEW_CAP).collect();
            let overflow = total - shown.len();
            let message = format!(
                "「{}」を「{}」に変更します。参照している {total} 件のリソースも更新されます。",
                p.old_id, p.new_id,
            );
            let on_cancel = Callback::new(move |_| pending.set(None));
            let on_confirm = move |_| {
                let applied = client
                    .signal()
                    .try_update(|b| {
                        b.rename_category(
                            &p.old_id,
                            p.new_id.clone(),
                            &p.label,
                            p.attribute.clone(),
                        )
                        .is_ok()
                    })
                    .unwrap_or(false);
                pending.set(None);
                if applied {
                    client.save();
                    on_after.run(());
                } else {
                    client.notify("名前を変更できませんでした");
                }
            };
            view! {
                <ConfirmShell on_cancel=on_cancel>
                    <p class="confirm-text">{message}</p>
                    <ul class="confirm-list">
                        {shown.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}
                        {(overflow > 0).then(|| view! { <li>{format!("ほか {overflow} 件")}</li> })}
                    </ul>
                    <div class="confirm-btns">
                        <button class="confirm-cancel" on:click=move |_| pending.set(None)>
                            "キャンセル"
                        </button>
                        <button class="confirm-ok" on:click=on_confirm>
                            "変更"
                        </button>
                    </div>
                </ConfirmShell>
            }
        })
    }
}

#[component]
pub fn CategoryDeleteConfirm(
    client: Client,
    target: RwSignal<Option<(CategoryId, bool)>>,
    on_after: Callback<()>,
) -> impl IntoView {
    let apply = move |id: CategoryId, subtree: bool| {
        client.update(|b| b.delete_category(&id, subtree));
        on_after.run(());
        target.set(None);
    };
    move || {
        target.get().map(|(id, has_children)| {
            let (promote_count, subtree_names) = client.read().with(|b| {
                let subtree_names = b
                    .resources_affected_by_delete(&id, true)
                    .iter()
                    .map(|r| {
                        r.resolved_label(&b.taxonomy)
                            .unwrap_or_else(|| r.id.to_string())
                    })
                    .collect::<Vec<_>>();
                (b.resources_affected_by_delete(&id, false).len(), subtree_names)
            });
            let total = subtree_names.len();
            let shown: Vec<String> = subtree_names.into_iter().take(AFFECTED_PREVIEW_CAP).collect();
            let overflow = total - shown.len();
            let impact = if has_children {
                format!("子を繰り上げ: {promote_count} 件 / サブツリーごと: {total} 件 のリソースからタグを外します。")
            } else {
                format!("参照している {total} 件のリソースからタグを外します。")
            };
            let on_cancel = Callback::new(move |_| target.set(None));
            let label = id.to_string();
            let buttons = if has_children {
                let promote_id = id.clone();
                view! {
                    <button class="confirm-ok" on:click=move |_| apply(promote_id.clone(), false)>
                        "子を繰り上げ"
                    </button>
                    <button
                        class="confirm-ok confirm-danger"
                        on:click=move |_| apply(id.clone(), true)
                    >
                        "サブツリーごと"
                    </button>
                }
                .into_any()
            } else {
                view! {
                    <button class="confirm-ok" on:click=move |_| apply(id.clone(), false)>
                        "削除"
                    </button>
                }
                .into_any()
            };
            view! {
                <ConfirmShell on_cancel=on_cancel>
                    <p class="confirm-text">{format!("「{label}」を削除します")}</p>
                    {(total > 0)
                        .then(|| {
                            view! {
                                <p class="confirm-text">{impact}</p>
                                <ul class="confirm-list">
                                    {shown
                                        .into_iter()
                                        .map(|n| view! { <li>{n}</li> })
                                        .collect_view()}
                                    {(overflow > 0)
                                        .then(|| view! { <li>{format!("ほか {overflow} 件")}</li> })}
                                </ul>
                            }
                        })}
                    <div class="confirm-btns">
                        <button class="confirm-cancel" on:click=move |_| target.set(None)>
                            "キャンセル"
                        </button>
                        {buttons}
                    </div>
                </ConfirmShell>
            }
        })
    }
}

#[component]
pub fn ForestDeleteConfirm<F, S, L>(
    client: Client,
    select: S,
    target: RwSignal<Option<(Id<F::Node>, bool)>>,
    label: L,
    #[prop(optional)] on_after: Option<Callback<()>>,
) -> impl IntoView
where
    F: ForestMut + 'static,
    F::Node: 'static,
    S: Fn(&mut App) -> &mut F + Copy + Send + Sync + 'static,
    L: Fn(&Id<F::Node>) -> String + Copy + Send + Sync + 'static,
{
    let apply = move |id: Id<F::Node>, subtree: bool| {
        client.update(|b| {
            let forest = select(b);
            if subtree {
                forest.delete_subtree(&id);
            } else {
                forest.delete_promote(&id);
            }
        });
        if let Some(cb) = on_after {
            cb.run(());
        }
        target.set(None);
    };

    move || {
        target.get().map(|(id, has_children)| {
            let text = label(&id);
            let on_cancel = Callback::new(move |_| target.set(None));
            if has_children {
                let promote_id = id.clone();
                view! {
                    <DeleteShell label=text on_cancel=on_cancel>
                        <button class="confirm-ok" on:click=move |_| apply(promote_id.clone(), false)>
                            "子を繰り上げ"
                        </button>
                        <button
                            class="confirm-ok confirm-danger"
                            on:click=move |_| apply(id.clone(), true)
                        >
                            "サブツリーごと"
                        </button>
                    </DeleteShell>
                }
                .into_any()
            } else {
                view! {
                    <DeleteShell label=text on_cancel=on_cancel>
                        <button class="confirm-ok" on:click=move |_| apply(id.clone(), false)>
                            "削除"
                        </button>
                    </DeleteShell>
                }
                .into_any()
            }
        })
    }
}
