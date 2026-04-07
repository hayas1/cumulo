use crate::logic::facet::filter_resources;
use crate::map_bridge;
use crate::model::AppStore;
use leptos::*;

#[component]
pub fn Controls(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
    zoom_axes: RwSignal<Vec<String>>,
    zoom_level: ReadSignal<u32>,
) -> impl IntoView {
    let dimensions = create_memo(move |_| {
        store
            .get()
            .dimensions
            .iter()
            .map(|d| (d.id.clone(), d.label.clone()))
            .collect::<Vec<_>>()
    });

    let resource_count = create_memo(move |_| {
        let s = store.get();
        let tags = selected_tags.get();
        filter_resources(&s.resources, &tags).len()
    });

    let total_count = create_memo(move |_| store.get().resources.len());

    view! {
        <div class="controls-bar">
            <div class="controls-left">
                <span class="controls-label">"ズーム軸:"</span>
                <div class="zoom-axes-row">
                    {move || {
                        (0usize..3)
                            .map(|i| {
                                let dims = dimensions.get();
                                let current = zoom_axes
                                    .with(|a| a.get(i).cloned().unwrap_or_default());
                                view! {
                                    {if i > 0 {
                                        Some(
                                            view! { <span class="axis-arrow">"›"</span> },
                                        )
                                    } else {
                                        None
                                    }}
                                    <select
                                        class="axis-select"
                                        on:change=move |ev| {
                                            let val = event_target_value(&ev);
                                            zoom_axes
                                                .update(|a| {
                                                    if let Some(slot) = a.get_mut(i) {
                                                        *slot = val;
                                                    }
                                                });
                                        }
                                    >
                                        {dims
                                            .into_iter()
                                            .map(|(id, label)| {
                                                let selected = id == current;
                                                view! {
                                                    <option value={id} selected=selected>
                                                        {label}
                                                    </option>
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </select>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                </div>
            </div>
            <div class="controls-right">
                <span class="level-badge">
                    "Lv." {move || zoom_level.get()}
                </span>
                <span class="resource-count">
                    {move || resource_count.get()}
                    " / "
                    {move || total_count.get()}
                    " 件"
                </span>
                <div class="zoom-buttons">
                    <button
                        class="zoom-btn"
                        title="ズームアウト"
                        on:click=|_| map_bridge::zoom_out()
                    >
                        "−"
                    </button>
                    <button
                        class="zoom-btn"
                        title="ズームイン"
                        on:click=|_| map_bridge::zoom_in()
                    >
                        "+"
                    </button>
                    <button
                        class="zoom-btn zoom-fit"
                        title="全体表示"
                        on:click=|_| map_bridge::zoom_to_fit()
                    >
                        "⊡"
                    </button>
                </div>
            </div>
        </div>
    }
}
