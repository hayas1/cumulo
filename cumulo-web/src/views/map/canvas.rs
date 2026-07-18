use cumulo_model::Bipartite;
use leptos::prelude::*;
use leptos::svg::Svg;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, PointerEvent, WheelEvent};

use super::layout::{Cluster, Layout, LayoutEngine, MapNode, Placement, ResourceNode};
use super::lod::Lod;
use super::zoom::{Pan, Transform, ZoomController};
use crate::category::{CategoryAttribute, Filters};
use crate::client::Client;
use crate::query::QueryState;
use crate::resource::{ResourceAttribute, ResourceId};

const MAX_LABEL_CHARS: usize = 12;

const RESOURCE_OPACITY_MATCH: f64 = 0.85;
const RESOURCE_OPACITY_DIM: f64 = 0.1;
const CLUSTER_FILL_OPACITY_MATCH: f64 = 0.16;
const CLUSTER_FILL_OPACITY_DIM: f64 = 0.03;
const CLUSTER_STROKE_OPACITY_MATCH: f64 = 1.0;
const CLUSTER_STROKE_OPACITY_DIM: f64 = 0.2;

const CLUSTER_LABEL_FS_DIVISOR_TOP: f64 = 4.0;
const CLUSTER_LABEL_FS_DIVISOR_SUB: f64 = 3.5;
const CLUSTER_LABEL_FS_MIN_TOP: f64 = 13.0;
const CLUSTER_LABEL_FS_MIN_SUB: f64 = 8.0;
const CLUSTER_COUNT_FS_TOP: f64 = 11.0;
const CLUSTER_COUNT_FS_SUB: f64 = 9.0;
const CLUSTER_COUNT_DY_OFFSET: f64 = 14.0;
const RESOURCE_LABEL_FS: f64 = 5.0;
const RESOURCE_LABEL_FS_MAX: f64 = 11.0;

#[derive(Clone, Copy)]
struct NodeRenderer {
    controller: ZoomController,
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_resource: RwSignal<Option<ResourceId>>,
    state: RwSignal<QueryState>,
    filters: Memo<Filters>,
    zoom_level: RwSignal<u32>,
    scale: Memo<f64>,
    lod: Lod,
}

impl NodeRenderer {
    fn nodes(&self, nodes: &[MapNode], parent: Option<Placement>) -> Vec<AnyView> {
        nodes
            .iter()
            .map(|node| match node {
                MapNode::Cluster(c) => self.cluster(c, parent),
                MapNode::Resource(n) => self.resource(n, parent),
            })
            .collect()
    }

    fn cluster(&self, c: &Cluster, parent: Option<Placement>) -> AnyView {
        let (px, py) = parent.map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        let transform = format!("translate({},{})", c.placement.x - px, c.placement.y - py);

        let depth = c.depth;
        let radius = c.placement.r;
        let color = c.color.clone();
        let scale = self.scale;
        let lod = self.lod;

        let mut desc_ids = Vec::new();
        for sub in &c.sub_nodes {
            sub.collect_resource_ids(&mut desc_ids);
        }
        let bipartite = self.bipartite;
        let tags = self.filters;

        let bg_fill_opacity = {
            let ids = desc_ids.clone();
            move || {
                let has = bipartite.with(|b| tags.with(|t| ids.iter().any(|id| b.matches(id, t))));
                if has {
                    CLUSTER_FILL_OPACITY_MATCH
                } else {
                    CLUSTER_FILL_OPACITY_DIM
                }
            }
        };
        let bg_stroke_opacity = {
            let ids = desc_ids;
            move || {
                let has = bipartite.with(|b| tags.with(|t| ids.iter().any(|id| b.matches(id, t))));
                if has {
                    CLUSTER_STROKE_OPACITY_MATCH
                } else {
                    CLUSTER_STROKE_OPACITY_DIM
                }
            }
        };

        let drill = c.drill_target();
        let bg_class = if c.is_drillable() {
            "cluster-bg drillable"
        } else {
            "cluster-bg"
        };
        let (abs_x, abs_y, abs_r) = (c.placement.x, c.placement.y, c.placement.r);
        let state = self.state;
        let zoom_level = self.zoom_level;
        let controller = self.controller;
        let on_click = move |ev: MouseEvent| {
            ev.stop_propagation();
            if let Some((axis, value)) = drill.clone() {
                state.update(|q| q.filters.set(axis, value));
            }
            controller.zoom_to_node(abs_x, abs_y, abs_r);
            zoom_level.set(1);
        };

        let label = c.label.clone();
        let leaf_count = c.leaf_count;
        let label_base_fs = if depth == 0 {
            (radius / CLUSTER_LABEL_FS_DIVISOR_TOP).max(CLUSTER_LABEL_FS_MIN_TOP)
        } else {
            (radius / CLUSTER_LABEL_FS_DIVISOR_SUB).max(CLUSTER_LABEL_FS_MIN_SUB)
        };
        let count_base_fs = if depth == 0 {
            CLUSTER_COUNT_FS_TOP
        } else {
            CLUSTER_COUNT_FS_SUB
        };
        let count_dy = label_base_fs / 2.0 + CLUSTER_COUNT_DY_OFFSET;

        let label_fs =
            move || Lod::text_font_size(label_base_fs, Lod::default_max_fs(), scale.get());
        let count_fs =
            move || Lod::text_font_size(count_base_fs, Lod::default_max_fs(), scale.get());
        let label_opacity = move || lod.cluster_label_opacity(depth, scale.get());
        let count_opacity = label_opacity;

        let group_visible = move || lod.cluster_visible(depth, scale.get());
        let group_opacity = move || if group_visible() { "1" } else { "0" };
        let group_pointer = move || if group_visible() { "auto" } else { "none" };

        let children = self.nodes(&c.sub_nodes, Some(c.placement));

        view! {
            <g
                class=format!("cluster cluster-d{depth}")
                transform=transform
                style:opacity=group_opacity
                style:pointer-events=group_pointer
            >
                <circle
                    class=bg_class
                    r=radius
                    fill=color.clone()
                    fill-opacity=bg_fill_opacity
                    stroke=color.clone()
                    stroke-opacity=bg_stroke_opacity
                    on:click=on_click
                />
                <text
                    class="cluster-label"
                    dy="0.2em"
                    fill=color.clone()
                    font-size=label_fs
                    style:opacity=move || label_opacity().to_string()
                >
                    {label}
                </text>
                <text
                    class="cluster-count"
                    dy=count_dy
                    fill=color
                    font-size=count_fs
                    style:opacity=move || count_opacity().to_string()
                >
                    {format!("{leaf_count} リソース")}
                </text>
                {children}
            </g>
        }
        .into_any()
    }

    fn resource(&self, n: &ResourceNode, parent: Option<Placement>) -> AnyView {
        let (px, py) = parent.map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        let transform = format!("translate({},{})", n.placement.x - px, n.placement.y - py);

        let radius = n.placement.r;
        let color = n.color.clone();
        let scale = self.scale;
        let lod = self.lod;
        let bipartite = self.bipartite;
        let tags = self.filters;

        let id_for_fill = n.id.clone();
        let circle_opacity = move || {
            if bipartite.with(|b| tags.with(|t| b.matches(&id_for_fill, t))) {
                RESOURCE_OPACITY_MATCH
            } else {
                RESOURCE_OPACITY_DIM
            }
        };

        let node_visible = move || lod.node_visible(scale.get());
        let node_opacity = move || if node_visible() { "1" } else { "0" };
        let node_pointer = move || if node_visible() { "auto" } else { "none" };

        let label_visible = move || lod.node_label_visible(scale.get());
        let label_opacity = move || if label_visible() { "1" } else { "0" };
        let label_fs =
            move || Lod::text_font_size(RESOURCE_LABEL_FS, RESOURCE_LABEL_FS_MAX, scale.get());

        let label_text = {
            let full = n.label.clone();
            if full.chars().count() > MAX_LABEL_CHARS {
                let head: String = full.chars().take(MAX_LABEL_CHARS - 1).collect();
                format!("{head}…")
            } else {
                full
            }
        };

        let id_for_click = n.id.clone();
        let selected_resource = self.selected_resource;
        let on_click = move |ev: MouseEvent| {
            ev.stop_propagation();
            selected_resource.set(Some(id_for_click.clone()));
        };

        view! {
            <g
                class="mini-node"
                transform=transform
                style:opacity=node_opacity
                style:pointer-events=node_pointer
                on:click=on_click
            >
                <circle
                    class="mini-node-circle"
                    r=radius
                    fill=color
                    fill-opacity=circle_opacity
                />
                <text class="node-label" font-size=label_fs style:opacity=label_opacity>
                    {label_text}
                </text>
            </g>
        }
        .into_any()
    }
}

#[component]
pub fn MapCanvas(
    client: Client,
    state: RwSignal<QueryState>,
    selected_resource: RwSignal<Option<ResourceId>>,
    zoom_level: RwSignal<u32>,
    controller: ZoomController,
    fit_action: Callback<()>,
) -> impl IntoView {
    let bipartite = client.read();
    let selected_tags = Memo::new(move |_| state.with(|q| q.filters.clone()));
    let zoom_axis = Memo::new(move |_| state.with(|q| q.zoom_axis.clone()));
    let scale = Memo::new(move |_| controller.transform.get().scale);

    let layout = RwSignal::new(Layout {
        tree: Vec::new(),
        lod: Lod::new(1, 1.0),
    });
    Effect::new(move |_| {
        let b = bipartite.get();
        let zd = zoom_axis
            .get()
            .unwrap_or_else(|| client.default_zoom_axis());
        let (w, h) = controller.viewport.get();
        let result = LayoutEngine::new(&b.taxonomy, &zd, w, h).build(&b.catalog);
        controller.content_bounds.set(result.content_bounds());
        layout.set(result);
    });

    let svg_ref = NodeRef::<Svg>::new();

    Effect::new(move |_| {
        request_animation_frame(move || {
            if let Some(el) = svg_ref.get_untracked() {
                let rect = el.get_bounding_client_rect();
                let w = if rect.width() > 0.0 {
                    rect.width()
                } else {
                    900.0
                };
                let h = if rect.height() > 0.0 {
                    rect.height()
                } else {
                    600.0
                };
                controller.viewport.set((w, h));
            }
            request_animation_frame(move || {
                let axis = zoom_axis
                    .get_untracked()
                    .unwrap_or_else(|| client.default_zoom_axis());
                let target = selected_tags.with_untracked(|t| t.get(&axis).cloned());
                let placement =
                    target.and_then(|v| layout.with_untracked(|l| l.cluster_placement(&axis, &v)));
                match placement {
                    Some(p) => {
                        controller.zoom_to_node(p.x, p.y, p.r);
                        zoom_level.set(1);
                    }
                    None => controller.zoom_to_fit(),
                }
            });
        });
    });

    let pan = RwSignal::new(Option::<Pan>::None);
    let did_drag = RwSignal::new(false);

    let on_pointer_down = move |ev: PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        pan.set(Some(Pan::begin(
            ev.client_x() as f64,
            ev.client_y() as f64,
            controller.transform.get_untracked(),
        )));
        did_drag.set(false);
    };

    let on_pointer_move = move |ev: PointerEvent| {
        if let Some(p) = pan.get_untracked() {
            let (x, y) = (ev.client_x() as f64, ev.client_y() as f64);
            if p.is_drag(x, y) && !did_drag.get_untracked() {
                did_drag.set(true);
                if let Some(target) = ev.current_target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let _ = el.set_pointer_capture(ev.pointer_id());
                    }
                }
            }
            if did_drag.get_untracked() {
                controller.set_immediate(p.transform_at(x, y));
            }
        }
    };

    let on_pointer_up = move |ev: PointerEvent| {
        if let Some(target) = ev.current_target() {
            if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                let _ = el.release_pointer_capture(ev.pointer_id());
            }
        }
        pan.set(None);
    };

    let on_wheel = move |ev: WheelEvent| {
        ev.prevent_default();
        let factor = Transform::wheel_factor(ev.delta_y(), ev.delta_mode(), ev.ctrl_key());
        controller.zoom_by(factor, ev.offset_x() as f64, ev.offset_y() as f64);
    };

    let on_background_click = move |_ev: MouseEvent| {
        if did_drag.get_untracked() {
            return;
        }
        fit_action.run(());
    };

    let zoom_transform = move || controller.transform.get().to_svg();

    view! {
        <div id="map-container">
            <svg
                node_ref=svg_ref
                id="main-svg"
                on:wheel=on_wheel
                on:pointerdown=on_pointer_down
                on:pointermove=on_pointer_move
                on:pointerup=on_pointer_up
                on:pointercancel=on_pointer_up
                on:click=on_background_click
            >
                <g class="zoom-group" transform=zoom_transform>
                    {move || {
                        let l = layout.get();
                        let renderer = NodeRenderer {
                            controller,
                            bipartite,
                            selected_resource,
                            state,
                            filters: selected_tags,
                            zoom_level,
                            scale,
                            lod: l.lod,
                        };
                        renderer.nodes(&l.tree, None)
                    }}
                </g>
            </svg>
        </div>
    }
}
