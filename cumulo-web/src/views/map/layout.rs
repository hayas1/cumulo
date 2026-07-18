use std::f64::consts::PI;

use cumulo_model::{Forest, Resource, Taxonomy};

use super::force::{Body, Simulation};
use super::lod::Lod;
use crate::category::{CategoryAttribute, CategoryId};
use crate::resource::{ResourceAttribute, ResourceId};

pub const DEFAULT_COLOR: &str = "#6b8099";

const BASELINE_LEAVES: usize = 3;
const TOP_MAX_RADIUS_FACTOR: f64 = 0.22;
const TOP_MIN_RADIUS: f64 = 60.0;
const TOP_ORBIT_FACTOR: f64 = 0.3;
const CHILD_MAX_RADIUS_FACTOR: f64 = 0.40;
const CHILD_MIN_RADIUS_FACTOR: f64 = 0.12;
const CHILD_MIN_RADIUS: f64 = 8.0;
const CHILD_ORBIT_FACTOR: f64 = 0.45;
const GOLDEN_ANGLE_DEG: f64 = 137.508;
const SATELLITE_SPREAD_FACTOR: f64 = 0.68;
const SATELLITE_PACKING: f64 = 0.28;
const RESOURCE_MIN_RADIUS: f64 = 4.0;
const RESOURCE_MAX_RADIUS: f64 = 10.0;
const RESOURCE_RADIUS_BASE: f64 = 2.5;
const RESOURCE_RADIUS_PER_FREQ: f64 = 0.7;

type Res = Resource<ResourceAttribute, CategoryAttribute>;

#[derive(Clone, Debug, PartialEq)]
pub enum PathSeg {
    Category(CategoryId),
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

#[derive(Clone, Debug)]
pub enum MapNode {
    Cluster(Cluster),
    Resource(ResourceNode),
}

#[derive(Clone, Debug)]
pub struct Cluster {
    pub key: PathSeg,
    pub label: String,
    pub color: String,
    pub axis: CategoryId,
    pub depth: usize,
    pub total_freq: f64,
    pub leaf_count: usize,
    pub sub_nodes: Vec<MapNode>,
    pub placement: Placement,
}

impl Cluster {
    pub fn drill_target(&self) -> Option<(CategoryId, CategoryId)> {
        match &self.key {
            PathSeg::Category(value) => Some((self.axis.clone(), value.clone())),
            PathSeg::Other => None,
        }
    }

    pub fn is_drillable(&self) -> bool {
        self.drill_target().is_some()
    }
}

#[derive(Clone, Debug)]
pub struct ResourceNode {
    pub id: ResourceId,
    pub label: String,
    pub color: String,
    pub freq: f64,
    pub placement: Placement,
}

impl MapNode {
    fn total_freq(&self) -> f64 {
        match self {
            MapNode::Cluster(c) => c.total_freq,
            MapNode::Resource(_) => 1.0,
        }
    }

    fn placement_mut(&mut self) -> &mut Placement {
        match self {
            MapNode::Cluster(c) => &mut c.placement,
            MapNode::Resource(n) => &mut n.placement,
        }
    }

    pub fn placement(&self) -> &Placement {
        match self {
            MapNode::Cluster(c) => &c.placement,
            MapNode::Resource(n) => &n.placement,
        }
    }

    pub fn collect_resource_ids(&self, out: &mut Vec<ResourceId>) {
        match self {
            MapNode::Resource(n) => out.push(n.id.clone()),
            MapNode::Cluster(c) => {
                for sub in &c.sub_nodes {
                    sub.collect_resource_ids(out);
                }
            }
        }
    }

    fn is_resource(&self) -> bool {
        matches!(self, MapNode::Resource(_))
    }

    fn max_resource_child_count(&self) -> usize {
        let MapNode::Cluster(c) = self else {
            return 0;
        };
        match c.sub_nodes.first() {
            None => 0,
            Some(MapNode::Resource(_)) => c.sub_nodes.len(),
            Some(_) => c
                .sub_nodes
                .iter()
                .map(MapNode::max_resource_child_count)
                .max()
                .unwrap_or(0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
    pub fn center(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
    fn union(self, other: Bounds) -> Bounds {
        Bounds {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

#[derive(Clone)]
pub struct Layout {
    pub tree: Vec<MapNode>,
    pub lod: Lod,
}

impl Layout {
    pub fn content_bounds(&self) -> Option<Bounds> {
        let mut acc: Option<Bounds> = None;
        for node in &self.tree {
            Self::accumulate_bounds(node, &mut acc);
        }
        acc
    }

    pub fn cluster_placement(&self, axis: &CategoryId, value: &CategoryId) -> Option<Placement> {
        Self::find_cluster(&self.tree, axis, value)
    }

    fn find_cluster(nodes: &[MapNode], axis: &CategoryId, value: &CategoryId) -> Option<Placement> {
        for node in nodes {
            let MapNode::Cluster(c) = node else { continue };
            if matches!(c.drill_target(), Some((a, v)) if &a == axis && &v == value) {
                return Some(c.placement);
            }
            if let Some(p) = Self::find_cluster(&c.sub_nodes, axis, value) {
                return Some(p);
            }
        }
        None
    }

    fn accumulate_bounds(node: &MapNode, acc: &mut Option<Bounds>) {
        let p = node.placement();
        let b = Bounds {
            min_x: p.x - p.r,
            min_y: p.y - p.r,
            max_x: p.x + p.r,
            max_y: p.y + p.r,
        };
        *acc = Some(match acc {
            Some(a) => a.union(b),
            None => b,
        });
        if let MapNode::Cluster(c) = node {
            for sub in &c.sub_nodes {
                Self::accumulate_bounds(sub, acc);
            }
        }
    }
}

pub struct LayoutEngine<'a> {
    taxonomy: &'a Taxonomy<CategoryAttribute>,
    zoom_axis: &'a CategoryId,
    width: f64,
    height: f64,
}

struct Item {
    id: ResourceId,
    label: String,
    color: String,
    freq: f64,
    path: Vec<PathSeg>,
}

impl<'a> LayoutEngine<'a> {
    pub fn new(
        taxonomy: &'a Taxonomy<CategoryAttribute>,
        zoom_axis: &'a CategoryId,
        width: f64,
        height: f64,
    ) -> Self {
        LayoutEngine {
            taxonomy,
            zoom_axis,
            width,
            height,
        }
    }

    pub fn build(&self, resources: &[Res]) -> Layout {
        let items: Vec<Item> = resources.iter().map(|r| self.to_item(r)).collect();

        let max_depth = items
            .iter()
            .map(|it| it.path.len())
            .max()
            .unwrap_or(1)
            .max(1);

        let mut tree = self.build_level(items, 0);
        let layout_scale = self.layout_top_level(&mut tree);

        Layout {
            tree,
            lod: Lod::new(max_depth, layout_scale),
        }
    }

    fn to_item(&self, r: &Res) -> Item {
        Item {
            id: r.id.clone(),
            label: r
                .resolved_label(self.taxonomy)
                .unwrap_or_else(|| r.id.to_string()),
            color: self.resource_color(r),
            freq: (r.attribute.freq.max(1)) as f64,
            path: self.zoom_path(r),
        }
    }

    fn zoom_path(&self, r: &Res) -> Vec<PathSeg> {
        let Some(leaf) = r.category(self.taxonomy, self.zoom_axis) else {
            return vec![PathSeg::Other];
        };
        let mut chain = self.taxonomy.ancestry(leaf);
        chain.pop();
        chain.reverse();
        chain.into_iter().map(PathSeg::Category).collect()
    }

    fn resource_color(&self, r: &Res) -> String {
        match r.category(self.taxonomy, self.zoom_axis) {
            Some(leaf) => self.category_color(leaf),
            None => DEFAULT_COLOR.to_string(),
        }
    }

    fn category_color(&self, id: &CategoryId) -> String {
        self.taxonomy
            .node(id)
            .and_then(|c| c.attribute.color)
            .map(|col| col.to_hex())
            .unwrap_or_else(|| DEFAULT_COLOR.to_string())
    }

    fn key_label(&self, key: &PathSeg) -> String {
        match key {
            PathSeg::Other => String::new(),
            PathSeg::Category(id) => match self.taxonomy.node(id) {
                Some(c) if !c.label.is_empty() => c.label.clone(),
                _ => id.to_string(),
            },
        }
    }

    fn key_color(&self, key: &PathSeg) -> String {
        match key {
            PathSeg::Other => DEFAULT_COLOR.to_string(),
            PathSeg::Category(id) => self.category_color(id),
        }
    }

    fn build_level(&self, items: Vec<Item>, level: usize) -> Vec<MapNode> {
        let mut leaves: Vec<Item> = Vec::new();
        let mut deeper: Vec<Item> = Vec::new();
        for it in items {
            if it.path.len() <= level {
                leaves.push(it);
            } else {
                deeper.push(it);
            }
        }

        let mut groups: Vec<(PathSeg, Vec<Item>)> = Vec::new();
        for it in deeper {
            let key = it.path[level].clone();
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, v)) => v.push(it),
                None => groups.push((key, vec![it])),
            }
        }

        let mut nodes: Vec<MapNode> = groups
            .into_iter()
            .map(|(key, group_items)| {
                let total_freq = group_items.iter().map(|it| it.freq).sum();
                let leaf_count = group_items.len();
                let sub_nodes = self.build_level(group_items, level + 1);
                MapNode::Cluster(Cluster {
                    label: self.key_label(&key),
                    color: self.key_color(&key),
                    key,
                    axis: self.zoom_axis.clone(),
                    depth: level,
                    total_freq,
                    leaf_count,
                    sub_nodes,
                    placement: Placement::default(),
                })
            })
            .collect();

        nodes.extend(leaves.into_iter().map(|it| {
            MapNode::Resource(ResourceNode {
                id: it.id,
                label: it.label,
                color: it.color,
                freq: it.freq,
                placement: Placement::default(),
            })
        }));

        nodes
    }

    fn layout_top_level(&self, nodes: &mut [MapNode]) -> f64 {
        if nodes.is_empty() {
            return 1.0;
        }

        let max_leaves = nodes
            .iter()
            .map(MapNode::max_resource_child_count)
            .max()
            .unwrap_or(0)
            .max(BASELINE_LEAVES) as f64;
        let leaf_scale = (max_leaves / BASELINE_LEAVES as f64).sqrt();

        let (w, h) = (self.width, self.height);
        let min_wh = w.min(h);
        let max_freq = nodes.iter().map(MapNode::total_freq).fold(1.0, f64::max);
        let max_r = min_wh * TOP_MAX_RADIUS_FACTOR * leaf_scale;
        let min_r = TOP_MIN_RADIUS * leaf_scale;
        let orbit_r = min_wh * TOP_ORBIT_FACTOR * leaf_scale;

        let len = nodes.len() as f64;
        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = node.total_freq();
            let p = node.placement_mut();
            p.r = min_r + (max_r - min_r) * (freq / max_freq).sqrt();
            let angle = (i as f64 / len) * 2.0 * PI - PI / 2.0;
            p.x = w / 2.0 + angle.cos() * orbit_r;
            p.y = h / 2.0 + angle.sin() * orbit_r;
        }

        Self::simulate_forces(nodes, w / 2.0, h / 2.0, None);

        for node in nodes.iter_mut() {
            let (cx, cy, cr) = {
                let p = node.placement();
                (p.x, p.y, p.r)
            };
            if let MapNode::Cluster(c) = node {
                self.layout_children(&mut c.sub_nodes, cx, cy, cr);
            }
        }

        leaf_scale
    }

    fn layout_children(&self, nodes: &mut [MapNode], parent_x: f64, parent_y: f64, parent_r: f64) {
        if nodes.is_empty() {
            return;
        }
        if nodes[0].is_resource() {
            Self::layout_resource_nodes(nodes, parent_x, parent_y, parent_r);
            return;
        }

        let max_freq = nodes.iter().map(MapNode::total_freq).fold(1.0, f64::max);
        let max_r = parent_r * CHILD_MAX_RADIUS_FACTOR;
        let min_r = (parent_r * CHILD_MIN_RADIUS_FACTOR).max(CHILD_MIN_RADIUS);

        let len = nodes.len().max(1) as f64;
        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = node.total_freq();
            let p = node.placement_mut();
            let v = min_r + (max_r - min_r) * (freq / max_freq).sqrt();
            p.r = min_r.max(max_r.min(v));
            let angle = (i as f64 / len) * 2.0 * PI;
            p.x = parent_x + angle.cos() * parent_r * CHILD_ORBIT_FACTOR;
            p.y = parent_y + angle.sin() * parent_r * CHILD_ORBIT_FACTOR;
        }

        Self::simulate_forces(nodes, parent_x, parent_y, Some(parent_r));

        for node in nodes.iter_mut() {
            let (cx, cy, cr) = {
                let p = node.placement();
                (p.x, p.y, p.r)
            };
            if let MapNode::Cluster(c) = node {
                self.layout_children(&mut c.sub_nodes, cx, cy, cr);
            }
        }
    }

    fn layout_resource_nodes(nodes: &mut [MapNode], parent_x: f64, parent_y: f64, parent_r: f64) {
        let golden = GOLDEN_ANGLE_DEG.to_radians();
        let spread = parent_r * SATELLITE_SPREAD_FACTOR;

        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = match node {
                MapNode::Resource(n) => n.freq,
                _ => 1.0,
            };
            let base_r = RESOURCE_MIN_RADIUS.max(
                RESOURCE_MAX_RADIUS.min(freq * RESOURCE_RADIUS_PER_FREQ + RESOURCE_RADIUS_BASE),
            );
            let p = node.placement_mut();
            p.r = base_r;
            let angle = i as f64 * golden;
            let dist = (SATELLITE_PACKING * spread * ((i as f64) + 1.0).sqrt()).min(spread - p.r);
            p.x = parent_x + angle.cos() * dist.max(0.0);
            p.y = parent_y + angle.sin() * dist.max(0.0);
        }

        Self::simulate_forces(nodes, parent_x, parent_y, Some(parent_r));
    }

    fn simulate_forces(nodes: &mut [MapNode], cx: f64, cy: f64, bound_r: Option<f64>) {
        let bodies: Vec<Body> = nodes
            .iter()
            .map(|n| {
                let p = n.placement();
                Body::at(p.x, p.y, p.r)
            })
            .collect();
        let result = Simulation::new(bodies, cx, cy, bound_r).run();
        for (node, body) in nodes.iter_mut().zip(result) {
            let p = node.placement_mut();
            p.x = body.x;
            p.y = body.y;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::Color;
    use cumulo_model::{Catalog, Category, Id};

    fn cid(s: &str) -> CategoryId {
        s.try_into().unwrap()
    }
    fn rid(s: &str) -> ResourceId {
        s.try_into().unwrap()
    }

    fn cat(
        id: &str,
        label: &str,
        parent: Option<&str>,
        color: &str,
    ) -> Category<CategoryAttribute> {
        Category {
            id: cid(id),
            label: label.into(),
            parent: parent.map(cid),
            attribute: CategoryAttribute {
                color: Color::from_hex(color),
            },
        }
    }

    fn res(id: &str, categories: &[&str], freq: u32) -> Res {
        Resource {
            id: rid(id),
            label: None,
            parent: None,
            categories: categories.iter().map(|c| cid(c)).collect(),
            attribute: ResourceAttribute {
                console_url: String::new(),
                created_at: None,
                freq,
            },
        }
    }

    fn taxonomy() -> Taxonomy<CategoryAttribute> {
        Taxonomy(vec![
            cat("platform", "Platform", None, "#111111"),
            cat("gcp", "GCP", Some("platform"), "#22aa22"),
            cat("bigquery", "BigQuery", Some("gcp"), "#3333ff"),
            cat("bigtable", "Bigtable", Some("gcp"), "#33ccff"),
            cat("aws", "AWS", Some("platform"), "#ff9900"),
            cat("s3", "S3", Some("aws"), "#ee5555"),
        ])
    }

    #[test]
    fn zoom_path_excludes_axis_root_and_orders_top_down() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let r = res("r1", &["bigquery"], 1);
        assert_eq!(
            engine.zoom_path(&r),
            vec![
                PathSeg::Category(cid("gcp")),
                PathSeg::Category(cid("bigquery"))
            ]
        );
    }

    #[test]
    fn zoom_path_without_value_is_other() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let r = res("r1", &[], 1);
        assert_eq!(engine.zoom_path(&r), vec![PathSeg::Other]);
    }

    #[test]
    fn build_groups_resources_under_shared_ancestors() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let resources = vec![
            res("r1", &["bigquery"], 1),
            res("r2", &["bigtable"], 1),
            res("r3", &["s3"], 1),
        ];
        let layout = engine.build(&resources);
        let top_keys: Vec<&PathSeg> = layout
            .tree
            .iter()
            .filter_map(|n| match n {
                MapNode::Cluster(c) => Some(&c.key),
                _ => None,
            })
            .collect();
        assert!(top_keys.contains(&&PathSeg::Category(cid("gcp"))));
        assert!(top_keys.contains(&&PathSeg::Category(cid("aws"))));
        assert_eq!(top_keys.len(), 2);
    }

    #[test]
    fn cluster_carries_label_and_color() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[res("r1", &["bigquery"], 1)]);
        let MapNode::Cluster(gcp) = &layout.tree[0] else {
            panic!("expected cluster");
        };
        assert_eq!(gcp.label, "GCP");
        assert_eq!(gcp.color, "#22aa22");
        assert_eq!(gcp.leaf_count, 1);
    }

    #[test]
    fn build_assigns_finite_placement() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[res("r1", &["bigquery"], 1), res("r2", &["s3"], 1)]);
        for node in &layout.tree {
            let p = match node {
                MapNode::Cluster(c) => c.placement,
                MapNode::Resource(n) => n.placement,
            };
            assert!(p.x.is_finite() && p.y.is_finite());
            assert!(p.r > 0.0);
        }
    }

    #[test]
    fn cluster_drill_target_is_axis_and_value() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[res("r1", &["bigquery"], 1)]);
        let MapNode::Cluster(gcp) = &layout.tree[0] else {
            panic!("expected cluster");
        };
        assert_eq!(gcp.drill_target(), Some((cid("platform"), cid("gcp"))));
    }

    #[test]
    fn other_cluster_has_no_drill_target() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[res("r1", &[], 1)]);
        let MapNode::Cluster(other) = &layout.tree[0] else {
            panic!("expected cluster");
        };
        assert_eq!(other.key, PathSeg::Other);
        assert_eq!(other.drill_target(), None);
        assert!(!other.is_drillable());
    }

    #[test]
    fn category_cluster_is_drillable() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[res("r1", &["bigquery"], 1)]);
        let MapNode::Cluster(gcp) = &layout.tree[0] else {
            panic!("expected cluster");
        };
        assert!(gcp.is_drillable());
    }

    #[test]
    fn empty_resources_produce_empty_tree() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[]);
        assert!(layout.tree.is_empty());
    }

    #[test]
    fn works_with_catalog_slice() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let catalog = Catalog(vec![res("r1", &["bigquery"], 3)]);
        let layout = engine.build(&catalog);
        assert_eq!(layout.tree.len(), 1);
        let _ = Id::<()>::try_from("x");
    }
}
