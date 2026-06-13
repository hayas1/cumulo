pub const CLOUD: &str = include_str!("cloud.json");

#[cfg(test)]
mod tests {
    use crate::ExportData;

    // attribute 型は web 層にあるので、ここでは serde_json::Value で flatten を受けてスキーマ整合のみ検証する。
    // store 内のキー名（catalog / taxonomy）のドリフトをここで捕まえる。
    #[test]
    fn cloud_demo_parses_into_catalog_and_taxonomy() {
        let bipartite =
            ExportData::<serde_json::Value, serde_json::Value>::parse(super::CLOUD).unwrap();
        assert!(!bipartite.catalog.is_empty());
        assert!(!bipartite.taxonomy.is_empty());
    }
}
