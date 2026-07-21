use super::axis_facet::AxisFacet;
use crate::category::{CategoryAttribute, CategoryId, Filters, DEFAULT_COLOR};
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::{ResourceAttribute, ResourceCard};
use cumulo_model::{Category, Forest, Resource, Selection};
use leptos::prelude::*;
use std::collections::HashSet;

type Cat = Category<CategoryAttribute>;
type Axes = (CategoryId, CategoryId);
type Expanded = RwSignal<Option<CategoryId>>;
type Editing = RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>;

const TREE_INDENT_BASE_REM: f32 = 0.35;
const TREE_INDENT_STEP_REM: f32 = 0.85;

#[derive(Clone, PartialEq)]
struct CellSel {
    row: Option<(CategoryId, CategoryId)>,
    col: Option<(CategoryId, CategoryId)>,
}

impl CellSel {
    fn is_cell(&self, row_val: &CategoryId, col_val: &CategoryId) -> bool {
        self.row.as_ref().map(|(_, v)| v) == Some(row_val)
            && self.col.as_ref().map(|(_, v)| v) == Some(col_val)
    }

    fn compose(&self, base: &Filters, axes: Option<&Axes>) -> Filters {
        let mut filters = base.clone();
        if let Some((row_axis, col_axis)) = axes {
            filters.remove_root(row_axis);
            filters.remove_root(col_axis);
        }
        for (axis, value) in [&self.row, &self.col].into_iter().flatten() {
            filters.set(axis.clone(), value.clone());
        }
        filters
    }
}

#[component]
pub fn MatrixView(client: Client, state: RwSignal<QueryState>, editing: Editing) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let selection = RwSignal::new(Option::<CellSel>::None);
    let row_expanded: Expanded = RwSignal::new(None);
    let col_expanded: Expanded = RwSignal::new(None);

    let effective = Memo::new(move |_| {
        bipartite.with(|s| {
            let axis_of = |id: &CategoryId| {
                s.taxonomy.node(id).is_some() && !s.taxonomy.children_of(id).is_empty()
            };
            let roots: Vec<CategoryId> = s
                .taxonomy
                .roots()
                .iter()
                .filter(|r| !s.taxonomy.children_of(&r.id).is_empty())
                .map(|r| r.id.clone())
                .collect();
            let first = roots.first()?.clone();
            let (chosen_row, chosen_col) = state.with(|q| (q.row_axis.clone(), q.col_axis.clone()));
            let row = chosen_row.filter(|id| axis_of(id)).unwrap_or_else(|| first.clone());
            let col = chosen_col
                .filter(|id| axis_of(id))
                .unwrap_or_else(|| roots.get(1).cloned().unwrap_or(first));
            Some((row, col))
        })
    });

    Effect::new(move |_| {
        effective.with(|_| ());
        selection.set(None);
        row_expanded.set(None);
        col_expanded.set(None);
    });

    view! {
        <div class="matrix-view">
            <MatrixControls client=client state=state editing=editing />
            <div class="matrix-area">
                {move || {
                    let Some((row_axis, col_axis)) = effective.get() else {
                        return view! {
                            <div class="matrix-empty matrix-empty-full">{t!(i18n, matrix_empty)}</div>
                        }
                        .into_any();
                    };
                    view! {
                        <AxisFacet client=client state=state selected=row_axis.clone() is_row=true />
                        <AxisFacet client=client state=state selected=col_axis.clone() is_row=false />
                        <main class="matrix-main">
                            <div class="matrix-inner">
                                <div class="matrix-grid">
                                    <Grid
                                        client=client
                                        state=state
                                        selection=selection
                                        row_expanded=row_expanded
                                        col_expanded=col_expanded
                                        row_axis=row_axis
                                        col_axis=col_axis
                                    />
                                </div>
                            </div>
                        </main>
                        <CellPanel
                            client=client
                            state=state
                            selection=selection
                            effective=effective
                            editing=editing
                        />
                    }
                    .into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn Grid(
    client: Client,
    state: RwSignal<QueryState>,
    selection: RwSignal<Option<CellSel>>,
    row_expanded: Expanded,
    col_expanded: Expanded,
    row_axis: CategoryId,
    col_axis: CategoryId,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();

    view! {
        {move || {
            let row_axis = row_axis.clone();
            let col_axis = col_axis.clone();
            let s = bipartite.get();
            let filters = state.with(|q| q.filters.clone());
            let sel = selection.get();
            let row_open = row_expanded.get();
            let col_open = col_expanded.get();
            let expanded_rows: HashSet<CategoryId> = row_open.iter().cloned().collect();
            let expanded_cols: HashSet<CategoryId> = col_open.iter().cloned().collect();
            let row_root = s.taxonomy.root_or_self(&row_axis);
            let col_root = s.taxonomy.root_or_self(&col_axis);
            let pivot = s.tree_pivot(&row_axis, &col_axis, &expanded_rows, &expanded_cols, &filters);
            if pivot.rows.is_empty() || pivot.cols.is_empty() {
                return view! {
                    <div class="matrix-empty">{t!(i18n, matrix_empty)}</div>
                }
                .into_any();
            }

            let color = |c: &Cat| {
                c.attribute
                    .color
                    .map(|c| c.to_hex())
                    .unwrap_or_else(|| DEFAULT_COLOR.to_hex())
            };
            let parent_color = |open: &Option<CategoryId>| {
                open.as_ref()
                    .and_then(|id| s.taxonomy.node(id))
                    .map(color)
                    .unwrap_or_else(|| DEFAULT_COLOR.to_hex())
            };
            let row_parent_color = parent_color(&row_open);
            let col_parent_color = parent_color(&col_open);
            let indent = |depth: usize| {
                format!(
                    "padding-left:{}rem",
                    TREE_INDENT_BASE_REM + depth as f32 * TREE_INDENT_STEP_REM
                )
            };

            let mut max = 0;
            for r in &pivot.rows {
                for c in &pivot.cols {
                    max = max.max(pivot.count(&r.node.id, &c.node.id));
                }
            }

            let header_cells = pivot
                .cols
                .iter()
                .map(|c| {
                    let ca = col_root.clone();
                    let cv = c.node.id.clone();
                    let nested = c.depth > 0;
                    let col_style = if nested {
                        format!("{};border-top:3px solid {col_parent_color}", indent(c.depth))
                    } else {
                        indent(c.depth)
                    };
                    let chevron = (c.depth == 0 && c.has_children).then(|| {
                        let id = c.node.id.clone();
                        let open = col_open.as_ref() == Some(&c.node.id);
                        view! {
                            <button
                                class="matrix-tree-chevron"
                                class:open=open
                                on:click=move |_| {
                                    let id = id.clone();
                                    col_expanded.update(move |e| {
                                        *e = if e.as_ref() == Some(&id) {
                                            None
                                        } else {
                                            Some(id.clone())
                                        };
                                    });
                                }
                            >
                                {if open { "\u{25be}" } else { "\u{25b8}" }}
                            </button>
                        }
                    });
                    view! {
                        <th class="matrix-colhead" class:matrix-nested=nested style=col_style>
                            {chevron}
                            <button
                                class="matrix-head-btn"
                                on:click=move |_| {
                                    selection.set(Some(CellSel { row: None, col: Some((ca.clone(), cv.clone())) }))
                                }
                            >
                                <span class="matrix-swatch" style=format!("background:{}", color(c.node)) />
                                {c.node.display_label().to_string()}
                            </button>
                        </th>
                    }
                })
                .collect::<Vec<_>>();

            let body_rows = pivot
                .rows
                .iter()
                .map(|r| {
                    let row_color = color(r.node);
                    let nested = r.depth > 0;
                    let row_style = if nested {
                        format!("{};border-left:3px solid {row_parent_color}", indent(r.depth))
                    } else {
                        indent(r.depth)
                    };
                    let chevron = (r.depth == 0 && r.has_children).then(|| {
                        let id = r.node.id.clone();
                        let open = row_open.as_ref() == Some(&r.node.id);
                        view! {
                            <button
                                class="matrix-tree-chevron"
                                class:open=open
                                on:click=move |_| {
                                    let id = id.clone();
                                    row_expanded.update(move |e| {
                                        *e = if e.as_ref() == Some(&id) {
                                            None
                                        } else {
                                            Some(id.clone())
                                        };
                                    });
                                }
                            >
                                {if open { "\u{25be}" } else { "\u{25b8}" }}
                            </button>
                        }
                    });
                    let cells = pivot
                        .cols
                        .iter()
                        .map(|c| {
                            let n = pivot.count(&r.node.id, &c.node.id);
                            let alpha = if max > 0 {
                                0x22 + (n as f32 / max as f32 * 0xaa as f32) as u32
                            } else {
                                0
                            };
                            let style = if n > 0 {
                                format!("background:{row_color}{alpha:02x}")
                            } else {
                                String::new()
                            };
                            let selected = sel.as_ref().is_some_and(|s| s.is_cell(&r.node.id, &c.node.id));
                            let ra = row_root.clone();
                            let rv = r.node.id.clone();
                            let ca = col_root.clone();
                            let cv = c.node.id.clone();
                            view! {
                                <td
                                    class="matrix-cell"
                                    class:matrix-cell-zero=(n == 0)
                                    class:matrix-cell-sel=selected
                                    style=style
                                    on:click=move |_| {
                                        selection.set(Some(CellSel {
                                            row: Some((ra.clone(), rv.clone())),
                                            col: Some((ca.clone(), cv.clone())),
                                        }))
                                    }
                                >
                                    {n}
                                </td>
                            }
                        })
                        .collect::<Vec<_>>();
                    let ra = row_root.clone();
                    let rv = r.node.id.clone();
                    let rt_axis = row_root.clone();
                    let rt_val = r.node.id.clone();
                    view! {
                        <tr>
                            <th class="matrix-rowhead" class:matrix-nested=nested style=row_style>
                                {chevron}
                                <button
                                    class="matrix-head-btn"
                                    on:click=move |_| {
                                        selection.set(Some(CellSel { row: Some((ra.clone(), rv.clone())), col: None }))
                                    }
                                >
                                    <span class="matrix-swatch" style=format!("background:{row_color}") />
                                    {r.node.display_label().to_string()}
                                </button>
                            </th>
                            {cells}
                            <td
                                class="matrix-total matrix-total-btn"
                                on:click=move |_| {
                                    selection.set(Some(CellSel { row: Some((rt_axis.clone(), rt_val.clone())), col: None }))
                                }
                            >
                                {pivot.row_total(&r.node.id)}
                            </td>
                        </tr>
                    }
                })
                .collect::<Vec<_>>();

            let total_cells = pivot
                .cols
                .iter()
                .map(|c| {
                    let ca = col_root.clone();
                    let cv = c.node.id.clone();
                    view! {
                        <td
                            class="matrix-total matrix-total-btn"
                            on:click=move |_| {
                                selection.set(Some(CellSel { row: None, col: Some((ca.clone(), cv.clone())) }))
                            }
                        >
                            {pivot.col_total(&c.node.id)}
                        </td>
                    }
                })
                .collect::<Vec<_>>();

            view! {
                <table class="matrix-table">
                    <thead>
                        <tr>
                            <th class="matrix-corner" />
                            {header_cells}
                            <th class="matrix-colhead matrix-total">{t!(i18n, matrix_total)}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {body_rows}
                        <tr class="matrix-totals-row">
                            <th class="matrix-rowhead matrix-total">{t!(i18n, matrix_total)}</th>
                            {total_cells}
                            <td
                                class="matrix-total matrix-total-btn"
                                on:click=move |_| selection.set(Some(CellSel { row: None, col: None }))
                            >
                                {pivot.total()}
                            </td>
                        </tr>
                    </tbody>
                </table>
            }
            .into_any()
        }}
    }
}

#[component]
fn CellPanel(
    client: Client,
    state: RwSignal<QueryState>,
    selection: RwSignal<Option<CellSel>>,
    effective: Memo<Option<Axes>>,
    editing: Editing,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();

    view! {
        <aside class="matrix-detail">
            {move || {
                let Some(sel) = selection.get() else {
                    return view! {
                        <div class="matrix-detail-empty">{t!(i18n, matrix_pick_cell)}</div>
                    }
                    .into_any();
                };
                let s = bipartite.get();
                let axes = effective.get();
                let roots = axes
                    .as_ref()
                    .map(|(row, col)| (s.taxonomy.root_or_self(row), s.taxonomy.root_or_self(col)));
                let filters = sel.compose(&state.with(|q| q.filters.clone()), roots.as_ref());

                let parts: Vec<_> = [&sel.row, &sel.col]
                    .into_iter()
                    .flatten()
                    .map(|(_, v)| s.taxonomy.label_of(v))
                    .collect();
                let title = if parts.is_empty() {
                    t_string!(i18n, matrix_total).to_string()
                } else {
                    parts.join(" × ")
                };

                let resources: Vec<_> =
                    s.filtered(&filters).items().iter().map(|r| (*r).clone()).collect();
                let count = resources.len();
                let facet_filters = filters.clone();

                let list = if resources.is_empty() {
                    view! { <div class="matrix-detail-empty">{t!(i18n, facet_no_match)}</div> }
                        .into_any()
                } else {
                    resources
                        .into_iter()
                        .map(|r| {
                            view! { <ResourceCard client=client resource=r editing=editing /> }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                };

                view! {
                    <div class="matrix-detail-header">
                        <span class="matrix-detail-title">{title}</span>
                        <span class="matrix-detail-count">{count}</span>
                    </div>
                    <button
                        class="matrix-detail-facet"
                        on:click=move |_| {
                            let f = facet_filters.clone();
                            state.update(move |q| {
                                q.filters = f;
                                q.view = View::Facet;
                            });
                        }
                    >
                        {t!(i18n, matrix_open_facet)}
                    </button>
                    <div class="matrix-detail-list">{list}</div>
                }
                .into_any()
            }}
        </aside>
    }
}

#[component]
fn MatrixControls(client: Client, state: RwSignal<QueryState>, editing: Editing) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let filtered = Memo::new(move |_| {
        let filters = state.with(|q| q.filters.clone());
        bipartite.with(|s| s.filtered(&filters).len())
    });
    let total = Memo::new(move |_| bipartite.with(|s| s.catalog.len()));

    view! {
        <div class="controls-bar">
            <div class="controls-left"></div>
            <div class="controls-right">
                <button
                    class="add-resource-btn"
                    on:click=move |_| editing.set(Some(Platform::new_resource()))
                    title=move || t_string!(i18n, add_resource)
                >
                    "+"
                </button>
                <span class="resource-count">
                    {move || format!("{} / {}", filtered.get(), total.get())}
                </span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(s: &str) -> CategoryId {
        s.try_into().unwrap()
    }

    fn cell(row: (&str, &str), col: (&str, &str)) -> CellSel {
        CellSel {
            row: Some((cid(row.0), cid(row.1))),
            col: Some((cid(col.0), cid(col.1))),
        }
    }

    #[test]
    fn is_cell_matches_only_when_both_values_equal() {
        let sel = cell(("env", "prod"), ("platform", "gcp"));
        assert!(sel.is_cell(&cid("prod"), &cid("gcp")));
        assert!(!sel.is_cell(&cid("prod"), &cid("aws")));
        assert!(!sel.is_cell(&cid("stg"), &cid("gcp")));
    }

    #[test]
    fn is_cell_is_false_for_row_or_column_only_selection() {
        let row_only = CellSel {
            row: Some((cid("env"), cid("prod"))),
            col: None,
        };
        assert!(!row_only.is_cell(&cid("prod"), &cid("gcp")));
    }

    #[test]
    fn compose_sets_both_axes_for_a_cell_and_keeps_unrelated_base() {
        let axes = (cid("env"), cid("platform"));
        let base: Filters = [(cid("team"), cid("data"))].into_iter().collect();
        let f = cell(("env", "prod"), ("platform", "gcp")).compose(&base, Some(&axes));
        assert_eq!(f.get(&cid("team")), Some(&cid("data")));
        assert_eq!(f.get(&cid("env")), Some(&cid("prod")));
        assert_eq!(f.get(&cid("platform")), Some(&cid("gcp")));
    }

    #[test]
    fn compose_row_only_selection_clears_the_column_axis() {
        let axes = (cid("env"), cid("platform"));
        let base: Filters = [(cid("platform"), cid("aws"))].into_iter().collect();
        let sel = CellSel {
            row: Some((cid("env"), cid("prod"))),
            col: None,
        };
        let f = sel.compose(&base, Some(&axes));
        assert_eq!(f.get(&cid("env")), Some(&cid("prod")));
        assert_eq!(f.get(&cid("platform")), None);
    }

    #[test]
    fn compose_total_selection_clears_both_axes() {
        let axes = (cid("env"), cid("platform"));
        let base: Filters = [(cid("env"), cid("prod")), (cid("platform"), cid("gcp"))]
            .into_iter()
            .collect();
        let sel = CellSel {
            row: None,
            col: None,
        };
        let f = sel.compose(&base, Some(&axes));
        assert_eq!(f.get(&cid("env")), None);
        assert_eq!(f.get(&cid("platform")), None);
        assert!(f.is_empty());
    }
}
