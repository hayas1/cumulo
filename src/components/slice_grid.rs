use leptos::prelude::*;
use crate::model::{
    AppStore, Resource, Dimension, build_slice_grid, dimension_values, resolve_dimension,
};

#[component]
pub fn SliceGrid(
    store: ReadSignal<AppStore>,
    selected_resource: WriteSignal<Option<Resource>>,
) -> impl IntoView {
    // 軸セレクト用のシグナル
    let axis_x = RwSignal::new("vendor".to_string());
    let axis_y = RwSignal::new("env".to_string());
    let axis_z = RwSignal::new("category".to_string());
    let z_value = RwSignal::new(String::new());

    // axis_z が変わったら z_value をリセット
    Effect::new(move |_| {
        let _ = axis_z.get(); // track
        z_value.set(String::new());
    });

    // Z軸の全ユニーク値
    let z_values = Memo::new(move |_| {
        let s = store.get();
        let az = axis_z.get();
        if let Some(dim) = s.dimensions.iter().find(|d| d.id == az) {
            dimension_values(&s.resources, dim)
        } else {
            vec![]
        }
    });

    // z_value が空なら最初の値を使う
    let effective_z = Memo::new(move |_| {
        let zv = z_value.get();
        if zv.is_empty() {
            z_values.get().into_iter().next().unwrap_or_default()
        } else {
            zv
        }
    });

    // グリッドデータ
    let grid_data = Memo::new(move |_| {
        let s = store.get();
        let ax = axis_x.get();
        let ay = axis_y.get();
        let az = axis_z.get();
        let ez = effective_z.get();

        build_slice_grid(
            &s.resources,
            &s.dimensions,
            &ax,
            &ay,
            &az,
            &ez,
            &s.cube_config.filters,
        )
    });

    // X/Y軸のユニーク値
    let x_values = Memo::new(move |_| {
        let s = store.get();
        let ax = axis_x.get();
        if let Some(dim) = s.dimensions.iter().find(|d| d.id == ax) {
            dimension_values(&s.resources, dim)
        } else {
            vec![]
        }
    });

    let y_values = Memo::new(move |_| {
        let s = store.get();
        let ay = axis_y.get();
        if let Some(dim) = s.dimensions.iter().find(|d| d.id == ay) {
            dimension_values(&s.resources, dim)
        } else {
            vec![]
        }
    });

    let dim_options = Memo::new(move |_| {
        store.get()
            .dimensions
            .iter()
            .map(|d| (d.id.clone(), d.label.clone()))
            .collect::<Vec<_>>()
    });

    view! {
        <div class="slice-grid-container">
            // ツールバー: 軸セレクト
            <div class="grid-toolbar">
                <AxisSelect
                    label="X軸"
                    value=axis_x
                    options=dim_options
                    exclude=Signal::derive(move || vec![axis_y.get(), axis_z.get()])
                />
                <AxisSelect
                    label="Y軸"
                    value=axis_y
                    options=dim_options
                    exclude=Signal::derive(move || vec![axis_x.get(), axis_z.get()])
                />
                <AxisSelect
                    label="奥行き軸"
                    value=axis_z
                    options=dim_options
                    exclude=Signal::derive(move || vec![axis_x.get(), axis_y.get()])
                />
            </div>

            // グリッド本体
            <div class="grid-scroll-area">
                <table class="slice-table">
                    <thead>
                        <tr>
                            <th class="grid-corner">
                                {move || {
                                    let s = store.get();
                                    let ay = axis_y.get();
                                    s.dimensions.iter()
                                        .find(|d| d.id == ay)
                                        .map(|d| d.label.clone())
                                        .unwrap_or_default()
                                }} " \\ "
                                {move || {
                                    let s = store.get();
                                    let ax = axis_x.get();
                                    s.dimensions.iter()
                                        .find(|d| d.id == ax)
                                        .map(|d| d.label.clone())
                                        .unwrap_or_default()
                                }}
                            </th>
                            {move || x_values.get().into_iter().map(|xv| {
                                let cls = format!("grid-header vendor-{}", xv.to_lowercase());
                                view! {
                                    <th class=cls>
                                        {xv}
                                    </th>
                                }
                            }).collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let xvs = x_values.get();
                            let gd = grid_data.get();
                            y_values.get().into_iter().map(|yv| {
                                let yv2 = yv.clone();
                                let row_cells = xvs.clone().into_iter().map(|xv| {
                                    let key = (xv.clone(), yv.clone());
                                    let resources = gd.get(&key).cloned().unwrap_or_default();
                                    let has_resources = !resources.is_empty();
                                    let resources2 = resources.clone();
                                    let set_sel = selected_resource;

                                    view! {
                                        <td
                                            class=if has_resources { "grid-cell active" } else { "grid-cell empty" }
                                            on:click=move |_| {
                                                if has_resources {
                                                    set_sel.set(resources2.first().cloned());
                                                }
                                            }
                                        >
                                            {if has_resources {
                                                view! {
                                                    <div class="cell-content">
                                                        <span class="cell-count">{resources.len()}</span>
                                                        <div class="cell-names">
                                                            {resources.iter().map(|r| view! {
                                                                <div class="cell-name">{r.name.clone()}</div>
                                                            }).collect_view()}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span class="cell-empty-mark">&mdash;</span> }.into_any()
                                            }}
                                        </td>
                                    }
                                }).collect_view();

                                view! {
                                    <tr>
                                        <td class="grid-row-header">{yv2}</td>
                                        {row_cells}
                                    </tr>
                                }
                            }).collect_view()
                        }}
                    </tbody>
                </table>
            </div>

            // Z軸ピル（奥行き軸の値切り替え）
            <div class="z-pills">
                <span class="z-pills-label">
                    {move || {
                        let s = store.get();
                        let az = axis_z.get();
                        s.dimensions.iter()
                            .find(|d| d.id == az)
                            .map(|d| d.label.clone())
                            .unwrap_or_default()
                    }} ":"
                </span>
                {move || {
                    let ez = effective_z.get();
                    z_values.get().into_iter().map(|zv| {
                        let zv2 = zv.clone();
                        let is_active = zv == ez;
                        view! {
                            <button
                                class=if is_active { "z-pill active" } else { "z-pill" }
                                on:click=move |_| { z_value.set(zv2.clone()); }
                            >
                                {zv}
                            </button>
                        }
                    }).collect_view()
                }}
            </div>
        </div>
    }
}

#[component]
fn AxisSelect(
    label: &'static str,
    value: RwSignal<String>,
    options: Memo<Vec<(String, String)>>,
    exclude: Signal<Vec<String>>,
) -> impl IntoView {
    view! {
        <div class="axis-select">
            <label>{label} ":"</label>
            <select
                on:change=move |ev| {
                    value.set(event_target_value(&ev));
                }
            >
                {move || {
                    let current = value.get();
                    let ex = exclude.get();
                    options.get().into_iter()
                        .filter(|(id, _)| !ex.contains(id) || id == &current)
                        .map(|(id, lbl)| {
                            let selected = id == current;
                            view! {
                                <option value=id selected=selected>{lbl}</option>
                            }
                        }).collect_view()
                }}
            </select>
        </div>
    }
}
