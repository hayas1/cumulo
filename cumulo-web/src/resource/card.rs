use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use cumulo_model::{Forest, Resource};
use icondata as icon;
use leptos::prelude::*;
use leptos_icons::Icon;

type Res = Resource<ResourceAttribute, CategoryAttribute>;
type Editing = RwSignal<Option<Res>>;

#[component]
pub fn ResourceCard(client: Client, resource: Res, editing: Editing) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let s = bipartite.get();

    let url = resource.attribute.console_url.clone();
    let name = resource
        .resolved_label(&s.taxonomy)
        .unwrap_or_else(|| resource.id.to_string());
    let chips = resource
        .rooted_nodes(&s.taxonomy)
        .into_iter()
        .map(|(k, v)| {
            let color = s
                .taxonomy
                .node(&v)
                .and_then(|n| n.attribute.color)
                .map(|c| c.to_hex())
                .unwrap_or_default();
            (k.to_string(), s.taxonomy.label_of(&v), color)
        })
        .collect::<Vec<_>>();
    let r_edit = resource.clone();

    view! {
        <div class="result-card">
            <div class="result-card-header">
                <span
                    class="result-name result-name-link"
                    on:click=move |_| Platform::open_url(&url)
                >
                    {name}
                </span>
                <div class="result-card-actions">
                    <button
                        class="result-edit-btn"
                        title=move || t_string!(i18n, action_edit)
                        on:click=move |_| editing.set(Some(r_edit.clone()))
                    >
                        <Icon icon=icon::HiPencilOutlineLg width="14" height="14" />
                    </button>
                </div>
            </div>
            <div class="result-value">
                {chips
                    .into_iter()
                    .map(|(k, label, color)| {
                        let style = if !color.is_empty() {
                            format!("border-color:{color};background:{color}1a")
                        } else {
                            String::new()
                        };
                        view! {
                            <span class="result-chip" style=style>
                                <span class="chip-k">{k}</span>
                                <span class="chip-sep">":"</span>
                                <span class="chip-v">{label}</span>
                            </span>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </div>
    }
}
