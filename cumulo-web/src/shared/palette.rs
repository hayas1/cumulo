use crate::category::CategoryId;
use crate::client::Client;
use crate::state::State;
use cumulo_model::{Forest, Selection};
use leptos::prelude::*;

/// 検索窓のキーボードフォーカス位置。3 ゾーンを空間的に移動する:
/// - `Input`: 検索入力
/// - `Pill(i)`: 選択中フィルタのピル（入力の左隣）。↓ ではなく ← で入る
/// - `Candidate(i)`: 下に並ぶ候補（入力の下）。↓ で入り ↑ で戻る
///
/// DOM フォーカスとは独立に持ち、遷移だけを純粋に扱えるようにする。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaletteFocus {
    Input,
    Pill(usize),
    Candidate(usize),
}

impl PaletteFocus {
    /// 下キー: 入力・ピルから候補一覧の先頭へ。候補内・候補がなければ留まる。
    fn down(self, candidate_count: usize) -> Self {
        match self {
            PaletteFocus::Candidate(_) => self,
            _ if candidate_count > 0 => PaletteFocus::Candidate(0),
            _ => self,
        }
    }

    /// 上キー: 候補一覧からは検索入力に戻る。それ以外は留まる。
    fn up(self) -> Self {
        match self {
            PaletteFocus::Candidate(_) => PaletteFocus::Input,
            other => other,
        }
    }

    /// 左キー: 入力からは末尾ピルへ入る。ピル・候補の上では1つ左へ（左端で留まる）。
    fn left(self, pill_count: usize) -> Self {
        match self {
            PaletteFocus::Input if pill_count > 0 => PaletteFocus::Pill(pill_count - 1),
            PaletteFocus::Input => PaletteFocus::Input,
            PaletteFocus::Pill(i) => PaletteFocus::Pill(i.saturating_sub(1)),
            PaletteFocus::Candidate(i) => PaletteFocus::Candidate(i.saturating_sub(1)),
        }
    }

    /// 右キー: ピル上では1つ右へ（末尾の右は検索入力に戻る）。候補上では1つ右へ（右端で留まる）。
    fn right(self, pill_count: usize, candidate_count: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if i + 1 < pill_count => PaletteFocus::Pill(i + 1),
            PaletteFocus::Pill(_) => PaletteFocus::Input,
            PaletteFocus::Candidate(i) if i + 1 < candidate_count => PaletteFocus::Candidate(i + 1),
            PaletteFocus::Candidate(i) => PaletteFocus::Candidate(i),
            PaletteFocus::Input => PaletteFocus::Input,
        }
    }

    /// 選択中ピルを削除した後のフォーカス。`remaining` は削除後のピル数。
    fn after_removal(self, remaining: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if remaining > 0 => PaletteFocus::Pill(i.min(remaining - 1)),
            _ => PaletteFocus::Input,
        }
    }

    /// 選択中のピルの位置（描画のハイライト判定に使う）。
    fn pill(self) -> Option<usize> {
        match self {
            PaletteFocus::Pill(i) => Some(i),
            _ => None,
        }
    }

    /// ハイライト中の候補の位置（描画のハイライト判定に使う）。
    fn candidate(self) -> Option<usize> {
        match self {
            PaletteFocus::Candidate(i) => Some(i),
            _ => None,
        }
    }
}

#[component]
pub fn Palette(client: Client, state: State) -> impl IntoView {
    let bipartite = client.read();
    let selected_tags = state.filters;
    let input_text = RwSignal::new(String::new());
    // キーボードでのフォーカス位置（検索入力 / 選択中フィルタのピル / 下の候補）
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
        cursor.set(PaletteFocus::Input);
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
                        on:input=move |ev| {
                            input_text.set(event_target_value(&ev));
                            // 文字入力に戻ったらゾーン移動は解除し検索入力へ
                            cursor.set(PaletteFocus::Input);
                        }
                        on:keydown=move |ev| {
                            let pills_len = selected_tags.with(|t| t.iter().count());
                            let cand_len = suggestions.with(|s| s.len());
                            let empty_input = input_text.with(|t| t.is_empty());
                            let cur = cursor.get_untracked();
                            match (cur, ev.key().as_str()) {
                                // 検索入力: ↓ で候補へ。空のとき ← でピルへ（文字編集を邪魔しない）
                                (PaletteFocus::Input, "ArrowDown") if cand_len > 0 => {
                                    ev.prevent_default();
                                    cursor.set(cur.down(cand_len));
                                }
                                (PaletteFocus::Input, "ArrowLeft") if empty_input && pills_len > 0 => {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                }
                                // ピル: ←→ で移動（→ 末尾で入力へ戻る）、Backspace/Delete/Enter で削除
                                (PaletteFocus::Pill(_), "ArrowLeft") => {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                }
                                (PaletteFocus::Pill(_), "ArrowRight") => {
                                    ev.prevent_default();
                                    cursor.set(cur.right(pills_len, cand_len));
                                }
                                (PaletteFocus::Pill(_), "ArrowDown") if cand_len > 0 => {
                                    ev.prevent_default();
                                    cursor.set(cur.down(cand_len));
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
                                }
                                // 候補: ←→ で移動、↑ で入力へ戻る、Enter で確定
                                (PaletteFocus::Candidate(_), "ArrowLeft") => {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                }
                                (PaletteFocus::Candidate(_), "ArrowRight") => {
                                    ev.prevent_default();
                                    cursor.set(cur.right(pills_len, cand_len));
                                }
                                (PaletteFocus::Candidate(_), "ArrowUp") => {
                                    ev.prevent_default();
                                    cursor.set(cur.up());
                                }
                                (PaletteFocus::Candidate(i), "Enter") => {
                                    if let Some((k, v)) = suggestions.with(|s| s.get(i).cloned()) {
                                        ev.prevent_default();
                                        commit_tag(k, v);
                                    }
                                }
                                (_, "Escape") => {
                                    ev.prevent_default();
                                    cursor.set(PaletteFocus::Input);
                                }
                                _ => {}
                            }
                        }
                    />
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

            // 候補一覧（常時表示）。↓↑ でハイライト移動、Enter かクリックで確定
            <Show when=move || suggestions.with(|s| !s.is_empty())>
                <div class="palette-suggestions">
                    <span class="suggestions-label">"候補:"</span>
                    {move || {
                        suggestions
                            .get()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (k, v))| {
                                let key = k.clone();
                                let val = v.clone();
                                let key2 = key.clone();
                                let val2 = val.clone();
                                view! {
                                    <button
                                        class="suggestion-btn"
                                        class:focused=move || cursor.get().candidate() == Some(i)
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
    use super::PaletteFocus::{self, Candidate, Input, Pill};

    // 下キー: 入力・ピルから候補先頭へ。候補がなければ留まる
    #[test]
    fn down_enters_candidates_from_input_and_pill() {
        assert_eq!(Input.down(3), Candidate(0));
        assert_eq!(Pill(1).down(3), Candidate(0));
        assert_eq!(Input.down(0), Input);
        // 候補内では下キーで留まる
        assert_eq!(Candidate(1).down(3), Candidate(1));
    }

    // 上キー: 候補からは入力へ。それ以外は留まる
    #[test]
    fn up_returns_to_input_from_candidates() {
        assert_eq!(Candidate(2).up(), Input);
        assert_eq!(Pill(1).up(), Pill(1));
        assert_eq!(Input.up(), Input);
    }

    // 左キー: 入力からは末尾ピルに入る。ピルがなければ入力のまま
    #[test]
    fn left_enters_last_pill_from_input() {
        assert_eq!(Input.left(3), Pill(2));
        assert_eq!(Input.left(0), Input);
    }

    // 左キー: ピル・候補の上では1つ左へ。左端では留まる
    #[test]
    fn left_moves_within_zone_and_stops_at_head() {
        assert_eq!(Pill(2).left(3), Pill(1));
        assert_eq!(Pill(0).left(3), Pill(0));
        assert_eq!(Candidate(2).left(3), Candidate(1));
        assert_eq!(Candidate(0).left(3), Candidate(0));
    }

    // 右キー: ピルは末尾の右で入力へ。候補は右端で留まる
    #[test]
    fn right_moves_within_zone_then_pills_return_to_input() {
        assert_eq!(Pill(0).right(3, 5), Pill(1));
        assert_eq!(Pill(2).right(3, 5), Input);
        assert_eq!(Candidate(0).right(3, 5), Candidate(1));
        assert_eq!(Candidate(4).right(3, 5), Candidate(4));
        assert_eq!(Input.right(3, 5), Input);
    }

    // 削除後: 残りがあれば範囲内にクランプ、なくなれば入力へ
    #[test]
    fn after_removal_clamps_or_returns_to_input() {
        assert_eq!(Pill(1).after_removal(2), Pill(1));
        assert_eq!(Pill(2).after_removal(2), Pill(1));
        assert_eq!(Pill(0).after_removal(0), Input);
    }

    #[test]
    fn pill_and_candidate_report_selected_index() {
        assert_eq!(Pill(1).pill(), Some(1));
        assert_eq!(Candidate(2).pill(), None);
        assert_eq!(Candidate(2).candidate(), Some(2));
        assert_eq!(Pill(1).candidate(), None);
        assert_eq!(PaletteFocus::Input.pill(), None);
    }
}
