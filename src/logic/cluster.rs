use crate::model::Resource;
use std::collections::HashMap;

/// ズーム軸の指定されたレベルでリソースをグループ化する
pub fn cluster_resources<'a>(
    resources: &'a [Resource],
    zoom_axes: &[String],
    level: usize,
) -> HashMap<String, Vec<&'a Resource>> {
    let dim_key = zoom_axes
        .get(level.min(2))
        .map(|s| s.as_str())
        .unwrap_or("vendor");
    let mut groups: HashMap<String, Vec<&Resource>> = HashMap::new();
    for r in resources {
        let key = r
            .attrs
            .get(dim_key)
            .cloned()
            .unwrap_or_else(|| "その他".to_string());
        groups.entry(key).or_default().push(r);
    }
    groups
}
