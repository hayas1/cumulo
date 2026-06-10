use crate::model::*;
use std::collections::HashSet;

/// 選択中のタグにマッチするリソースを返す。
///
/// 祖先一致: `(platform, gcp)` を選ぶと `attrs["platform"] == "bigquery"` の
/// リソースもマッチ（bigquery の祖先に gcp が含まれるため）。
pub fn filter_resources<'a>(
    resources: &'a [Resource],
    selected_tags: &[(String, String)],
    dimensions: &[DimensionNode],
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

/// リソースがタグ (k, v) にマッチするか。
/// k は軸の根id、v はノードid。リソースの attrs[k] の祖先チェーンに v が含まれればマッチ。
fn tag_matches(r: &Resource, k: &str, v: &str, dimensions: &[DimensionNode]) -> bool {
    let Some(rv) = r.dimensions.get(k) else {
        return false;
    };
    // rv == v なら直接一致
    if rv == v {
        return true;
    }
    // rv の祖先チェーンに v が含まれるか（ancestry は根を含まない）
    ancestry(dimensions, rv).iter().any(|a| a == v)
}

/// 根ノード（軸）ごとに、現在の絞り込みで取り得るノードid の候補を返す。
#[allow(dead_code)]
pub fn available_facets(
    resources: &[Resource],
    selected_tags: &[(String, String)],
    dimensions: &[DimensionNode],
) -> Vec<(DimensionNode, Vec<String>)> {
    let filtered = filter_resources(resources, selected_tags, dimensions);
    let used_keys: HashSet<&str> = selected_tags.iter().map(|(k, _)| k.as_str()).collect();

    roots(dimensions)
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

/// リソースの属性値（軸の根id をキーとして直接引く）を返す。
pub fn resolve_dimension(resource: &Resource, root_id: &str) -> Option<String> {
    resource.dimensions.get(root_id).cloned()
}

/// 現在の絞り込み後リソースから選択可能な (軸id, ノードid) ペアを返す（祖先展開あり）。
pub fn available_tags(
    resources: &[Resource],
    selected: &[(String, String)],
    dimensions: &[DimensionNode],
) -> Vec<(String, String)> {
    let filtered = filter_resources(resources, selected, dimensions);
    let selected_set: HashSet<(&str, &str)> = selected
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut tags: HashSet<(String, String)> = HashSet::new();
    for r in &filtered {
        for (k, v) in &r.dimensions {
            // v 自身
            if !selected_set.contains(&(k.as_str(), v.as_str())) {
                tags.insert((k.clone(), v.clone()));
            }
            // 祖先ノードも候補として展開
            for anc in ancestry(dimensions, v) {
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
    use std::collections::HashMap;

    fn dims() -> Vec<DimensionNode> {
        // platform > cloud > gcp > bigquery / bigtable
        //                  > aws > s3
        vec![
            DimensionNode { id: "platform".into(), label: "Platform".into(), color: "#aaa".into(), parent: None },
            DimensionNode { id: "cloud".into(),    label: "Cloud".into(),    color: "#5B8DEF".into(), parent: Some("platform".into()) },
            DimensionNode { id: "gcp".into(),      label: "GCP".into(),      color: "#1D9E75".into(), parent: Some("cloud".into()) },
            DimensionNode { id: "aws".into(),      label: "AWS".into(),      color: "#F2920C".into(), parent: Some("cloud".into()) },
            DimensionNode { id: "bigquery".into(), label: "BigQuery".into(), color: "#1D9E75".into(), parent: Some("gcp".into()) },
            DimensionNode { id: "bigtable".into(), label: "Bigtable".into(), color: "#3A9E86".into(), parent: Some("gcp".into()) },
            DimensionNode { id: "s3".into(),       label: "S3".into(),       color: "#F2920C".into(), parent: Some("aws".into()) },
        ]
    }

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
    fn ancestry_walks_to_root_exclusive() {
        let d = dims();
        // bigquery の祖先: bigquery 自身 + gcp + cloud（platform は根なので含まない）
        assert_eq!(ancestry(&d, "bigquery"), vec!["bigquery", "gcp", "cloud"]);
        // cloud の祖先: cloud のみ（platform は根）
        assert_eq!(ancestry(&d, "cloud"), vec!["cloud"]);
        // 未定義の値は空（親なし、自身も根として扱われる）
        assert_eq!(ancestry(&d, "unknown"), Vec::<String>::new());
    }

    #[test]
    fn selecting_ancestor_matches_descendants() {
        let dimensions = dims();
        let resources = vec![
            res("a", "bigquery"),
            res("b", "s3"),
            res("c", "bigtable"),
        ];

        // gcp を選ぶと bigquery / bigtable がマッチ、s3 は外れる
        let got = filter_resources(
            &resources,
            &[("platform".into(), "gcp".into())],
            &dimensions,
        );
        let ids: Vec<&str> = got.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"c"));
        assert!(!ids.contains(&"b"));

        // cloud を選ぶと全部マッチ
        let got = filter_resources(
            &resources,
            &[("platform".into(), "cloud".into())],
            &dimensions,
        );
        assert_eq!(got.len(), 3);

        // 葉を直接選べば1件
        let got = filter_resources(
            &resources,
            &[("platform".into(), "s3".into())],
            &dimensions,
        );
        assert_eq!(got.len(), 1);
    }
}
