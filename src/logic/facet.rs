use crate::model::*;
use std::collections::HashSet;

/// 祖先一致: `(platform, gcp)` を選ぶと `attrs["platform"] == "bigquery"` の
/// リソースもマッチ（bigquery の祖先に gcp が含まれるため）。
pub fn filter_resources<'a>(
    resources: &'a [Resource],
    selected_tags: &[(String, String)],
    dimensions: &DimensionForest,
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

fn tag_matches(r: &Resource, k: &str, v: &str, dimensions: &DimensionForest) -> bool {
    let Some(rv) = r.dimensions.get(k) else {
        return false;
    };
    if rv == v {
        return true;
    }
    dimensions.ancestry(rv).iter().any(|a| a == v)
}

/// 根ノード（軸）ごとに、現在の絞り込みで取り得るノードid の候補を返す。
#[allow(dead_code)]
pub fn available_facets(
    resources: &[Resource],
    selected_tags: &[(String, String)],
    dimensions: &DimensionForest,
) -> Vec<(DimensionNode, Vec<String>)> {
    let filtered = filter_resources(resources, selected_tags, dimensions);
    let used_keys: HashSet<&str> = selected_tags.iter().map(|(k, _)| k.as_str()).collect();

    dimensions
        .roots()
        .into_iter()
        .filter(|root| !used_keys.contains(root.id.as_str()))
        .map(|root| {
            let mut vals: Vec<String> = filtered
                .iter()
                .filter_map(|r| r.dimensions.get(&root.id).cloned())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            vals.sort();
            (root.clone(), vals)
        })
        .filter(|(_, vals)| !vals.is_empty())
        .collect()
}

pub fn resolve_dimension(resource: &Resource, root_id: &str) -> Option<String> {
    resource.dimensions.get(root_id).cloned()
}

/// 現在の絞り込み後リソースから選択可能な (軸id, ノードid) ペアを返す（祖先展開あり）。
pub fn available_tags(
    resources: &[Resource],
    selected: &[(String, String)],
    dimensions: &DimensionForest,
) -> Vec<(String, String)> {
    let filtered = filter_resources(resources, selected, dimensions);
    let selected_set: HashSet<(&str, &str)> = selected
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut tags: HashSet<(String, String)> = HashSet::new();
    for r in &filtered {
        for (k, v) in &r.dimensions {
            if !selected_set.contains(&(k.as_str(), v.as_str())) {
                tags.insert((k.clone(), v.clone()));
            }
            for anc in dimensions.ancestry(v) {
                if !selected_set.contains(&(k.as_str(), anc.as_str())) {
                    tags.insert((k.clone(), anc));
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
    use crate::model::tests::test_forest;
    use std::collections::HashMap;

    fn res(id: &str, platform: &str) -> Resource {
        Resource {
            id: id.into(),
            label: None,
            dimensions: HashMap::from([("platform".into(), platform.into())]),
            console_url: String::new(),
            created_at: None,
            freq: 1,
        }
    }

    #[test]
    fn selecting_ancestor_matches_descendants() {
        let dimensions = test_forest();
        let resources = vec![res("a", "bigquery"), res("b", "s3"), res("c", "bigtable")];

        let got = filter_resources(
            &resources,
            &[("platform".into(), "gcp".into())],
            &dimensions,
        );
        let ids: Vec<&str> = got.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"c"));
        assert!(!ids.contains(&"b"));

        let got = filter_resources(
            &resources,
            &[("platform".into(), "cloud".into())],
            &dimensions,
        );
        assert_eq!(got.len(), 3);

        let got = filter_resources(&resources, &[("platform".into(), "s3".into())], &dimensions);
        assert_eq!(got.len(), 1);
    }
}
