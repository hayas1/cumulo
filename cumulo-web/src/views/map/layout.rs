//! リソース群をズーム軸の階層クラスタへ配置するレイアウト計算。
//!
//! map.js の buildLevel / layoutTopLevel / layoutChildren / layoutResourceNodes と
//! ラベル・色の導出を移植する。座標はすべて「ルート座標系の絶対値」で持ち、
//! 入れ子描画時の相対変換は描画層（view）が行う。

use std::f64::consts::PI;

use cumulo_model::{Forest, Resource, Taxonomy};

use super::force::{Body, Simulation};
use super::lod::Lod;
use crate::platform::{CategoryAttribute, CategoryId, ResourceAttribute, ResourceId};

/// 値が見つからないノードの既定色。
pub const DEFAULT_COLOR: &str = "#6b8099";
/// ズーム軸に値を持たないリソースを集約するクラスタの表示名。
pub const OTHER_LABEL: &str = "その他";

type Res = Resource<ResourceAttribute, CategoryAttribute>;

/// ズーム軸パスの 1 セグメント。値なしリソースは [`PathSeg::Other`] に集約される。
#[derive(Clone, Debug, PartialEq)]
pub enum PathSeg {
    Category(CategoryId),
    Other,
}

/// 絶対座標での配置（中心 x,y と半径 r）。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

/// レイアウト済みのノード木。クラスタ（入れ子）かリソース（葉）のいずれか。
#[derive(Clone, Debug)]
pub enum MapNode {
    Cluster(Cluster),
    Resource(ResourceNode),
}

/// 同一ズーム軸値でまとめたクラスタ。
#[derive(Clone, Debug)]
pub struct Cluster {
    /// グルーピングキー。ドリル可否の判定に使う（Other はドリル対象外）。
    pub key: PathSeg,
    pub label: String,
    pub color: String,
    /// ドリル先の軸（zoom_dim）。
    pub axis: CategoryId,
    pub depth: usize,
    pub total_freq: f64,
    /// 配下のリソース総数（"N リソース" 表示用）。
    pub leaf_count: usize,
    pub sub_nodes: Vec<MapNode>,
    pub placement: Placement,
}

impl Cluster {
    /// このクラスタをクリックしたときにドリルダウンすべき (軸, 値)。
    /// 値を持たない Other クラスタはドリル対象外なので None。
    /// 「クリックが何を意味するか」はイベント型やビュー状態に依存しない純粋な判定なので、
    /// 描画クロージャではなくレイアウト層（ここ）に置きテスト可能にする。
    pub fn drill_target(&self) -> Option<(CategoryId, CategoryId)> {
        match &self.key {
            PathSeg::Category(value) => Some((self.axis.clone(), value.clone())),
            PathSeg::Other => None,
        }
    }
}

/// 葉となる 1 リソース。
#[derive(Clone, Debug)]
pub struct ResourceNode {
    pub id: ResourceId,
    pub label: String,
    pub color: String,
    /// 衛星円の半径計算に使う出現頻度（最低 1）。
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

    /// このノード以下のリソース id をすべて集める（クラスタ背景のフィルタ濃淡判定用）。
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

    /// このサブツリーで、直下にリソースを最も多く持つクラスタの子数（layoutScale 用の B）。
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

/// 内容を囲む軸平行バウンディングボックス（ルート絶対座標）。
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
        ((self.min_x + self.max_x) / 2.0, (self.min_y + self.max_y) / 2.0)
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

/// レイアウト結果。木と、LOD 計算に必要な最大深さ・レイアウト倍率を返す。
#[derive(Clone)]
pub struct Layout {
    pub tree: Vec<MapNode>,
    pub lod: Lod,
}

impl Layout {
    /// 全ノードの円を囲む内容バウンディングボックス。空なら None。
    /// getBBox 相当をレイアウト座標から直接求めるため DOM 測定が不要になる。
    pub fn content_bounds(&self) -> Option<Bounds> {
        let mut acc: Option<Bounds> = None;
        for node in &self.tree {
            Self::accumulate_bounds(node, &mut acc);
        }
        acc
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

/// レイアウト計算の入口。taxonomy / zoom_dim / キャンバス寸法を固定して resources を配置する。
pub struct LayoutEngine<'a> {
    taxonomy: &'a Taxonomy<CategoryAttribute>,
    zoom_dim: &'a CategoryId,
    width: f64,
    height: f64,
}

/// 構築途中のリソース項目。木構築で消費する。
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
        zoom_dim: &'a CategoryId,
        width: f64,
        height: f64,
    ) -> Self {
        LayoutEngine {
            taxonomy,
            zoom_dim,
            width,
            height,
        }
    }

    /// resources を木へ構築し、座標を確定して返す。
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

    /// 1 リソースを Item（パス・ラベル・色・freq）へ変換する。
    fn to_item(&self, r: &Res) -> Item {
        Item {
            id: r.id.clone(),
            label: r.display_label(self.taxonomy),
            color: self.resource_color(r),
            freq: (r.attribute.freq.max(1)) as f64,
            path: self.zoom_path(r),
        }
    }

    /// ズーム軸でのフォレスト根直下から葉までのパス（上→下）。値なしは [Other]。
    fn zoom_path(&self, r: &Res) -> Vec<PathSeg> {
        let Some(leaf) = r.category(self.taxonomy, self.zoom_dim) else {
            return vec![PathSeg::Other];
        };
        // ancestry は [leaf, .., root(=zoom_dim)]。根（軸自身）を除いて上→下へ並べ替える。
        let mut chain = self.taxonomy.ancestry(leaf);
        chain.pop(); // 軸の根を除く
        chain.reverse();
        chain.into_iter().map(PathSeg::Category).collect()
    }

    /// リソース円の色＝ズーム軸の値（葉）の色。
    fn resource_color(&self, r: &Res) -> String {
        match r.category(self.taxonomy, self.zoom_dim) {
            Some(leaf) => self.category_color(leaf),
            None => DEFAULT_COLOR.to_string(),
        }
    }

    /// カテゴリ id の色。color 属性が空なら既定色。
    fn category_color(&self, id: &CategoryId) -> String {
        match self.taxonomy.node(id) {
            Some(c) if !c.attribute.color.is_empty() => c.attribute.color.clone(),
            _ => DEFAULT_COLOR.to_string(),
        }
    }

    /// クラスタキーの表示ラベル。
    fn key_label(&self, key: &PathSeg) -> String {
        match key {
            PathSeg::Other => OTHER_LABEL.to_string(),
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

    /// items を level でグルーピングしながら葉まで入れ子にする（map.js buildLevel）。
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

        // d3.group と同じく初出順を保ってグルーピングする。
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
                    axis: self.zoom_dim.clone(),
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

    /// トップレベル配置。最も密集したクラスタに合わせて全体を leafScale 倍に広げる。
    /// 返り値は layout_scale。
    fn layout_top_level(&self, nodes: &mut [MapNode]) -> f64 {
        if nodes.is_empty() {
            return 1.0;
        }

        let max_leaves = nodes
            .iter()
            .map(MapNode::max_resource_child_count)
            .max()
            .unwrap_or(0)
            .max(3) as f64;
        let leaf_scale = (max_leaves / 3.0).sqrt();

        let (w, h) = (self.width, self.height);
        let min_wh = w.min(h);
        let max_freq = nodes.iter().map(MapNode::total_freq).fold(1.0, f64::max);
        let max_r = min_wh * 0.22 * leaf_scale;
        let min_r = 60.0 * leaf_scale;
        let orbit_r = min_wh * 0.3 * leaf_scale;

        let len = nodes.len() as f64;
        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = node.total_freq();
            let p = node.placement_mut();
            p.r = min_r + (max_r - min_r) * (freq / max_freq).sqrt();
            let angle = (i as f64 / len) * 2.0 * PI - PI / 2.0;
            p.x = w / 2.0 + angle.cos() * orbit_r;
            p.y = h / 2.0 + angle.sin() * orbit_r;
        }

        Self::run_force(nodes, w / 2.0, h / 2.0, None);

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

    /// クラスタ内の子配置。子がリソースなら衛星配置、クラスタなら再帰。
    fn layout_children(&self, nodes: &mut [MapNode], parent_x: f64, parent_y: f64, parent_r: f64) {
        if nodes.is_empty() {
            return;
        }
        if nodes[0].is_resource() {
            Self::layout_resource_nodes(nodes, parent_x, parent_y, parent_r);
            return;
        }

        let max_freq = nodes.iter().map(MapNode::total_freq).fold(1.0, f64::max);
        let max_r = parent_r * 0.40;
        let min_r = (parent_r * 0.12).max(8.0);

        let len = nodes.len().max(1) as f64;
        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = node.total_freq();
            let p = node.placement_mut();
            // JS の Math.max(minR, Math.min(maxR, v)): min>max でも破綻しないよう clamp は使わない。
            let v = min_r + (max_r - min_r) * (freq / max_freq).sqrt();
            p.r = min_r.max(max_r.min(v));
            let angle = (i as f64 / len) * 2.0 * PI;
            p.x = parent_x + angle.cos() * parent_r * 0.45;
            p.y = parent_y + angle.sin() * parent_r * 0.45;
        }

        Self::run_force(nodes, parent_x, parent_y, Some(parent_r));

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

    /// リソース葉の衛星配置。golden-angle 初期配置のあと衝突解消する。
    fn layout_resource_nodes(
        nodes: &mut [MapNode],
        parent_x: f64,
        parent_y: f64,
        parent_r: f64,
    ) {
        let golden = 137.508_f64.to_radians();
        let spread = parent_r * 0.68;

        for (i, node) in nodes.iter_mut().enumerate() {
            let freq = match node {
                MapNode::Resource(n) => n.freq,
                _ => 1.0,
            };
            // JS: clamp(4, 10, freq*0.7+2.5)。min<max が保証される定数なので素直に書ける。
            let base_r = 4.0_f64.max(10.0_f64.min(freq * 0.7 + 2.5));
            let p = node.placement_mut();
            p.r = base_r;
            let angle = i as f64 * golden;
            let dist = (0.28 * spread * ((i as f64) + 1.0).sqrt()).min(spread - p.r);
            p.x = parent_x + angle.cos() * dist.max(0.0);
            p.y = parent_y + angle.sin() * dist.max(0.0);
        }

        Self::run_force(nodes, parent_x, parent_y, Some(parent_r));
    }

    /// runForce: ノードの現在位置から Body を作りシミュレーションし、x,y を書き戻す。
    fn run_force(nodes: &mut [MapNode], cx: f64, cy: f64, bound_r: Option<f64>) {
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
    use cumulo_model::{Catalog, Category, Id};

    fn cid(s: &str) -> CategoryId {
        s.try_into().unwrap()
    }
    fn rid(s: &str) -> ResourceId {
        s.try_into().unwrap()
    }

    fn cat(id: &str, label: &str, parent: Option<&str>, color: &str) -> Category<CategoryAttribute> {
        Category {
            id: cid(id),
            label: label.into(),
            parent: parent.map(cid),
            attribute: CategoryAttribute {
                color: color.into(),
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

    // platform > gcp > bigquery / bigtable ; aws > s3
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

    // ズーム軸 platform で bigquery のパスは [gcp, bigquery]（軸 platform 自身は除く）。
    #[test]
    fn zoom_path_excludes_axis_root_and_orders_top_down() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let r = res("r1", &["bigquery"], 1);
        assert_eq!(
            engine.zoom_path(&r),
            vec![PathSeg::Category(cid("gcp")), PathSeg::Category(cid("bigquery"))]
        );
    }

    // 軸に値を持たないリソースは Other パスへ。
    #[test]
    fn zoom_path_without_value_is_other() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let r = res("r1", &[], 1);
        assert_eq!(engine.zoom_path(&r), vec![PathSeg::Other]);
    }

    // 同じ gcp 配下のリソースは gcp クラスタ → bigquery/bigtable のサブクラスタへ集約される。
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
        // トップレベルは gcp と aws の 2 クラスタ
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

    // クラスタは色とラベルを taxonomy から得る。
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

    // 配置後、トップレベルクラスタには有限の座標と正の半径が入る。
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

    // 値ありクラスタのドリル先は (軸, 値)。クリックの意味を純粋判定としてテストできる。
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

    // 値を持たない Other クラスタはドリル対象外（None）。
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
    }

    // 空入力でもパニックせず、空の木を返す。
    #[test]
    fn empty_resources_produce_empty_tree() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let layout = engine.build(&[]);
        assert!(layout.tree.is_empty());
    }

    // Catalog 経由でも問題なく動く（型確認）。
    #[test]
    fn works_with_catalog_slice() {
        let tax = taxonomy();
        let zd = cid("platform");
        let engine = LayoutEngine::new(&tax, &zd, 900.0, 600.0);
        let catalog = Catalog(vec![res("r1", &["bigquery"], 3)]);
        let layout = engine.build(&catalog);
        assert_eq!(layout.tree.len(), 1);
        let _ = Id::<()>::try_from("x"); // Id 型の再エクスポート確認
    }
}
