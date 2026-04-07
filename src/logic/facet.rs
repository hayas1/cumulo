use crate::model::*;
use std::collections::HashSet;

/// 選択中のタグにマッチするリソースを返す（attrs キー直接参照）
pub fn filter_resources<'a>(
    resources: &'a [Resource],
    selected_tags: &[(String, String)],
) -> Vec<&'a Resource> {
    resources
        .iter()
        .filter(|r| {
            selected_tags
                .iter()
                .all(|(k, v)| r.attrs.get(k).map(|s| s.as_str()) == Some(v.as_str()))
        })
        .collect()
}

/// 現在の絞り込み状態で次に選択可能な属性と値の候補を返す（Dimension ベース）
pub fn available_facets(
    resources: &[Resource],
    selected_tags: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<(Dimension, Vec<String>)> {
    let filtered = filter_resources(resources, selected_tags);
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
            if let Some(order) = &dim.ordered_values {
                vals.sort_by_key(|v| order.iter().position(|o| o == v).unwrap_or(usize::MAX));
            } else {
                vals.sort();
            }
            (dim.clone(), vals)
        })
        .filter(|(_, vals)| !vals.is_empty())
        .collect()
}

/// DimensionのmappingsからこのリソースのDimension値を解決する
pub fn resolve_dimension(resource: &Resource, dim: &Dimension) -> Option<String> {
    for mapping in &dim.mappings {
        let matches = mapping
            .conditions
            .iter()
            .all(|(k, v)| resource.attrs.get(k).map(|s| s.as_str()) == Some(v.as_str()));
        if matches {
            let raw = resource.attrs.get(&mapping.source_key)?;
            return Some(match &mapping.value_map {
                Some(map) => map.get(raw).cloned().unwrap_or_else(|| raw.clone()),
                None => raw.clone(),
            });
        }
    }
    None
}

/// パレット用: 現在の絞り込み後リソースから選択可能な (attr_key, value) ペアを返す。
/// 既に選択済みのペアは除外する。
pub fn available_tags(
    resources: &[Resource],
    selected: &[(String, String)],
) -> Vec<(String, String)> {
    let filtered = filter_resources(resources, selected);
    let selected_set: HashSet<(&str, &str)> = selected
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut tags: HashSet<(String, String)> = HashSet::new();
    for r in &filtered {
        for (k, v) in &r.attrs {
            if !selected_set.contains(&(k.as_str(), v.as_str())) {
                tags.insert((k.clone(), v.clone()));
            }
        }
    }

    let mut tags_vec: Vec<(String, String)> = tags.into_iter().collect();
    tags_vec.sort();
    tags_vec
}
