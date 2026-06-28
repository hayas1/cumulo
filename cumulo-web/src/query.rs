//! URL クエリに載りうる全状態を 1 つの struct に集約する。
//!
//! クエリの名前空間（どのキーが何を表すか）はここで一元管理し、[`ParamsMap`] との変換も
//! ここに閉じる。新しいクエリ要素はフィールドを足して扱う。`QueryState` はクエリ全体を表す
//! ので [`Self::to_params`] は毎回まっさら組み直せて、「一部だけ差し替える」破壊的更新が要らない。
//! 将来フィールドを足しても read→該当フィールド差し替え→write を型経由で回せば他要素は保たれる。
//!
//! モデル化していない外部由来のキー（utm 等）は `rest` に素通しで退避し、書き出しで戻す。

use leptos_router::params::ParamsMap;

use crate::category::{CategoryId, Filters};

/// URL クエリに載りうる全状態。[`ParamsMap`]（生のキー値）と型付き状態の境界。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueryState {
    /// 絞り込み。クエリ上は `f.<軸>=<値>` の名前空間に載る。
    pub filters: Filters,
    // 将来の例: pub zoom: Option<CategoryId>,  // 例えば素のキー `zoom=<軸>` に載せる
    /// どのフィールドにもモデル化されていないクエリ（外部由来の utm 等）。
    /// 全状態を書き出す to_params で他人のキーを消さないよう、from_params で退避し書き戻す。
    rest: Vec<(String, String)>,
}

impl QueryState {
    /// フィルタ用クエリキーの接頭辞。値に任意のカテゴリ id を載せるため名前空間を確保する。
    /// 他のクエリ要素は、この接頭辞で始まらないキーを使えば衝突しない。
    const FILTER_PREFIX: &str = "f.";

    /// クエリ全体を型付き状態へ読み取る。各フィールドの consume に順に通し、
    /// どれも消費しなかったキーは rest へ退避する。フィールドを足すときは consume を 1 つ追加する。
    pub fn from_params(params: &ParamsMap) -> Self {
        let mut filters = Filters::new();
        let mut rest = Vec::new();
        for (key, value) in params {
            if Self::consume_filter(&mut filters, key, value) {
                continue;
            }
            rest.push((key.to_string(), value.to_string()));
        }
        Self { filters, rest }
    }

    /// 型付き状態をクエリ全体として書き出す。全状態を持つので毎回新規に組む。
    /// フィールドを足すときは write_* を 1 つ追加する。rest（外部キー）は最後に戻す。
    pub fn to_params(&self) -> ParamsMap {
        let mut params = ParamsMap::new();
        self.write_filters(&mut params);
        for (key, value) in &self.rest {
            params.insert(key.clone(), value.clone());
        }
        params
    }

    /// key がフィルタ namespace（`f.`）のキーなら filters に取り込んで true を返す。
    /// namespace 内だが壊れている（空 id 等）場合も「消費した」扱いで true にし、rest に流さず捨てる。
    /// 共有元と同じ bipartite 前提なので id の実在は検証しない。
    fn consume_filter(filters: &mut Filters, key: &str, value: &str) -> bool {
        let Some(root) = key.strip_prefix(Self::FILTER_PREFIX) else {
            return false;
        };
        if let (Ok(root), Ok(value)) = (CategoryId::try_from(root), CategoryId::try_from(value)) {
            filters.set(root, value);
        }
        true
    }

    /// filters を `f.<軸>=<値>` のキー列として書き込む。
    fn write_filters(&self, params: &mut ParamsMap) {
        for (root, value) in self.filters.iter() {
            params.insert(
                format!("{}{}", Self::FILTER_PREFIX, root.as_str()),
                value.to_string(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // 往復: to_params で書き出したクエリを from_params で復元すると元の状態に戻る（挿入順も保つ）
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

    // 各軸は f. 接頭辞付きキーになり、それ以外のキーは出さない
    #[test]
    fn to_params_emits_only_prefixed_keys() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("f.platform").as_deref(), Some("gcp"));
        assert_eq!(q.get("platform"), None);
    }

    // 接頭辞なしのキーは filters には入らない（フィルタ namespace を汚さない）
    #[test]
    fn unprefixed_keys_do_not_become_filters() {
        let q = params(&[("zoom", "region"), ("f.platform", "gcp")]);
        assert_eq!(
            QueryState::from_params(&q).filters,
            filters(&[("platform", "gcp")])
        );
    }

    // モデル外のキー（外部由来 utm 等）は rest に退避され、書き戻しで保持される
    #[test]
    fn preserves_foreign_params() {
        let q = params(&[("utm_source", "tw"), ("f.platform", "gcp")]);
        let out = QueryState::from_params(&q).to_params();
        assert_eq!(out.get("utm_source").as_deref(), Some("tw"));
        assert_eq!(out.get("f.platform").as_deref(), Some("gcp"));
    }

    // 空 id を値に持つ壊れたキーは取り込まない（Id の非空不変条件を守る）
    #[test]
    fn from_params_skips_empty_id() {
        let q = params(&[("f.platform", ""), ("f.env", "prod")]);
        assert_eq!(QueryState::from_params(&q), state(&[("env", "prod")]));
    }

    // カテゴリ id に . が含まれても壊れない。接頭辞 f. を剥がした「残り全体」が id なので、
    // id 内部の . は素通りする（区切りとして誤解釈しない）。
    #[test]
    fn handles_dotted_category_id() {
        let s = state(&[("a.b.c", "x.y")]);
        let q = s.to_params();
        assert_eq!(q.get("f.a.b.c").as_deref(), Some("x.y"));
        assert_eq!(QueryState::from_params(&q), s);
    }
}
