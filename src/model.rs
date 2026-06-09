use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub name: String,
    /// キー名はユーザー定義。システムは強制しない。
    pub attrs: HashMap<String, String>,
    pub console_url: String,
    pub created_at: Option<String>,
    /// アクセス頻度（表示サイズに使用）
    pub freq: u32,
    pub parent_id: Option<String>,
}

impl Resource {
    /// 親リソースの attrs を継承した実効 attrs を返す（子が優先）
    pub fn effective_attrs<'a>(&'a self, all: &'a [Resource]) -> HashMap<String, String> {
        if let Some(pid) = &self.parent_id {
            if let Some(parent) = all.iter().find(|r| &r.id == pid) {
                let mut merged = parent.effective_attrs(all);
                merged.extend(self.attrs.clone());
                return merged;
            }
        }
        self.attrs.clone()
    }
}

/// 論理軸の定義。ファセット推論とクラスタリングの両方に使う。
///
/// `values` は `parent` リンクによって森（複数の木）を成しうる。
/// 例: `Cloud ⊃ GCP ⊃ BigQuery`。リソースは葉（または任意のノード）を
/// `attrs[dim.id]` で指し、祖先は parent を辿って導出する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Dimension {
    pub id: String,
    pub label: String,
    /// あり得る値の列挙（順序＋カラー＋親）。空なら動的に収集してアルファベット順。
    pub values: Vec<DimensionValue>,
}

impl Dimension {
    /// このdimensionの値が階層（親リンク）を持つか
    pub fn is_hierarchical(&self) -> bool {
        self.values.iter().any(|dv| dv.parent.is_some())
    }

    /// `value` から根までの祖先チェーン（自身を含む、近い順）を返す。
    /// 未定義の値はその値だけを返す。循環は安全に打ち切る。
    pub fn ancestry(&self, value: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(value.to_string());
        while let Some(v) = cur {
            if chain.contains(&v) {
                break; // 循環ガード
            }
            cur = self
                .values
                .iter()
                .find(|dv| dv.value == v)
                .and_then(|dv| dv.parent.clone());
            chain.push(v);
        }
        chain
    }

    /// 指定した親を直接の親に持つ子values（`None`で根の一覧）。定義順を保つ。
    pub fn children_of(&self, parent: Option<&str>) -> Vec<&DimensionValue> {
        self.values
            .iter()
            .filter(|dv| dv.parent.as_deref() == parent)
            .collect()
    }
}

/// Dimensionに属する値の定義
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionValue {
    pub value: String,
    pub color: Option<String>,
    /// 同じdimension内の親valueへの参照。根はNone。
    #[serde(default)]
    pub parent: Option<String>,
}

/// マップビューの設定
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MapConfig {
    /// ズーム軸（3段階）
    pub zoom_axes: [String; 3],
    /// 色軸（最深ズームでの色分けに使う）
    pub color_axis: String,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            zoom_axes: ["platform".to_string(), "env".to_string(), "env".to_string()],
            color_axis: "platform".to_string(),
        }
    }
}

/// LocalStorageに保存するルートデータ構造
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppStore {
    pub resources: Vec<Resource>,
    pub dimensions: Vec<Dimension>,
    pub map_config: MapConfig,
}
