mod de;
mod error;
mod ser;

use std::collections::BTreeMap;

use leptos_router::params::ParamsMap;
use serde::{Deserialize, Serialize};

use crate::category::{CategoryId, Filters};
use crate::client::Client;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    Facet,
    Map,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryState {
    #[serde(default)]
    pub view: View,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub zoom_axis: Option<CategoryId>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(flatten)]
    rest: BTreeMap<String, String>,
}

impl QueryState {
    pub fn from_params(params: &ParamsMap) -> Self {
        de::from_1nest_params(params).unwrap_or_default()
    }

    pub fn to_params(&self) -> ParamsMap {
        ser::to_1nest_params(self).expect("QueryState serializes into a query map")
    }

    pub(crate) fn resolved_from(params: &ParamsMap, client: &Client) -> Self {
        let mut qs = Self::from_params(params);
        qs.zoom_axis = Some(qs.zoom_axis.unwrap_or_else(|| client.default_zoom_axis()));
        qs
    }

    pub(crate) fn adopt_url(&self, params: &ParamsMap, client: &Client) -> Option<Self> {
        let incoming = Self::resolved_from(params, client);
        (self != &incoming).then_some(incoming)
    }

    pub(crate) fn url_update(
        &self,
        current_url: &ParamsMap,
        pathname: &str,
    ) -> Option<(String, bool)> {
        let current = Self::from_params(current_url);
        if &current == self {
            return None;
        }
        let push = current.view != self.view;
        let url = format!("{}{}", pathname, self.to_params().to_query_string());
        Some((url, push))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::CategoryId;

    fn cid(s: &str) -> CategoryId {
        s.try_into().unwrap()
    }

    fn filters(pairs: &[(&str, &str)]) -> Filters {
        pairs.iter().map(|(r, v)| (cid(r), cid(v))).collect()
    }

    fn params(pairs: &[(&str, &str)]) -> ParamsMap {
        let mut m = ParamsMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    fn state(pairs: &[(&str, &str)]) -> QueryState {
        QueryState {
            filters: filters(pairs),
            ..Default::default()
        }
    }

    #[test]
    fn round_trips_preserving_order() {
        let s = state(&[("platform", "gcp"), ("env", "prod")]);
        let restored = QueryState::from_params(&s.to_params());
        assert_eq!(restored, s);
        let order: Vec<_> = restored
            .filters
            .iter()
            .map(|(k, _)| k.as_str().to_string())
            .collect();
        assert_eq!(order, vec!["platform", "env"]);
    }

    #[test]
    fn round_trips_view() {
        let s = QueryState {
            view: View::Map,
            filters: filters(&[("platform", "gcp")]),
            ..Default::default()
        };
        let q = s.to_params();
        assert_eq!(q.get("view").as_deref(), Some("map"));
        assert_eq!(QueryState::from_params(&q), s);
    }

    #[test]
    fn shows_view_even_when_default() {
        let q = QueryState::default().to_params();
        assert_eq!(q.get("view").as_deref(), Some("facet"));
    }

    #[test]
    fn round_trips_zoom_axis() {
        let s = QueryState {
            filters: filters(&[("platform", "gcp")]),
            zoom_axis: Some(cid("platform")),
            ..Default::default()
        };
        let q = s.to_params();
        assert_eq!(q.get("zoom_axis").as_deref(), Some("platform"));
        assert_eq!(QueryState::from_params(&q), s);
    }

    #[test]
    fn omits_zoom_axis_when_none() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("zoom_axis"), None);
    }

    #[test]
    fn round_trips_lang() {
        let s = QueryState {
            lang: Some("ja".to_string()),
            ..Default::default()
        };
        let q = s.to_params();
        assert_eq!(q.get("lang").as_deref(), Some("ja"));
        assert_eq!(QueryState::from_params(&q), s);
    }

    #[test]
    fn omits_lang_when_none() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("lang"), None);
    }

    #[test]
    fn to_params_uses_field_name_namespace() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("filters.platform").as_deref(), Some("gcp"));
        assert_eq!(q.get("platform"), None);
    }

    #[test]
    fn unprefixed_keys_do_not_become_filters() {
        let q = params(&[("zoom", "region"), ("filters.platform", "gcp")]);
        assert_eq!(
            QueryState::from_params(&q).filters,
            filters(&[("platform", "gcp")])
        );
    }

    #[test]
    fn preserves_foreign_params() {
        let q = params(&[("utm_source", "tw"), ("filters.platform", "gcp")]);
        let out = QueryState::from_params(&q).to_params();
        assert_eq!(out.get("utm_source").as_deref(), Some("tw"));
        assert_eq!(out.get("filters.platform").as_deref(), Some("gcp"));
    }

    #[test]
    fn handles_dotted_category_id() {
        let s = state(&[("a.b.c", "x.y")]);
        let q = s.to_params();
        assert_eq!(q.get("filters.a.b.c").as_deref(), Some("x.y"));
        assert_eq!(QueryState::from_params(&q), s);
    }

    #[test]
    fn url_update_is_none_when_url_already_canonical() {
        let desired = QueryState {
            zoom_axis: Some(cid("platform")),
            ..Default::default()
        };
        assert_eq!(desired.url_update(&desired.to_params(), "/base"), None);
    }

    #[test]
    fn url_update_canonicalizes_bare_url() {
        let desired = QueryState {
            zoom_axis: Some(cid("platform")),
            ..Default::default()
        };
        let (url, push) = desired
            .url_update(&ParamsMap::new(), "/base")
            .expect("bare URL is rewritten");
        assert_eq!(
            url,
            format!("/base{}", desired.to_params().to_query_string())
        );
        assert!(!push, "view unchanged, so replace (push=false)");
    }
}
