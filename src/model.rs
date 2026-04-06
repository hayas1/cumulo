use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub console_url: String,
    pub created_at: String,
}

/// 論理軸の定義
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
    pub conditions: Vec<(String, String)>,
    pub source_key: String,
    pub value_map: Option<HashMap<String, String>>,
}

/// キューブモードの表示設定
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CubeConfig {
    pub axis_x: String,
    pub axis_y: String,
    pub axis_z: String,
    pub filters: Vec<(String, String)>,
}

impl Default for CubeConfig {
    fn default() -> Self {
        Self {
            axis_x: "vendor".to_string(),
            axis_y: "env".to_string(),
            axis_z: "service".to_string(),
            filters: vec![],
        }
    }
}

/// ファセットモードの状態
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FacetState {
    /// (dimension_id, resolved_value) のペアのリスト
    pub selected: Vec<(String, String)>,
}

impl FacetState {
    pub fn get_selected(&self, dim_id: &str) -> Option<&str> {
        self.selected
            .iter()
            .find(|(k, _)| k == dim_id)
            .map(|(_, v)| v.as_str())
    }

    /// 同じ dimension_id の既存選択を置き換えるか、同じ値なら削除する（トグル）
    pub fn toggle(&mut self, dim_id: String, value: String) {
        if let Some(pos) = self
            .selected
            .iter()
            .position(|(k, v)| k == &dim_id && v == &value)
        {
            self.selected.remove(pos);
        } else {
            self.selected.retain(|(k, _)| k != &dim_id);
            self.selected.push((dim_id, value));
        }
    }

    pub fn remove(&mut self, dim_id: &str) {
        self.selected.retain(|(k, _)| k != dim_id);
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }
}

/// LocalStorageに保存するルートデータ構造
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppStore {
    pub resources: Vec<Resource>,
    pub dimensions: Vec<Dimension>,
    pub cube_config: CubeConfig,
}
