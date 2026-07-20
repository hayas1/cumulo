use crate::category::{CategoryAttribute, CategoryId, Filters, DEFAULT_COLOR};
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::{ResourceAttribute, ResourceCard};
use crate::views::facet::sidebar::FacetSidebar;
use cumulo_model::{Category, Forest, Resource, Selection};
use leptos::prelude::*;

type Cat = Category<CategoryAttribute>;
type Axes = (CategoryId, CategoryId);
type Editing = RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>;

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

    let effective = Memo::new(move |_| {
        let roots: Vec<CategoryId> =
            bipartite.with(|s| s.taxonomy.roots().iter().map(|r| r.id.clone()).collect());
        let axes = (!roots.is_empty()).then(|| {
            let (chosen_row, chosen_col) = state.with(|q| (q.row_axis.clone(), q.col_axis.clone()));
            let resolve = |chosen: Option<CategoryId>, fallback: &CategoryId| {
                chosen
                    .filter(|id| roots.contains(id))
                    .unwrap_or_else(|| fallback.clone())
            };
            let row = resolve(chosen_row, &roots[0]);
            let col = resolve(chosen_col, roots.get(1).unwrap_or(&roots[0]));
            (row, col)
        });
        (roots, axes)
    });

    Effect::new(move |_| {
        effective.with(|_| ());
        selection.set(None);
    });

    view! {
        <div class="matrix-view">
            <MatrixControls client=client state=state editing=editing />
            <div class="matrix-area">
                <FacetSidebar client=client state=state />
                <main class="matrix-main">
                    {move || {
                        let (roots, axes) = effective.get();
                        let Some((row_axis, col_axis)) = axes else {
                            return view! {
                                <div class="matrix-inner">
                                    <div class="matrix-empty">{t!(i18n, matrix_empty)}</div>
                                </div>
                            }
                            .into_any();
                        };
                        view! {
                            <div class="matrix-inner">
                                <div class="matrix-panel">
                                    <div class="matrix-pick matrix-pick-cols">
                                        <AxisPicker
                                            client=client
                                            state=state
                                            roots=roots.clone()
                                            selected=col_axis.clone()
                                            is_row=false
                                        />
                                    </div>
                                    <div class="matrix-pick matrix-pick-rows">
                                        <AxisPicker
                                            client=client
                                            state=state
                                            roots=roots
                                            selected=row_axis.clone()
                                            is_row=true
                                        />
                                    </div>
                                    <div class="matrix-grid">
                                        <Grid
                                            client=client
                                            state=state
                                            selection=selection
                                            row_axis=row_axis
                                            col_axis=col_axis
                                        />
                                    </div>
                                </div>
                            </div>
                        }
                        .into_any()
                    }}
                </main>
                <CellPanel
                    client=client
                    state=state
                    selection=selection
                    effective=effective
                    editing=editing
                />
            </div>
        </div>
    }
}

#[component]
fn AxisPicker(
    client: Client,
    state: RwSignal<QueryState>,
    roots: Vec<CategoryId>,
    selected: CategoryId,
    is_row: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let axis_label = move |id: &CategoryId| bipartite.with(|s| s.taxonomy.label_of(id));
    let options = roots
        .iter()
        .map(|id| {
            view! {
                <option value=id.to_string() selected=id == &selected>
                    {axis_label(id)}
                </option>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <span class="matrix-axis-name">
            {if is_row { t!(i18n, matrix_rows).into_any() } else { t!(i18n, matrix_cols).into_any() }}
        </span>
        <select
            class="matrix-axis-select"
            on:change=move |ev| {
                if let Ok(id) = CategoryId::try_from(event_target_value(&ev)) {
                    state.update(move |q| {
                        if is_row {
                            q.row_axis = Some(id);
                        } else {
                            q.col_axis = Some(id);
                        }
                    });
                }
            }
        >
            {options}
        </select>
    }
}

#[component]
fn Grid(
    client: Client,
    state: RwSignal<QueryState>,
    selection: RwSignal<Option<CellSel>>,
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
            let pivot = s.pivot(&row_axis, &col_axis, &filters);
            if pivot.rows.is_empty() || pivot.cols.is_empty() {
                return view! {
                    <div class="matrix-empty">{t!(i18n, matrix_empty)}</div>
                }
                .into_any();
            }

            let label = |c: &Cat| c.display_label().to_string();
            let color = |c: &Cat| {
                c.attribute
                    .color
                    .map(|c| c.to_hex())
                    .unwrap_or_else(|| DEFAULT_COLOR.to_hex())
            };

            let mut max = 0;
            for r in &pivot.rows {
                for c in &pivot.cols {
                    max = max.max(pivot.count(&r.id, &c.id));
                }
            }

            let header_cells = pivot
                .cols
                .iter()
                .map(|c| {
                    let ca = col_axis.clone();
                    let cv = c.id.clone();
                    view! {
                        <th
                            class="matrix-colhead matrix-head-btn"
                            on:click=move |_| {
                                selection.set(Some(CellSel { row: None, col: Some((ca.clone(), cv.clone())) }))
                            }
                        >
                            <span class="matrix-swatch" style=format!("background:{}", color(c)) />
                            {label(c)}
                        </th>
                    }
                })
                .collect::<Vec<_>>();

            let body_rows = pivot
                .rows
                .iter()
                .map(|r| {
                    let row_color = color(r);
                    let cells = pivot
                        .cols
                        .iter()
                        .map(|c| {
                            let n = pivot.count(&r.id, &c.id);
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
                            let selected = sel.as_ref().is_some_and(|s| s.is_cell(&r.id, &c.id));
                            let ra = row_axis.clone();
                            let rv = r.id.clone();
                            let ca = col_axis.clone();
                            let cv = c.id.clone();
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
                    let ra = row_axis.clone();
                    let rv = r.id.clone();
                    let rt_axis = row_axis.clone();
                    let rt_val = r.id.clone();
                    view! {
                        <tr>
                            <th
                                class="matrix-rowhead matrix-head-btn"
                                on:click=move |_| {
                                    selection.set(Some(CellSel { row: Some((ra.clone(), rv.clone())), col: None }))
                                }
                            >
                                <span class="matrix-swatch" style=format!("background:{row_color}") />
                                {label(r)}
                            </th>
                            {cells}
                            <td
                                class="matrix-total matrix-total-btn"
                                on:click=move |_| {
                                    selection.set(Some(CellSel { row: Some((rt_axis.clone(), rt_val.clone())), col: None }))
                                }
                            >
                                {pivot.row_total(&r.id)}
                            </td>
                        </tr>
                    }
                })
                .collect::<Vec<_>>();

            let total_cells = pivot
                .cols
                .iter()
                .map(|c| {
                    let ca = col_axis.clone();
                    let cv = c.id.clone();
                    view! {
                        <td
                            class="matrix-total matrix-total-btn"
                            on:click=move |_| {
                                selection.set(Some(CellSel { row: None, col: Some((ca.clone(), cv.clone())) }))
                            }
                        >
                            {pivot.col_total(&c.id)}
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
    effective: Memo<(Vec<CategoryId>, Option<Axes>)>,
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
                let (_, axes) = effective.get();
                let filters = sel.compose(&state.with(|q| q.filters.clone()), axes.as_ref());

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
