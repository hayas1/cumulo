use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Dimension {
    pub id: String,
    pub label: String,
    /// あり得る値の列挙（順序＋カラー）。空なら動的に収集してアルファベット順。
    pub values: Vec<DimensionValue>,
}

/// Dimensionに属する値の定義
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionValue {
    pub value: String,
    pub color: Option<String>,
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
            zoom_axes: [
                "vendor".to_string(),
                "service".to_string(),
                "resource_type".to_string(),
            ],
            color_axis: "resource_type".to_string(),
        }
    }
}

/// LocalStorageに保存するルートデータ構造
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppStore {
    pub resources: Vec<Resource>,
    pub dimensions: Vec<Dimension>,
    pub map_config: MapConfig,
}
