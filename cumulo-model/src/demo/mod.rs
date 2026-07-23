pub const CLOUD: &str = include_str!("cloud.json");

#[cfg(test)]
mod tests {
    use crate::{Category, ExportData, Forest, Id};

    type Demo = crate::Bipartite<serde_json::Value, serde_json::Value>;

    fn parse_demo() -> Demo {
        ExportData::parse(super::CLOUD).unwrap()
    }

    fn cid(s: &str) -> Id<Category<serde_json::Value>> {
        s.try_into().unwrap()
    }

    #[test]
    fn cloud_demo_parses_into_catalog_and_taxonomy() {
        let bipartite = parse_demo();
        assert!(!bipartite.catalog.is_empty());
        assert!(!bipartite.taxonomy.is_empty());
    }

    #[test]
    fn cloud_demo_pairs_each_cloud_resource_with_a_matching_tenant() {
        let bipartite = parse_demo();
        for (cloud, tenant_kind) in [("gcp", "gcp-project"), ("aws", "aws-account")] {
            for r in bipartite.catalog.iter() {
                let on_cloud = r
                    .categories
                    .iter()
                    .any(|c| bipartite.taxonomy.ancestry_contains(c, &cid(cloud)));
                if !on_cloud {
                    continue;
                }
                let tenant = r
                    .category(&bipartite.taxonomy, &cid("tenant"))
                    .unwrap_or_else(|| panic!("{} has no tenant value", r.id));
                assert!(
                    bipartite
                        .taxonomy
                        .ancestry_contains(tenant, &cid(tenant_kind)),
                    "{} on {} should have a tenant under {}",
                    r.id,
                    cloud,
                    tenant_kind
                );
            }
        }
    }
}
