use crate::category::CategoryId;
use crate::client::Client;
use crate::query::QueryState;
use cumulo_model::{Forest, Selection};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaletteFocus {
    Input,
    Pill(usize),
    Candidate(usize),
}

impl PaletteFocus {
    fn down(self, candidate_count: usize) -> Self {
        match self {
            PaletteFocus::Candidate(_) => self,
            _ if candidate_count > 0 => PaletteFocus::Candidate(0),
            _ => self,
        }
    }

    fn up(self) -> Self {
        match self {
            PaletteFocus::Candidate(_) => PaletteFocus::Input,
            other => other,
        }
    }

    fn left(self, pill_count: usize) -> Self {
        match self {
            PaletteFocus::Input if pill_count > 0 => PaletteFocus::Pill(pill_count - 1),
            PaletteFocus::Input => PaletteFocus::Input,
            PaletteFocus::Pill(i) => PaletteFocus::Pill(i.saturating_sub(1)),
            PaletteFocus::Candidate(i) => PaletteFocus::Candidate(i.saturating_sub(1)),
        }
    }

    fn right(self, pill_count: usize, candidate_count: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if i + 1 < pill_count => PaletteFocus::Pill(i + 1),
            PaletteFocus::Pill(_) => PaletteFocus::Input,
            PaletteFocus::Candidate(i) if i + 1 < candidate_count => PaletteFocus::Candidate(i + 1),
            PaletteFocus::Candidate(i) => PaletteFocus::Candidate(i),
            PaletteFocus::Input => PaletteFocus::Input,
        }
    }

    fn after_removal(self, remaining: usize) -> Self {
        match self {
            PaletteFocus::Pill(i) if remaining > 0 => PaletteFocus::Pill(i.min(remaining - 1)),
            _ => PaletteFocus::Input,
        }
    }

    fn pill(self) -> Option<usize> {
        match self {
            PaletteFocus::Pill(i) => Some(i),
            _ => None,
        }
    }

    fn candidate(self) -> Option<usize> {
        match self {
            PaletteFocus::Candidate(i) => Some(i),
            _ => None,
        }
    }
}

#[component]
pub fn Palette(client: Client, state: RwSignal<QueryState>) -> impl IntoView {
    let bipartite = client.read();
    let filters = Memo::new(move |_| state.with(|q| q.filters.clone()));
    let input_text = RwSignal::new(String::new());
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
        state.update(|q| q.filters.set(k, v));
        input_text.set(String::new());
        cursor.set(PaletteFocus::Input);
    };

    view! {
        <div class="palette-bar">
            <div class="palette-input-row">
                {move || {
                    filters
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
                                            state.update(|q| q.filters.remove_root(&root));
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
                            cursor.set(PaletteFocus::Input);
                        }
                        on:keydown=move |ev| {
                            let pills_len = filters.with(|t| t.iter().count());
                            let cand_len = suggestions.with(|s| s.len());
                            let empty_input = input_text.with(|t| t.is_empty());
                            let cur = cursor.get_untracked();
                            match (cur, ev.key().as_str()) {
                                (PaletteFocus::Input, "ArrowDown") if cand_len > 0 => {
                                    ev.prevent_default();
                                    cursor.set(cur.down(cand_len));
                                }
                                (PaletteFocus::Input, "ArrowLeft") if empty_input && pills_len > 0 => {
                                    ev.prevent_default();
                                    cursor.set(cur.left(pills_len));
                                }
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
                                    state.update(|q| {
                                        let root = q.filters.iter().nth(i).map(|(k, _)| k.clone());
                                        if let Some(root) = root {
                                            q.filters.remove_root(&root);
                                        }
                                    });
                                    cursor.set(cur.after_removal(pills_len.saturating_sub(1)));
                                }
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

                <Show when=move || !filters.with(|t| t.is_empty())>
                    <button
                        class="palette-clear-btn"
                        on:click=move |_| {
                            state.update(|q| q.filters.clear());
                            input_text.set(String::new());
                        }
                    >
                        "クリア"
                    </button>
                </Show>
            </div>

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

    #[test]
    fn down_enters_candidates_from_input_and_pill() {
        assert_eq!(Input.down(3), Candidate(0));
        assert_eq!(Pill(1).down(3), Candidate(0));
        assert_eq!(Input.down(0), Input);
        assert_eq!(Candidate(1).down(3), Candidate(1));
    }

    #[test]
    fn up_returns_to_input_from_candidates() {
        assert_eq!(Candidate(2).up(), Input);
        assert_eq!(Pill(1).up(), Pill(1));
        assert_eq!(Input.up(), Input);
    }

    #[test]
    fn left_enters_last_pill_from_input() {
        assert_eq!(Input.left(3), Pill(2));
        assert_eq!(Input.left(0), Input);
    }

    #[test]
    fn left_moves_within_zone_and_stops_at_head() {
        assert_eq!(Pill(2).left(3), Pill(1));
        assert_eq!(Pill(0).left(3), Pill(0));
        assert_eq!(Candidate(2).left(3), Candidate(1));
        assert_eq!(Candidate(0).left(3), Candidate(0));
    }

    #[test]
    fn right_moves_within_zone_then_pills_return_to_input() {
        assert_eq!(Pill(0).right(3, 5), Pill(1));
        assert_eq!(Pill(2).right(3, 5), Input);
        assert_eq!(Candidate(0).right(3, 5), Candidate(1));
        assert_eq!(Candidate(4).right(3, 5), Candidate(4));
        assert_eq!(Input.right(3, 5), Input);
    }

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
