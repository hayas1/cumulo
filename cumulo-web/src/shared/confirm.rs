use crate::category::{CategoryAttribute, CategoryId};
use crate::client::Client;
use crate::i18n::*;
use crate::resource::ResourceAttribute;
use cumulo_model::{Bipartite, Forest, ForestMut, Id};
use leptos::prelude::*;

#[component]
fn ConfirmShell(on_cancel: Callback<()>, children: Children) -> impl IntoView {
    let armed = RwSignal::new(false);
    view! {
        <div
            class="confirm-overlay"
            on:mousedown=move |ev: web_sys::MouseEvent| armed.set(ev.target() == ev.current_target())
            on:click=move |ev: web_sys::MouseEvent| {
                if armed.get() && ev.target() == ev.current_target() {
                    on_cancel.run(());
                }
                armed.set(false);
            }
        >
            <div class="confirm-dialog">{children()}</div>
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
    let i18n = use_i18n();
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
                    {t!(i18n, action_cancel)}
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
    let i18n = use_i18n();
    view! {
        <ConfirmShell on_cancel=on_cancel>
            <p class="confirm-text">{t!(i18n, confirm_delete_label, label = label)}</p>
            <div class="confirm-btns">
                <button class="confirm-cancel" on:click=move |_| on_cancel.run(())>
                    {t!(i18n, action_cancel)}
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
    let i18n = use_i18n();
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
            let message = t_string!(
                i18n,
                rename_message,
                old = p.old_id.to_string(),
                new = p.new_id.to_string(),
                count = total,
            )
            .to_string();
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
                    client.notify(t_string!(i18n, rename_failed).to_string());
                }
            };
            view! {
                <ConfirmShell on_cancel=on_cancel>
                    <p class="confirm-text">{message}</p>
                    <ul class="confirm-list">
                        {shown.into_iter().map(|n| view! { <li>{n}</li> }).collect_view()}
                        {(overflow > 0).then(|| view! { <li>{t!(i18n, confirm_more, count = overflow)}</li> })}
                    </ul>
                    <div class="confirm-btns">
                        <button class="confirm-cancel" on:click=move |_| pending.set(None)>
                            {t!(i18n, action_cancel)}
                        </button>
                        <button class="confirm-ok" on:click=on_confirm>
                            {t!(i18n, rename_confirm)}
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
    let i18n = use_i18n();
    let apply = move |id: CategoryId, subtree: bool| {
        client.update(|b| b.delete_category(&id, subtree));
        on_after.run(());
        target.set(None);
    };
    move || {
        target.get().map(|(id, has_children)| {
            let (promote_count, subtree_names, parent_label) = client.read().with(|b| {
                let subtree_names = b
                    .resources_affected_by_delete(&id, true)
                    .iter()
                    .map(|r| {
                        r.resolved_label(&b.taxonomy)
                            .unwrap_or_else(|| r.id.to_string())
                    })
                    .collect::<Vec<_>>();
                let parent_label = b
                    .taxonomy
                    .node(&id)
                    .and_then(|n| n.parent.as_ref())
                    .and_then(|p| b.taxonomy.node(p))
                    .map(|p| {
                        if p.label.is_empty() {
                            p.id.to_string()
                        } else {
                            p.label.clone()
                        }
                    });
                (
                    b.resources_affected_by_delete(&id, false).len(),
                    subtree_names,
                    parent_label,
                )
            });
            let total = subtree_names.len();
            let shown: Vec<String> = subtree_names.into_iter().take(AFFECTED_PREVIEW_CAP).collect();
            let overflow = total - shown.len();
            let impact = match (&parent_label, has_children) {
                (Some(p), true) => t_string!(
                    i18n,
                    impact_promote_parent,
                    promote = promote_count,
                    parent = p.clone(),
                    total = total,
                )
                .to_string(),
                (Some(p), false) => {
                    t_string!(i18n, impact_reparent, total = total, parent = p.clone()).to_string()
                }
                (None, true) => t_string!(
                    i18n,
                    impact_promote_root,
                    promote = promote_count,
                    total = total,
                )
                .to_string(),
                (None, false) => t_string!(i18n, impact_untag, total = total).to_string(),
            };
            let on_cancel = Callback::new(move |_| target.set(None));
            let label = id.to_string();
            let buttons = if has_children {
                let promote_id = id.clone();
                view! {
                    <button class="confirm-ok" on:click=move |_| apply(promote_id.clone(), false)>
                        {t!(i18n, promote_children)}
                    </button>
                    <button
                        class="confirm-ok confirm-danger"
                        on:click=move |_| apply(id.clone(), true)
                    >
                        {t!(i18n, delete_subtree)}
                    </button>
                }
                .into_any()
            } else {
                view! {
                    <button class="confirm-ok" on:click=move |_| apply(id.clone(), false)>
                        {t!(i18n, action_delete)}
                    </button>
                }
                .into_any()
            };
            view! {
                <ConfirmShell on_cancel=on_cancel>
                    <p class="confirm-text">{t!(i18n, confirm_delete_label, label = label.clone())}</p>
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
                                        .then(|| view! { <li>{t!(i18n, confirm_more, count = overflow)}</li> })}
                                </ul>
                            }
                        })}
                    <div class="confirm-btns">
                        <button class="confirm-cancel" on:click=move |_| target.set(None)>
                            {t!(i18n, action_cancel)}
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
    let i18n = use_i18n();
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
                            {t!(i18n, promote_children)}
                        </button>
                        <button
                            class="confirm-ok confirm-danger"
                            on:click=move |_| apply(id.clone(), true)
                        >
                            {t!(i18n, delete_subtree)}
                        </button>
                    </DeleteShell>
                }
                .into_any()
            } else {
                view! {
                    <DeleteShell label=text on_cancel=on_cancel>
                        <button class="confirm-ok" on:click=move |_| apply(id.clone(), false)>
                            {t!(i18n, action_delete)}
                        </button>
                    </DeleteShell>
                }
                .into_any()
            }
        })
    }
}
