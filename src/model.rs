use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// サブリソース（BQのdataset、Auroraのdbなど）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChildResource {
    pub id: String,
    pub name: String,
    pub freq: u32,
    pub console_url: String,
}

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub name: String,
    /// キー名はユーザー定義。システムは強制しない。
    pub attrs: HashMap<String, String>,
    pub console_url: String,
    pub created_at: String,
    /// アクセス頻度（表示サイズに使用）
    pub freq: u32,
    pub parent_id: Option<String>,
    pub children: Vec<ChildResource>,
}

/// 論理軸の定義。ファセット推論とクラスタリングの両方に使う。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Dimension {
    pub id: String,
    pub label: String,
    pub mappings: Vec<TagMapping>,
    pub ordered_values: Option<Vec<String>>,
}

/// attrsのどのキーをこのDimensionの値として解釈するかの定義
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TagMapping {
    /// マッチ条件（AND）。空なら全リソースに適用。
    pub conditions: Vec<(String, String)>,
    pub source_key: String,
    /// 値の読み替えマップ（表記ゆれ吸収）
    pub value_map: Option<HashMap<String, String>>,
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
