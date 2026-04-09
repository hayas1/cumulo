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
                    // 軸セレクト（動的リスト）
                    {move || {
                        let axes = zoom_axes.get();
                        let dims = dimensions.get();
                        let can_remove = axes.len() > 1;
                        axes.into_iter()
                            .enumerate()
                            .map(|(i, current)| {
                                let dims_i = dims.clone();
                                view! {
                                    {if i > 0 {
                                        Some(view! { <span class="axis-arrow">"›"</span> })
                                    } else {
                                        None
                                    }}
                                    <span class="axis-item">
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
                                            {dims_i
                                                .into_iter()
                                                .map(|(id, label)| {
                                                    let sel = id == current;
                                                    view! {
                                                        <option value={id} selected=sel>
                                                            {label}
                                                        </option>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </select>
                                        {if can_remove {
                                            Some(
                                                view! {
                                                    <button
                                                        class="axis-remove"
                                                        title="この軸を削除"
                                                        on:click=move |_| {
                                                            zoom_axes.update(|a| { a.remove(i); });
                                                        }
                                                    >
                                                        "×"
                                                    </button>
                                                },
                                            )
                                        } else {
                                            None
                                        }}
                                    </span>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                    // ＋ボタン（未使用のdimがある間だけ表示）
                    {move || {
                        let axes = zoom_axes.get();
                        let dims = dimensions.get();
                        if axes.len() >= dims.len() {
                            return None;
                        }
                        let dims_clone = dims.clone();
                        Some(
                            view! {
                                <button
                                    class="axis-add"
                                    title="ズーム軸を追加"
                                    on:click=move |_| {
                                        let cur = zoom_axes.get();
                                        if let Some((id, _)) = dims_clone
                                            .iter()
                                            .find(|(id, _)| !cur.contains(id))
                                        {
                                            let new_id = id.clone();
                                            zoom_axes.update(|a| a.push(new_id));
                                        }
                                    }
                                >
                                    "＋"
                                </button>
                            },
                        )
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
