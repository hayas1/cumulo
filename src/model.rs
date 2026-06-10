use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// キーは軸の根id。値はその軸内のノードid。
    pub dimensions: HashMap<String, String>,
    pub console_url: String,
    pub created_at: Option<String>,
    /// アクセス頻度（表示サイズに使用）
    pub freq: u32,
}

impl Default for Resource {
    fn default() -> Self {
        Resource {
            id: String::new(),
            label: None,
            dimensions: HashMap::new(),
            console_url: String::new(),
            created_at: None,
            freq: 1,
        }
    }
}

impl Resource {
    /// 表示用ラベルを返す。label が空の場合はディメンション値のラベルで代替する。
    pub fn display_label(&self, dim_nodes: &[DimensionNode]) -> String {
        if let Some(l) = &self.label {
            if !l.is_empty() {
                return l.clone();
            }
        }
        let mut parts: Vec<String> = self
            .dimensions
            .values()
            .filter_map(|v| node(dim_nodes, v))
            .map(|n| {
                if n.label.is_empty() {
                    n.id.clone()
                } else {
                    n.label.clone()
                }
            })
            .collect();
        parts.sort();
        if parts.is_empty() {
            "(名前なし)".to_string()
        } else {
            parts.join(" / ")
        }
    }
}

/// ディメンション森の1ノード。
/// parent が None のノードが軸の根（＝属性キー）となる。
/// リソースの attrs は { 根id → ノードid } で表現する。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionNode {
    pub id: String,
    pub label: String,
    /// color は必須（UI で常に表示する）
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// LocalStorageに保存するルートデータ構造
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppStore {
    pub resources: Vec<Resource>,
    /// フラットなノード配列。parent リンクで森を構成する。
    pub dimensions: Vec<DimensionNode>,
}

// ── フラットノード列に対するヘルパー ────────────────────────────────────────

/// parent が None のノード（軸の根）を定義順で返す。
pub fn roots(nodes: &[DimensionNode]) -> Vec<&DimensionNode> {
    nodes.iter().filter(|n| n.parent.is_none()).collect()
}

/// 指定 id を直接の親に持つ子ノードを定義順で返す。
pub fn children_of<'a>(nodes: &'a [DimensionNode], parent_id: &str) -> Vec<&'a DimensionNode> {
    nodes
        .iter()
        .filter(|n| n.parent.as_deref() == Some(parent_id))
        .collect()
}

/// id から根まで辿った祖先チェーン（自身を含む、近い順）を返す。
/// ただし軸の根 (parent==None) の id は含めない（根はキーであり値ではない）。
/// 循環は安全に打ち切る。
pub fn ancestry(nodes: &[DimensionNode], id: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut cur = Some(id.to_string());
    while let Some(c) = cur {
        if chain.contains(&c) {
            break; // 循環ガード
        }
        let found = nodes.iter().find(|n| n.id == c);
        let parent = found.and_then(|n| n.parent.clone());
        if parent.is_none() {
            // 根ノード自身は chain に含めない
            break;
        }
        chain.push(c);
        cur = parent;
    }
    chain
}

/// id が属する軸の根id を返す。id が根自身なら None。
pub fn root_of(nodes: &[DimensionNode], id: &str) -> Option<String> {
    let mut cur = id.to_string();
    let mut seen = std::collections::HashSet::new();
    loop {
        if !seen.insert(cur.clone()) {
            return None; // 循環
        }
        match nodes.iter().find(|n| n.id == cur) {
            None => return None,
            Some(n) => match &n.parent {
                None => return None, // cur 自身が根
                Some(p) => {
                    // p が根かどうか確認
                    if nodes
                        .iter()
                        .find(|n| &n.id == p)
                        .map_or(false, |n| n.parent.is_none())
                    {
                        return Some(p.clone());
                    }
                    cur = p.clone();
                }
            },
        }
    }
}

/// id に一致するノードを返す。
pub fn node<'a>(nodes: &'a [DimensionNode], id: &str) -> Option<&'a DimensionNode> {
    nodes.iter().find(|n| n.id == id)
}
