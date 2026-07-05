//! URL クエリに載りうる全状態を 1 つの struct に集約する。
//!
//! クエリ⇄型の変換は serde に委ねる。各フィールド名がそのまま名前空間になり（`filters.<軸>`）、
//! どのキーがどのフィールドかの振り分けは自作フォーマット（[`de`]/[`ser`]、ドットパス）＋derive が担う。
//! 新しいクエリ要素はフィールドを 1 つ足すだけで、振り分けコードは要らない。
//!
//! `QueryState` はクエリ全体を表すので [`Self::to_params`] は毎回まっさら組み直せて、
//! 「一部だけ差し替える」破壊的更新が要らない。モデル化していない外部由来のキー（utm 等）は
//! `#[serde(flatten)]` の `rest` に吸って書き戻すので、フィールド更新で他人のキーを消さない。

// ドットパス・クエリ ⇄ serde 型 の自作フォーマット（ser/de/error）。
mod de;
mod error;
mod ser;

use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos_router::params::ParamsMap;
use leptos_router::NavigateOptions;
use serde::{Deserialize, Serialize};

use crate::category::{CategoryId, Filters};
use crate::client::Client;

/// メイン画面のビュー（ファセット一覧 / マップ）。クエリ上は `view=map`（既定 facet は省略）。
/// パスではなくクエリに載せ、URL 全体を State の射影として一様に扱う。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    Facet,
    Map,
}

/// URL クエリに載りうる全状態。[`ParamsMap`]（生のキー値）と型付き状態の境界。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryState {
    /// メイン画面のビュー。クエリ上は `view=facet`/`view=map`。既定(facet)でも常に出す。
    #[serde(default)]
    pub view: View,
    /// 絞り込み。クエリ上は `filters.<軸>=<値>`（フィールド名 `filters` がそのまま名前空間）。
    #[serde(default)]
    pub filters: Filters,
    /// マップのクラスタリング軸（ズーム軸）。クエリ上は `zoom_axis=<軸>`。
    /// None は既定軸（taxonomy の先頭根）を表し、URL には出さない。
    #[serde(default)]
    pub zoom_axis: Option<CategoryId>,
    /// どのフィールドにもモデル化されていない外部由来のキー（utm 等）。flatten で吸い、
    /// to_params で書き戻して消さない。BTreeMap は出力順を安定させるため。
    #[serde(flatten)]
    rest: BTreeMap<String, String>,
}

impl QueryState {
    /// クエリ全体を型付き状態へ読み取る。
    /// 壊れたクエリ（型不一致など）でも画面を壊さないよう、解釈不能なら空状態に倒す。
    pub fn from_params(params: &ParamsMap) -> Self {
        de::from_1nest_params(params).unwrap_or_default()
    }

    /// 型付き状態をクエリ全体として書き出す。全状態を持つので毎回新規に組む。
    /// 文字列スカラと 1 段 map だけで構成されるので直列化は失敗しない。
    pub fn to_params(&self) -> ParamsMap {
        ser::to_1nest_params(self).expect("QueryState serializes into a query map")
    }

    /// URL から読み、既定を解決した具体状態にする。zoom_axis は URL 未指定でも
    /// 既定軸（taxonomy の先頭根）に補完し、常に具体値にする（＝URL にも既定を出す）。
    /// 初期 seed（App 起動時のちらつき防止の同期読み）にも使う。
    pub(crate) fn resolved_from(params: &ParamsMap, client: &Client) -> Self {
        let mut qs = Self::from_params(params);
        qs.zoom_axis = Some(qs.zoom_axis.unwrap_or_else(|| client.default_zoom_axis()));
        qs
    }

    /// URL→signal: クエリから丸ごと復元して signal に反映する（共有リンク復元・戻る/進む）。
    /// 同値なら据え置き、signal→URL 側と往復しない。`query` を購読するので Effect 内で呼ぶ。
    pub(crate) fn load_from_url(state: RwSignal<Self>, query: Memo<ParamsMap>, client: &Client) {
        let incoming = query.with(|p| Self::resolved_from(p, client));
        if state.get_untracked() != incoming {
            state.set(incoming);
        }
    }

    /// signal→URL: signal の変更を URL クエリへ書き出す（UI 操作の反映）。
    /// 比較相手は raw な from_params（未解決）にする。既定 zoom_axis を持つ signal と
    /// ずれることで URL に既定を書き出せる（この非対称は意図的）。`state` を購読するので Effect 内で呼ぶ。
    pub(crate) fn store_to_url(
        state: RwSignal<Self>,
        query: Memo<ParamsMap>,
        pathname: Memo<String>,
        navigate: impl Fn(&str, NavigateOptions),
    ) {
        let desired = state.get();
        let current = query.with_untracked(Self::from_params);
        if current == desired {
            return;
        }
        // view 切替は履歴に積み（戻るで前の view へ）、絞り込み等の微調整は replace で汚さない。
        let push = current.view != desired.view;
        let url = format!(
            "{}{}",
            pathname.get_untracked(),
            desired.to_params().to_query_string()
        );
        navigate(
            &url,
            NavigateOptions {
                resolve: false,
                replace: !push,
                scroll: false,
                ..Default::default()
            },
        );
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

    // view は素キー view=map のスカラとして往復する
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

    // 既定（facet）でも view は常に URL に出す（現代的な挙動。デフォルトを隠さない）
    #[test]
    fn shows_view_even_when_default() {
        let q = QueryState::default().to_params();
        assert_eq!(q.get("view").as_deref(), Some("facet"));
    }

    // zoom_axis は素キー zoom_axis=<軸> のスカラとして往復する
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

    // 既定軸（None）のときは zoom_axis キーを出さない
    #[test]
    fn omits_zoom_axis_when_none() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("zoom_axis"), None);
    }

    // 各軸は filters.<軸> キーになる（フィールド名 filters が名前空間）
    #[test]
    fn to_params_uses_field_name_namespace() {
        let q = state(&[("platform", "gcp")]).to_params();
        assert_eq!(q.get("filters.platform").as_deref(), Some("gcp"));
        assert_eq!(q.get("platform"), None);
    }

    // filters. 接頭辞でないキーは filters に入らない（外部キーは rest 行き）
    #[test]
    fn unprefixed_keys_do_not_become_filters() {
        let q = params(&[("zoom", "region"), ("filters.platform", "gcp")]);
        assert_eq!(
            QueryState::from_params(&q).filters,
            filters(&[("platform", "gcp")])
        );
    }

    // モデル外のキー（外部由来 utm 等）は rest に退避され、書き戻しで保持される
    #[test]
    fn preserves_foreign_params() {
        let q = params(&[("utm_source", "tw"), ("filters.platform", "gcp")]);
        let out = QueryState::from_params(&q).to_params();
        assert_eq!(out.get("utm_source").as_deref(), Some("tw"));
        assert_eq!(out.get("filters.platform").as_deref(), Some("gcp"));
    }

    // カテゴリ id に . が含まれても壊れない。最初の . までが namespace、残り全体が id。
    #[test]
    fn handles_dotted_category_id() {
        let s = state(&[("a.b.c", "x.y")]);
        let q = s.to_params();
        assert_eq!(q.get("filters.a.b.c").as_deref(), Some("x.y"));
        assert_eq!(QueryState::from_params(&q), s);
    }
}
