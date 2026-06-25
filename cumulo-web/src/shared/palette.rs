use crate::category::{CategoryAttribute, CategoryId, Filters};
use crate::resource::ResourceAttribute;
use cumulo_model::{Bipartite, Forest, Selection};
use leptos::prelude::*;

/// 検索窓のキーボードフォーカス位置。検索入力（`Input`）と選択中フィルタのピル（`Pill(i)`）の
/// 間を左右キーで行き来する。DOM フォーカスとは独立に持ち、遷移だけを純粋に扱えるようにする。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaletteFocus {
    Input,
    Pill(usize),
}

impl PaletteFocus {
    /// 左キー: 入力からは末尾ピルへ入り、ピル上では1つ左へ（左端で留まる）。
    fn left(self, pill_count: usize) -> Self {
        match self {
            PaletteFocus::Input if pill_count > 0 => PaletteFocus::Pill(pill_count - 1),
            PaletteFocus::Input => PaletteFocus::Input,
            PaletteFocus::Pill(i) => PaletteFocus::Pill(i.saturating_sub(1)),
        }
    }

    /// 右キー: ピル上では1つ右へ。末尾ピルの右では検索入力に戻る。
    fn right(self, pill_count: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if i + 1 < pill_count => PaletteFocus::Pill(i + 1),
            _ => PaletteFocus::Input,
        }
    }

    /// 選択中ピルを削除した後のフォーカス。`remaining` は削除後のピル数。
    fn after_removal(self, remaining: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if remaining > 0 => PaletteFocus::Pill(i.min(remaining - 1)),
            _ => PaletteFocus::Input,
        }
    }

    /// このフォーカスが選択しているピルの位置（描画のハイライト判定に使う）。
    fn pill(self) -> Option<usize> {
        match self {
            PaletteFocus::Pill(i) => Some(i),
            PaletteFocus::Input => None,
        }
    }
}

#[component]
pub fn Palette(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
) -> impl IntoView {
    let input_text = RwSignal::new(String::new());
    let focused_index = RwSignal::new(Option::<usize>::None);
    let is_focused = RwSignal::new(false);
    // キーボードでのフォーカス位置（検索入力 ↔ 選択中フィルタのピル）
    let cursor = RwSignal::new(PaletteFocus::Input);

    let suggestions = Memo::new(move |_| {
        let s = bipartite.get();
        let input = input_text.get();

        let mut result: Vec<(CategoryId, CategoryId)> = s
            .category_selection()
            .query(&input)
            .items()
            .iter()
            .filter_map(|attr| Some((s.taxonomy.root_of(&attr.id)?, attr.id.clone())))
            .collect();
        result.truncate(10);
        result
    });

    let commit_tag = move |k: CategoryId, v: CategoryId| {
        // 1軸1フィルタ: その軸の値を設定（既存値は置換）
        selected_tags.update(|t| t.set(k, v));
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
                        .with(|f| f.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
                        .into_iter()
                        .enumerate()
                        .map(|(i, (k, v))| {
                            let root = k.clone();
                            view! {
                                <span
                                    class="tag-pill"
                                    class:focused=move || cursor.get().pill() == Some(i)
                                >
                                    <span class="pill-key">{k.to_string()}</span>
                                    <span class="pill-sep">":"</span>
                                    <span class="pill-val">{v.to_string()}</span>
                                    <button
                                        class="pill-remove"
                                        on:click=move |_| {
                                            // 根1つにつき値1つなので、その根を外せばこのピルが消える
                                            selected_tags.update(|t| t.remove_root(&root));
                                            cursor.set(PaletteFocus::Input);
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
                            // 文字入力に戻ったらピル選択は解除
                            cursor.set(PaletteFocus::Input);
                        }
                        on:keydown=move |ev| {
                            // 先にピル（選択中フィルタ）操作を処理する。候補が0件でも効くように
                            // サジェストの早期 return より前に置く。
                            let pills_len = selected_tags.with(|t| t.iter().count());
                            let cur = cursor.get_untracked();
                            match (cur, ev.key().as_str()) {
                                // 検索窓が空のときだけ左キーでピル選択に入る（文字編集を邪魔しない）
                                (PaletteFocus::Input, "ArrowLeft")
                                    if input_text.with(|t| t.is_empty()) && pills_len > 0 =>
                                {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                    return;
                                }
                                (PaletteFocus::Pill(_), "ArrowLeft") => {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                    return;
                                }
                                (PaletteFocus::Pill(_), "ArrowRight") => {
                                    ev.prevent_default();
                                    cursor.set(cur.right(pills_len));
                                    return;
                                }
                                (PaletteFocus::Pill(i), "Backspace" | "Delete" | "Enter") => {
                                    ev.prevent_default();
                                    selected_tags.update(|t| {
                                        let root = t.iter().nth(i).map(|(k, _)| k.clone());
                                        if let Some(root) = root {
                                            t.remove_root(&root);
                                        }
                                    });
                                    cursor.set(cur.after_removal(pills_len.saturating_sub(1)));
                                    return;
                                }
                                (PaletteFocus::Pill(_), "Escape") => {
                                    ev.prevent_default();
                                    cursor.set(PaletteFocus::Input);
                                    return;
                                }
                                _ => {}
                            }

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

#[cfg(test)]
mod tests {
    use super::PaletteFocus::{self, Input, Pill};

    // 左キー: 入力からは末尾ピルに入る。ピルがなければ入力のまま
    #[test]
    fn left_enters_last_pill_from_input() {
        assert_eq!(Input.left(3), Pill(2));
        assert_eq!(Input.left(0), Input);
    }

    // 左キー: ピル上では1つ左へ。左端では留まる
    #[test]
    fn left_moves_within_pills_and_stops_at_head() {
        assert_eq!(Pill(2).left(3), Pill(1));
        assert_eq!(Pill(0).left(3), Pill(0));
    }

    // 右キー: ピル上では1つ右へ。末尾の右では検索入力に戻る
    #[test]
    fn right_moves_within_pills_then_back_to_input() {
        assert_eq!(Pill(0).right(3), Pill(1));
        assert_eq!(Pill(2).right(3), Input);
        assert_eq!(Input.right(3), Input);
    }

    // 削除後: 残りがあれば範囲内にクランプ、なくなれば入力へ
    #[test]
    fn after_removal_clamps_or_returns_to_input() {
        assert_eq!(Pill(1).after_removal(2), Pill(1));
        assert_eq!(Pill(2).after_removal(2), Pill(1));
        assert_eq!(Pill(0).after_removal(0), Input);
    }

    #[test]
    fn pill_reports_selected_index() {
        assert_eq!(Pill(1).pill(), Some(1));
        assert_eq!(PaletteFocus::Input.pill(), None);
    }
}
