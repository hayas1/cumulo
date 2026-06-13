use crate::platform::{CategoryId, CategoryValue, ResourceValue};
use cumulo_model::Bipartite;
use leptos::*;

#[component]
pub fn Palette(
    bipartite: ReadSignal<Bipartite<ResourceValue, CategoryValue>>,
    selected_tags: RwSignal<Vec<(CategoryId, CategoryId)>>,
) -> impl IntoView {
    let input_text = create_rw_signal(String::new());
    let focused_index = create_rw_signal(Option::<usize>::None);
    let is_focused = create_rw_signal(false);

    let suggestions = create_memo(move |_| {
        let s = bipartite.get();
        let input = input_text.get();

        let mut result: Vec<(CategoryId, CategoryId)> = s
            .category_view()
            .query(&input)
            .view
            .into_iter()
            .filter_map(|attr| Some((s.taxonomy.root_of(&attr.id)?, attr.id.clone())))
            .collect();
        result.truncate(10);
        result
    });

    let commit_tag = move |k: CategoryId, v: CategoryId| {
        selected_tags.update(|t| {
            t.retain(|(tk, _)| tk != &k);
            t.push((k, v));
        });
        input_text.set(String::new());
        focused_index.set(None);
    };

    // 入力中（フォーカスあり＋文字あり）かつ候補があるときだけポップアップを表示
    let show_popup = move || {
        is_focused.get()
            && !input_text.with(|t| t.is_empty())
            && suggestions.with(|s| !s.is_empty())
    };

    view! {
        <div class="palette-bar">
            <div class="palette-input-row">
                {move || {
                    selected_tags
                        .get()
                        .into_iter()
                        .map(|(k, v)| {
                            let k2 = k.clone();
                            let v2 = v.clone();
                            view! {
                                <span class="tag-pill">
                                    <span class="pill-key">{k.to_string()}</span>
                                    <span class="pill-sep">":"</span>
                                    <span class="pill-val">{v.to_string()}</span>
                                    <button
                                        class="pill-remove"
                                        on:click=move |_| {
                                            selected_tags
                                                .update(|t| {
                                                    t.retain(|(tk, tv)| !(tk == &k2 && tv == &v2))
                                                })
                                        }
                                    >
                                        "×"
                                    </button>
                                </span>
                            }
                        })
                        .collect::<Vec<_>>()
                }}

                <div class="palette-input-wrapper">
                    <input
                        type="text"
                        class="palette-input"
                        placeholder="絞り込み... (例: service, auth)"
                        prop:value=move || input_text.get()
                        on:focus=move |_| is_focused.set(true)
                        on:blur=move |_| is_focused.set(false)
                        on:input=move |ev| {
                            input_text.set(event_target_value(&ev));
                            focused_index.set(None);
                        }
                        on:keydown=move |ev| {
                            let count = suggestions.with(|s| s.len());
                            if count == 0 {
                                return;
                            }
                            match ev.key().as_str() {
                                "ArrowDown" => {
                                    ev.prevent_default();
                                    focused_index.update(|fi| {
                                        *fi = Some(match *fi {
                                            None => 0,
                                            Some(i) => (i + 1) % count,
                                        });
                                    });
                                }
                                "ArrowUp" => {
                                    ev.prevent_default();
                                    focused_index.update(|fi| {
                                        *fi = Some(match *fi {
                                            None | Some(0) => count - 1,
                                            Some(i) => i - 1,
                                        });
                                    });
                                }
                                "Enter" => {
                                    if let Some(idx) = focused_index.get_untracked() {
                                        if let Some((k, v)) =
                                            suggestions.with(|s| s.get(idx).cloned())
                                        {
                                            ev.prevent_default();
                                            commit_tag(k, v);
                                        }
                                    }
                                }
                                "Escape" => {
                                    focused_index.set(None);
                                    is_focused.set(false);
                                }
                                _ => {}
                            }
                        }
                    />
                    // 入力中のみ表示するポップアップ
                    <Show when=show_popup>
                        <div class="palette-popup">
                            {move || {
                                let fi = focused_index.get();
                                suggestions
                                    .get()
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, (k, v))| {
                                        let key = k.clone();
                                        let val = v.clone();
                                        let key2 = key.clone();
                                        let val2 = val.clone();
                                        let is_focused_item = fi == Some(i);
                                        view! {
                                            <button
                                                class=if is_focused_item {
                                                    "popup-item focused"
                                                } else {
                                                    "popup-item"
                                                }
                                                // mousedown で prevent_default → blur を防いで確定
                                                on:mousedown=move |ev| {
                                                    ev.prevent_default();
                                                    commit_tag(key2.clone(), val2.clone());
                                                }
                                            >
                                                <span class="sug-key">{key.to_string()}</span>
                                                <span class="sug-sep">":"</span>
                                                <span class="sug-val">{val.to_string()}</span>
                                            </button>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </div>
                    </Show>
                </div>

                <Show when=move || !selected_tags.with(|t| t.is_empty())>
                    <button
                        class="palette-clear-btn"
                        on:click=move |_| {
                            selected_tags.update(|t| t.clear());
                            input_text.set(String::new());
                        }
                    >
                        "クリア"
                    </button>
                </Show>
            </div>

            // 既存の横並びチップ（常時表示）
            <Show when=move || suggestions.with(|s| !s.is_empty())>
                <div class="palette-suggestions">
                    <span class="suggestions-label">"候補:"</span>
                    {move || {
                        suggestions
                            .get()
                            .into_iter()
                            .map(|(k, v)| {
                                let key = k.clone();
                                let val = v.clone();
                                let key2 = key.clone();
                                let val2 = val.clone();
                                view! {
                                    <button
                                        class="suggestion-btn"
                                        on:click=move |_| {
                                            commit_tag(key2.clone(), val2.clone());
                                        }
                                    >
                                        <span class="sug-key">{key.to_string()}</span>
                                        ":"
                                        <span class="sug-val">{val.to_string()}</span>
                                    </button>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                </div>
            </Show>
        </div>
    }
}
