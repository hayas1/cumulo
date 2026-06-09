use crate::model::*;
use std::collections::HashSet;

/// 選択中のタグにマッチするリソースを返す。
///
/// 階層dimensionでは、祖先ノード（例: `GCP`）を選ぶと
/// その子孫（`BigQuery` 等）を持つリソースもマッチする。
pub fn filter_resources<'a>(
    resources: &'a [Resource],
    selected_tags: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<&'a Resource> {
    resources
        .iter()
        .filter(|r| {
            selected_tags
                .iter()
                .all(|(k, v)| tag_matches(r, k, v, dimensions))
        })
        .collect()
}

/// リソースがタグ (k, v) にマッチするか。階層dimensionなら祖先一致も許容。
fn tag_matches(r: &Resource, k: &str, v: &str, dimensions: &[Dimension]) -> bool {
    let Some(rv) = r.attrs.get(k) else {
        return false;
    };
    match dimensions.iter().find(|d| d.id == k) {
        Some(dim) if dim.is_hierarchical() => dim.ancestry(rv).iter().any(|a| a == v),
        _ => rv == v,
    }
}

/// 現在の絞り込み状態で次に選択可能な属性と値の候補を返す（Dimension ベース）
pub fn available_facets(
    resources: &[Resource],
    selected_tags: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<(Dimension, Vec<String>)> {
    let filtered = filter_resources(resources, selected_tags, dimensions);
    let used_keys: HashSet<&str> = selected_tags.iter().map(|(k, _)| k.as_str()).collect();

    dimensions
        .iter()
        .filter(|d| !used_keys.contains(d.id.as_str()))
        .map(|dim| {
            let mut vals: Vec<String> = filtered
                .iter()
                .filter_map(|r| resolve_dimension(r, dim))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            if !dim.values.is_empty() {
                vals.sort_by_key(|v| {
                    dim.values
                        .iter()
                        .position(|dv| &dv.value == v)
                        .unwrap_or(usize::MAX)
                });
            } else {
                vals.sort();
            }
            (dim.clone(), vals)
        })
        .filter(|(_, vals)| !vals.is_empty())
        .collect()
}

/// Dimensionのsource_keyからこのリソースのDimension値を解決する
pub fn resolve_dimension(resource: &Resource, dim: &Dimension) -> Option<String> {
    resource.attrs.get(&dim.id).cloned()
}

/// パレット用: 現在の絞り込み後リソースから選択可能な (attr_key, value) ペアを返す。
/// 既に選択済みのペアは除外する。
pub fn available_tags(
    resources: &[Resource],
    selected: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<(String, String)> {
    let filtered = filter_resources(resources, selected, dimensions);
    let selected_set: HashSet<(&str, &str)> = selected
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut tags: HashSet<(String, String)> = HashSet::new();
    for r in &filtered {
        for (k, v) in &r.attrs {
            // 階層dimensionは祖先ノードも候補として展開（GCP, Cloud …）
            let candidates = match dimensions.iter().find(|d| &d.id == k) {
                Some(dim) if dim.is_hierarchical() => dim.ancestry(v),
                _ => vec![v.clone()],
            };
            for cand in candidates {
                if !selected_set.contains(&(k.as_str(), cand.as_str())) {
                    tags.insert((k.clone(), cand));
                }
            }
        }
    }

    let mut tags_vec: Vec<(String, String)> = tags.into_iter().collect();
    tags_vec.sort();
    tags_vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn dim() -> Dimension {
        // Cloud ⊃ GCP ⊃ BigQuery / Bigtable, Cloud ⊃ AWS ⊃ S3
        Dimension {
            id: "platform".into(),
            label: "プラットフォーム".into(),
            values: vec![
                DimensionValue {
                    value: "Cloud".into(),
                    color: None,
                    parent: None,
                },
                DimensionValue {
                    value: "GCP".into(),
                    color: None,
                    parent: Some("Cloud".into()),
                },
                DimensionValue {
                    value: "AWS".into(),
                    color: None,
                    parent: Some("Cloud".into()),
                },
                DimensionValue {
                    value: "BigQuery".into(),
                    color: None,
                    parent: Some("GCP".into()),
                },
                DimensionValue {
                    value: "Bigtable".into(),
                    color: None,
                    parent: Some("GCP".into()),
                },
                DimensionValue {
                    value: "S3".into(),
                    color: None,
                    parent: Some("AWS".into()),
                },
            ],
        }
    }

    fn res(id: &str, platform: &str) -> Resource {
        Resource {
            id: id.into(),
            name: id.into(),
            attrs: HashMap::from([("platform".into(), platform.into())]),
            console_url: String::new(),
            created_at: None,
            freq: 1,
            parent_id: None,
        }
    }

    #[test]
    fn ancestry_walks_to_root() {
        assert_eq!(dim().ancestry("BigQuery"), vec!["BigQuery", "GCP", "Cloud"]);
        assert_eq!(dim().ancestry("Cloud"), vec!["Cloud"]);
        // 未定義の値は自身のみ
        assert_eq!(dim().ancestry("Unknown"), vec!["Unknown"]);
    }

    #[test]
    fn selecting_ancestor_matches_descendants() {
        let dims = vec![dim()];
        let resources = vec![res("a", "BigQuery"), res("b", "S3"), res("c", "Bigtable")];

        // GCP を選ぶと BigQuery / Bigtable がマッチ、S3 は外れる
        let got = filter_resources(&resources, &[("platform".into(), "GCP".into())], &dims);
        let ids: Vec<&str> = got.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c"]);

        // Cloud を選ぶと全部マッチ
        let got = filter_resources(&resources, &[("platform".into(), "Cloud".into())], &dims);
        assert_eq!(got.len(), 3);

        // 葉を直接選べば1件
        let got = filter_resources(&resources, &[("platform".into(), "S3".into())], &dims);
        assert_eq!(got.len(), 1);
    }
}
