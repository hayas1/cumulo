pub struct Query(pub String);

impl Query {
    pub fn new(s: impl Into<String>) -> Self {
        Query(s.into())
    }

    /// 文字が順番通りに全て含まれるかを確認する（サブシーケンス照合）。
    /// "bq" → "bigquery" のような略称検索に使う。
    pub fn matches(&self, target: &str) -> bool {
        let mut target_iter = target.chars();
        for qc in self.0.chars() {
            if !target_iter.any(|tc| tc == qc) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Query;

    #[test]
    fn abbreviation_matches() {
        assert!(Query::new("bq").matches("bigquery"));
        assert!(Query::new("gcs").matches("google-cloud-storage"));
    }

    #[test]
    fn substring_matches() {
        assert!(Query::new("big").matches("bigquery"));
        assert!(Query::new("query").matches("bigquery"));
    }

    #[test]
    fn no_match_when_char_missing() {
        assert!(!Query::new("bq").matches("bigtable"));
        assert!(!Query::new("bq").matches("storage"));
    }

    #[test]
    fn order_matters() {
        assert!(!Query::new("qb").matches("bigquery"));
    }

    #[test]
    fn empty_query_matches_any() {
        assert!(Query::new("").matches("bigquery"));
        assert!(Query::new("").matches(""));
    }
}
