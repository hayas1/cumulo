use crate::logic::facet::available_tags;
use crate::model::AppStore;
use leptos::*;

#[component]
pub fn Palette(
    store: ReadSignal<AppStore>,
    selected_tags: RwSignal<Vec<(String, String)>>,
) -> impl IntoView {
    let input_text = create_rw_signal(String::new());

    // 候補タグ: 現在の絞り込み後リソースから取得し、入力テキストでさらに絞る
    let suggestions = create_memo(move |_| {
        let s = store.get();
        let tags = selected_tags.get();
        let input = input_text.get();

        let mut avail = available_tags(&s.resources, &tags);

        if !input.is_empty() {
            let lower = input.to_lowercase();
            avail.retain(|(k, v)| {
                k.to_lowercase().contains(&lower) || v.to_lowercase().contains(&lower)
            });
        }

        avail.truncate(10);
        avail
    });

    view! {
        <div class="palette-bar">
            <div class="palette-input-row">
                // 選択済みタグをピルとして表示
                {move || {
                    selected_tags
                        .get()
                        .into_iter()
                        .map(|(k, v)| {
                            let k2 = k.clone();
                            let v2 = v.clone();
                            view! {
                                <span class="tag-pill">
                                    <span class="pill-key">{k.clone()}</span>
                                    <span class="pill-sep">":"</span>
                                    <span class="pill-val">{v.clone()}</span>
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
                <input
                    type="text"
                    class="palette-input"
                    placeholder="絞り込み... (例: service, auth)"
                    prop:value=move || input_text.get()
                    on:input=move |ev| {
                        input_text.set(event_target_value(&ev));
                    }
                />
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
            // 候補表示
            <Show when=move || suggestions.with(|s| !s.is_empty())>
                <div class="palette-suggestions">
                    <span class="suggestions-label">"候補:"</span>
                    {move || {
                        suggestions
                            .get()
                            .into_iter()
                            .map(|(k, v)| {
                                let k2 = k.clone();
                                let v2 = v.clone();
                                view! {
                                    <button
                                        class="suggestion-btn"
                                        on:click=move |_| {
                                            let k3 = k2.clone();
                                            let v3 = v2.clone();
                                            selected_tags
                                                .update(|t| {
                                                    t.retain(|(tk, _)| tk != &k3);
                                                    t.push((k3, v3));
                                                });
                                            input_text.set(String::new());
                                        }
                                    >
                                        <span class="sug-key">{k}</span>
                                        ":"
                                        <span class="sug-val">{v}</span>
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
