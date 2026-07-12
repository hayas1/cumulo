use crate::category::CategoryAttribute;
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
