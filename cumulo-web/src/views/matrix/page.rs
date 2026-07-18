use crate::category::{CategoryAttribute, CategoryId, DEFAULT_COLOR};
use crate::client::Client;
use crate::i18n::*;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use crate::resource::ResourceAttribute;
use crate::views::facet::sidebar::FacetSidebar;
use cumulo_model::{Category, Forest, Resource, Selection};
use leptos::prelude::*;

type Cat = Category<CategoryAttribute>;

#[component]
pub fn MatrixView(
    client: Client,
    state: RwSignal<QueryState>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();

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
                                    <AxisBar
                                        client=client
                                        state=state
                                        roots=roots
                                        row_axis=row_axis.clone()
                                        col_axis=col_axis.clone()
                                    />
                                    <div class="matrix-grid">
                                        <Grid
                                            client=client
                                            state=state
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
            </div>
        </div>
    }
}

#[component]
fn AxisBar(
    client: Client,
    state: RwSignal<QueryState>,
    roots: Vec<CategoryId>,
    row_axis: CategoryId,
    col_axis: CategoryId,
) -> impl IntoView {
    let i18n = use_i18n();
    let bipartite = client.read();
    let axis_label = move |id: &CategoryId| {
        bipartite.with(|s| {
            s.taxonomy
                .node(id)
                .map(|n| if n.label.is_empty() { id.to_string() } else { n.label.clone() })
                .unwrap_or_else(|| id.to_string())
        })
    };
    let options = |roots: &[CategoryId], selected: &CategoryId, label: &dyn Fn(&CategoryId) -> String| {
        roots
            .iter()
            .map(|id| {
                view! {
                    <option value=id.to_string() selected=id == selected>
                        {label(id)}
                    </option>
                }
            })
            .collect::<Vec<_>>()
    };
    let row_options = options(&roots, &row_axis, &axis_label);
    let col_options = options(&roots, &col_axis, &axis_label);

    view! {
        <div class="matrix-axis-bar">
            <label class="matrix-axis-pick">
                <span class="matrix-axis-name">{t!(i18n, matrix_rows)}</span>
                <select
                    class="matrix-axis-select"
                    on:change=move |ev| {
                        if let Ok(id) = CategoryId::try_from(event_target_value(&ev)) {
                            state.update(|q| q.row_axis = Some(id));
                        }
                    }
                >
                    {row_options}
                </select>
            </label>
            <span class="matrix-axis-x">"×"</span>
            <label class="matrix-axis-pick">
                <span class="matrix-axis-name">{t!(i18n, matrix_cols)}</span>
                <select
                    class="matrix-axis-select"
                    on:change=move |ev| {
                        if let Ok(id) = CategoryId::try_from(event_target_value(&ev)) {
                            state.update(|q| q.col_axis = Some(id));
                        }
                    }
                >
                    {col_options}
                </select>
            </label>
        </div>
    }
}

#[component]
fn Grid(
    client: Client,
    state: RwSignal<QueryState>,
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
            let pivot = s.pivot(&row_axis, &col_axis, &filters);
            if pivot.rows.is_empty() || pivot.cols.is_empty() {
                return view! {
                    <div class="matrix-empty">{t!(i18n, matrix_empty)}</div>
                }
                .into_any();
            }

            let label = |c: &Cat| if c.label.is_empty() { c.id.to_string() } else { c.label.clone() };
            let color = |c: &Cat| {
                c.attribute
                    .color
                    .map(|c| c.to_hex())
                    .unwrap_or_else(|| DEFAULT_COLOR.to_hex())
            };
            let drill = move |axis: CategoryId, value: CategoryId| {
                state.update(|q| {
                    q.filters.set(axis, value);
                    q.view = View::Facet;
                });
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
                            on:click=move |_| drill(ca.clone(), cv.clone())
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
                            let ra = row_axis.clone();
                            let rv = r.id.clone();
                            let ca = col_axis.clone();
                            let cv = c.id.clone();
                            view! {
                                <td
                                    class="matrix-cell"
                                    class:matrix-cell-zero=(n == 0)
                                    style=style
                                    on:click=move |_| {
                                        state.update(|q| {
                                            q.filters.set(ra.clone(), rv.clone());
                                            q.filters.set(ca.clone(), cv.clone());
                                            q.view = View::Facet;
                                        });
                                    }
                                >
                                    {n}
                                </td>
                            }
                        })
                        .collect::<Vec<_>>();
                    let ra = row_axis.clone();
                    let rv = r.id.clone();
                    view! {
                        <tr>
                            <th
                                class="matrix-rowhead matrix-head-btn"
                                on:click=move |_| drill(ra.clone(), rv.clone())
                            >
                                <span class="matrix-swatch" style=format!("background:{row_color}") />
                                {label(r)}
                            </th>
                            {cells}
                            <td class="matrix-total">{pivot.row_total(&r.id)}</td>
                        </tr>
                    }
                })
                .collect::<Vec<_>>();

            let total_cells = pivot
                .cols
                .iter()
                .map(|c| view! { <td class="matrix-total">{pivot.col_total(&c.id)}</td> })
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
                            <td class="matrix-total">{pivot.total()}</td>
                        </tr>
                    </tbody>
                </table>
            }
            .into_any()
        }}
    }
}

#[component]
fn MatrixControls(
    client: Client,
    state: RwSignal<QueryState>,
    editing: RwSignal<Option<Resource<ResourceAttribute, CategoryAttribute>>>,
) -> impl IntoView {
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
