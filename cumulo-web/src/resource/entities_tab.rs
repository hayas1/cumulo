use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::platform::Platform;
use crate::resource::{ResourceAttribute, ResourceId};
use crate::shared::settings_modal::SettingsEditFlow;
use crate::shared::ForestDeleteConfirm;
use cumulo_model::{Bipartite, Forest};

use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn EntitiesTab(client: Client, flow: SettingsEditFlow) -> impl IntoView {
    let bipartite = client.read();
    let delete_target = RwSignal::new(Option::<(ResourceId, bool)>::None);

    view! {
        <div class="resource-tab">
            <button
                class="resource-add-btn"
                on:click=move |_| flow.open_editor(Platform::new_resource())
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
                                        on:click=move |_| flow.open_editor(r_edit.clone())
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

        <ForestDeleteConfirm
            client=client
            select={|b: &mut Bipartite<ResourceAttribute, CategoryAttribute>| &mut b.catalog}
            target=delete_target
            label={move |id: &ResourceId| {
                bipartite
                    .with(|s| s.catalog.node(id).map(|r| r.display_label(&s.taxonomy)))
                    .unwrap_or_else(|| id.to_string())
            }}
        />
    }
}
