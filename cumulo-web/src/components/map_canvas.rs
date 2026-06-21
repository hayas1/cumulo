//! マップ可視化コンポーネント。d3.js / map.js を置き換え、レイアウト計算（[`crate::map`]）の
//! 結果を Leptos の view! で SVG として宣言的に描画する。ズーム/パンは [`ZoomController`] が担う。

use std::collections::HashSet;

use cumulo_model::Bipartite;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, PointerEvent, WheelEvent};

use crate::map::layout::{Cluster, Layout, LayoutEngine, MapNode, Placement, ResourceNode};
use crate::map::lod::Lod;
use crate::map::zoom::{Pan, Transform, ZoomController};
use crate::platform::{CategoryAttribute, CategoryId, Filters, ResourceAttribute, ResourceId};

/// レイアウト 1 ノードを描画する際に必要な共有状態。すべてシグナルなので `Copy`。
#[derive(Clone, Copy)]
struct NodeRenderer {
    controller: ZoomController,
    filtered: Memo<HashSet<ResourceId>>,
    selected_entity: RwSignal<Option<ResourceId>>,
    selected_tags: RwSignal<Filters>,
    zoom_level: RwSignal<u32>,
    lod: Lod,
}

impl NodeRenderer {
    /// ノード列を描画する。`parent` は入れ子変換のための親配置（トップレベルは None）。
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
        let tr = self.controller.transform;
        let lod = self.lod;

        // 配下リソース id（フィルタ濃淡用）
        let mut desc_ids = Vec::new();
        for sub in &c.sub_nodes {
            sub.collect_resource_ids(&mut desc_ids);
        }
        let filtered = self.filtered;

        // 背景円の塗り/枠は「配下にフィルタ一致があるか」で濃淡を変える
        let bg_fill = {
            let color = color.clone();
            let ids = desc_ids.clone();
            move || {
                let has = filtered.with(|s| ids.iter().any(|id| s.contains(id)));
                format!("{}{}", color, if has { "28" } else { "08" })
            }
        };
        let bg_stroke_opacity = {
            let ids = desc_ids;
            move || {
                let has = filtered.with(|s| ids.iter().any(|id| s.contains(id)));
                if has {
                    1.0
                } else {
                    0.2
                }
            }
        };

        // クリックの「意味」（ドリル先とフォーカス対象）は Cluster 側の純粋判定に委ね、
        // ここでは web_sys イベント → シグナルへの配線だけを行う。
        let drill = c.drill_target();
        let (abs_x, abs_y, abs_r) = (c.placement.x, c.placement.y, c.placement.r);
        let selected_tags = self.selected_tags;
        let zoom_level = self.zoom_level;
        let controller = self.controller;
        let on_click = move |ev: MouseEvent| {
            ev.stop_propagation();
            // 値ありクラスタはドリルダウン（軸へ値を反映）。Other はドリルしない。
            if let Some((axis, value)) = drill.clone() {
                selected_tags.update(|t| t.set(axis, value));
            }
            controller.zoom_to_node(abs_x, abs_y, abs_r);
            zoom_level.set(1);
        };

        // ラベル/件数のフォントとフェード
        let label = c.label.clone();
        let leaf_count = c.leaf_count;
        let label_base_fs = if depth == 0 {
            (radius / 4.0).max(13.0)
        } else {
            (radius / 3.5).max(8.0)
        };
        let count_base_fs = if depth == 0 { 11.0 } else { 9.0 };
        let count_dy = label_base_fs / 2.0 + 14.0;

        let label_fs = move || Lod::text_font_size(label_base_fs, Lod::default_max_fs(), tr.get().k);
        let count_fs = move || Lod::text_font_size(count_base_fs, Lod::default_max_fs(), tr.get().k);
        let label_opacity = move || lod.cluster_label_opacity(depth, tr.get().k);
        // ラベルと件数は同じフェード値を使う
        let count_opacity = label_opacity;

        let group_visible = move || lod.cluster_visible(depth, tr.get().k);
        let group_opacity = move || if group_visible() { "1" } else { "0" };
        let group_pointer = move || if group_visible() { "auto" } else { "none" };

        let stroke_width = if depth == 0 { 2.0 } else { 1.2 };
        let dash = if depth > 0 { "5,3" } else { "" };
        let font_weight = if depth == 0 { "700" } else { "600" };

        let children = self.nodes(&c.sub_nodes, Some(c.placement));

        view! {
            <g
                class=format!("cluster cluster-d{depth}")
                transform=transform
                style:opacity=group_opacity
                style:pointer-events=group_pointer
            >
                <circle
                    class="cluster-bg"
                    r=radius
                    fill=bg_fill
                    stroke=color.clone()
                    stroke-width=stroke_width
                    stroke-dasharray=dash
                    stroke-opacity=bg_stroke_opacity
                    style:cursor="pointer"
                    on:click=on_click
                />
                <text
                    class="cluster-label"
                    text-anchor="middle"
                    dy="0.2em"
                    fill=color.clone()
                    font-weight=font_weight
                    font-family="system-ui, sans-serif"
                    pointer-events="none"
                    font-size=label_fs
                    style:opacity=move || label_opacity().to_string()
                >
                    {label}
                </text>
                <text
                    class="cluster-count"
                    text-anchor="middle"
                    dy=count_dy
                    fill=color
                    fill-opacity="0.65"
                    font-family="system-ui, sans-serif"
                    pointer-events="none"
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
        let tr = self.controller.transform;
        let lod = self.lod;
        let filtered = self.filtered;

        let id_for_fill = n.id.clone();
        let circle_opacity =
            move || if filtered.with(|s| s.contains(&id_for_fill)) { 0.85 } else { 0.1 };

        let node_visible = move || lod.node_visible(tr.get().k);
        let node_opacity = move || if node_visible() { "1" } else { "0" };
        let node_pointer = move || if node_visible() { "auto" } else { "none" };

        let label_visible = move || lod.node_label_visible(tr.get().k);
        let label_opacity = move || if label_visible() { "1" } else { "0" };
        let label_fs = move || Lod::text_font_size(5.0, 11.0, tr.get().k);

        // 名前は円中央に表示。長い場合は 12 文字で切り詰める。
        let label_text = {
            let full = n.label.clone();
            if full.chars().count() > 12 {
                let head: String = full.chars().take(11).collect();
                format!("{head}…")
            } else {
                full
            }
        };

        let id_for_click = n.id.clone();
        let selected_entity = self.selected_entity;
        let on_click = move |ev: MouseEvent| {
            ev.stop_propagation();
            selected_entity.set(Some(id_for_click.clone()));
        };

        view! {
            <g
                class="mini-node"
                transform=transform
                style:opacity=node_opacity
                style:pointer-events=node_pointer
                style:cursor="pointer"
                on:click=on_click
            >
                <circle
                    class="mini-node-circle"
                    r=radius
                    fill=color
                    fill-opacity=circle_opacity
                    stroke="#0d1117"
                    stroke-width="1"
                />
                <text
                    class="node-label"
                    text-anchor="middle"
                    dominant-baseline="middle"
                    font-family="system-ui, sans-serif"
                    fill="#e6edf3"
                    pointer-events="none"
                    font-size=label_fs
                    style:opacity=label_opacity
                >
                    {label_text}
                </text>
            </g>
        }
        .into_any()
    }
}

#[component]
pub fn MapCanvas(
    bipartite: ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    selected_tags: RwSignal<Filters>,
    zoom_dim: RwSignal<CategoryId>,
    selected_entity: RwSignal<Option<ResourceId>>,
    zoom_level: RwSignal<u32>,
    controller: ZoomController,
    /// 全体表示（フィルタ解除込み）。背景クリックと「全体表示」ボタンで共有する。
    fit_action: Callback<()>,
) -> impl IntoView {
    // フィルタ一致リソース集合（円の不透明度に使う）
    let filtered = Memo::new(move |_| {
        let b = bipartite.get();
        let tags = selected_tags.get();
        b.filter_resources(&tags)
            .into_iter()
            .map(|r| r.id.clone())
            .collect::<HashSet<ResourceId>>()
    });

    // レイアウト（座標は filter 非依存。catalog / zoom_dim / viewport にのみ依存）
    let layout = RwSignal::new(Layout {
        tree: Vec::new(),
        lod: Lod::new(1, 1.0),
    });
    Effect::new(move |_| {
        let b = bipartite.get();
        let zd = zoom_dim.get();
        let (w, h) = controller.viewport.get();
        let result = LayoutEngine::new(&b.taxonomy, &zd, w, h).build(&b.catalog);
        controller.content_bounds.set(result.content_bounds());
        layout.set(result);
    });

    // 初回マウント時にビューポートを実測 → 全体表示。レイアウト確定後に行うため二段 rAF。
    Effect::new(move |_| {
        request_animation_frame(move || {
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(el) = doc.get_element_by_id("main-svg") {
                    let rect = el.get_bounding_client_rect();
                    let w = if rect.width() > 0.0 { rect.width() } else { 900.0 };
                    let h = if rect.height() > 0.0 { rect.height() } else { 600.0 };
                    controller.viewport.set((w, h));
                }
            }
            request_animation_frame(move || controller.zoom_to_fit());
        });
    });

    // パン状態。ドラッグ確定（しきい値超え）まではクリックとして扱い、背景クリックの誤発火を防ぐ。
    let pan = RwSignal::new(Option::<Pan>::None);
    let did_drag = RwSignal::new(false);

    let on_pointer_down = move |ev: PointerEvent| {
        if ev.button() != 0 {
            return;
        }
        // ここではキャプチャしない。pointerdown でキャプチャすると後続の click が
        // svg へ retarget され、クラスタ/リソースの on:click（ズームイン・選択）が失われる。
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
            // しきい値を超えて初めてドラッグ確定。その時点で初めてポインタをキャプチャする。
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

    // ホイールでカーソル位置を中心にズーム
    let on_wheel = move |ev: WheelEvent| {
        ev.prevent_default();
        let factor = Transform::wheel_factor(ev.delta_y(), ev.delta_mode(), ev.ctrl_key());
        let next = controller
            .transform
            .get_untracked()
            .scale_by_about(factor, ev.offset_x() as f64, ev.offset_y() as f64);
        controller.set_immediate(next);
    };

    // 背景クリック（ノードは stopPropagation するのでここには来ない）→ 全体表示
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
                            filtered,
                            selected_entity,
                            selected_tags,
                            zoom_level,
                            lod: l.lod,
                        };
                        renderer.nodes(&l.tree, None)
                    }}
                </g>
            </svg>
        </div>
    }
}
