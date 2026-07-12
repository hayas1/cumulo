pub const CLOUD: &str = include_str!("cloud.json");

#[cfg(test)]
mod tests {
    use crate::ExportData;

    #[test]
    fn cloud_demo_parses_into_catalog_and_taxonomy() {
        let bipartite =
            ExportData::<serde_json::Value, serde_json::Value>::parse(super::CLOUD).unwrap();
        assert!(!bipartite.catalog.is_empty());
        assert!(!bipartite.taxonomy.is_empty());
    }
}
