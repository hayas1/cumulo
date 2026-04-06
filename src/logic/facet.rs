use crate::model::*;
use std::collections::HashSet;

/// 現在の選択条件にマッチするリソースを返す（attrs キー直接参照版）
pub fn filter_resources<'a>(
    resources: &'a [Resource],
    selected: &[(String, String)],
) -> Vec<&'a Resource> {
    resources
        .iter()
        .filter(|r| {
            selected
                .iter()
                .all(|(k, v)| r.attrs.get(k).map(|s| s.as_str()) == Some(v.as_str()))
        })
        .collect()
}

/// Dimension の mappings を使って選択条件にマッチするリソースを返す
pub fn filter_resources_with_dimensions<'a>(
    resources: &'a [Resource],
    selected: &[(String, String)], // (dimension_id, resolved_value)
    dimensions: &[Dimension],
) -> Vec<&'a Resource> {
    resources
        .iter()
        .filter(|r| {
            selected.iter().all(|(dim_id, value)| {
                if let Some(dim) = dimensions.iter().find(|d| &d.id == dim_id) {
                    resolve_dimension(r, dim).as_deref() == Some(value.as_str())
                } else {
                    r.attrs.get(dim_id).map(|s| s.as_str()) == Some(value.as_str())
                }
            })
        })
        .collect()
}

/// Dimension の mappings から Resource の Dimension 値を解決する
pub fn resolve_dimension(resource: &Resource, dimension: &Dimension) -> Option<String> {
    for mapping in &dimension.mappings {
        let matches = mapping
            .conditions
            .iter()
            .all(|(k, v)| resource.attrs.get(k).map(|s| s.as_str()) == Some(v.as_str()));
        if matches {
            let raw_value = resource.attrs.get(&mapping.source_key)?;
            return Some(match &mapping.value_map {
                Some(map) => map
                    .get(raw_value)
                    .cloned()
                    .unwrap_or_else(|| raw_value.clone()),
                None => raw_value.clone(),
            });
        }
    }
    None
}

/// spec 準拠: 未選択の Dimension とその候補値を返す
pub fn available_facets(
    resources: &[Resource],
    selected: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<(Dimension, Vec<String>)> {
    let filtered = filter_resources(resources, selected);
    let selected_keys: HashSet<&str> = selected.iter().map(|(k, _)| k.as_str()).collect();

    dimensions
        .iter()
        .filter(|d| !selected_keys.contains(d.id.as_str()))
        .map(|dim| {
            let mut values: Vec<String> = filtered
                .iter()
                .filter_map(|r| resolve_dimension(r, dim))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            sort_values(&mut values, &dim.ordered_values);
            (dim.clone(), values)
        })
        .filter(|(_, values)| !values.is_empty())
        .collect()
}

/// UI 用: 選択済みも含めた全 Dimension の候補値を返す。
/// 各 Dimension の候補は「その Dimension 以外の選択条件」で絞り込んだ結果から計算する。
pub fn all_facets_with_values(
    resources: &[Resource],
    selected: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<(Dimension, Vec<String>)> {
    dimensions
        .iter()
        .map(|dim| {
            let other_selected: Vec<_> = selected
                .iter()
                .filter(|(k, _)| k != &dim.id)
                .cloned()
                .collect();

            let filtered =
                filter_resources_with_dimensions(resources, &other_selected, dimensions);

            let mut values: Vec<String> = filtered
                .iter()
                .filter_map(|r| resolve_dimension(r, dim))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            sort_values(&mut values, &dim.ordered_values);

            (dim.clone(), values)
        })
        .filter(|(_, values)| !values.is_empty())
        .collect()
}

/// 現在の選択条件にマッチするリソースを返す（UI の結果パネル用）
pub fn filtered_resources<'a>(
    resources: &'a [Resource],
    selected: &[(String, String)],
    dimensions: &[Dimension],
) -> Vec<&'a Resource> {
    filter_resources_with_dimensions(resources, selected, dimensions)
}

fn sort_values(values: &mut Vec<String>, ordered_values: &Option<Vec<String>>) {
    if let Some(order) = ordered_values {
        values.sort_by_key(|v| order.iter().position(|o| o == v).unwrap_or(usize::MAX));
    } else {
        values.sort();
    }
}
