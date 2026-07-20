use super::sidebar::FacetSidebar;
use crate::category::CategoryAttribute;
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::query::QueryState;
use crate::resource::{ResourceAttribute, ResourceCard};
use cumulo_model::{Resource, Selection};
use leptos::prelude::*;

#[component]
pub fn FacetView(
    client: Client,
    state: RwSignal<QueryState>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let selected_tags = Memo::new(move |_| state.with(|q| q.filters.clone()));
    view! {
        <div class="facet-view">
            <div class="facet-body">
                <FacetSidebar client=client state=state />

                <main class="facet-results">
                    {move || {
                        let s = bipartite.get();
                        let tags = selected_tags.get();

                        let entities: Vec<_> =
                            s.filtered(&tags).items().iter().map(|r| (*r).clone()).collect();

                        if entities.is_empty() {
                            return view! {
                                <div class="facet-empty">
                                    {t!(i18n, facet_no_match)}
                                </div>
                            }
                            .into_any();
                        }

                        let entity_count = entities.len();
                        view! {
                            <div class="results-header-row">
                                <span class="results-count">{entity_count}</span>
                                <button
                                    class="add-resource-btn"
                                    on:click=move |_| editing.set(Some(Platform::new_resource()))
                                    title=move || t_string!(i18n, add_resource)
                                >
                                    "+"
                                </button>
                            </div>
                            <div class="results-list">
                                {entities
                                    .into_iter()
                                    .map(|r| {
                                        view! { <ResourceCard client=client resource=r editing=editing /> }
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }
                        .into_any()
                    }}
                </main>
            </div>
        </div>
    }
}
