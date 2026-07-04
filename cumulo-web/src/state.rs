//! アプリの「URL に載る UI 状態」をまとめたハンドル。[`Client`](crate::client::Client) と同じく
//! `Copy` なので prop-drill で配れる。signal 群（live な状態）と [`QueryState`]（直列化スナップショット）
//! は同じものの二面で、その対応（apply / overlay）をここに閉じる。
//! URL ⇄ 状態の同期そのものは router 文脈が要るので App 側の Effect が担う。

use leptos::prelude::*;

use crate::category::{CategoryId, Filters};
use crate::client::Client;
use crate::platform::Platform;
use crate::query::{QueryState, View};
use cumulo_model::Forest;

/// URL に載る UI 状態の live ハンドル（view / 絞り込み / ズーム軸）。
#[derive(Clone, Copy)]
pub struct State {
    pub view: RwSignal<View>,
    pub filters: RwSignal<Filters>,
    pub zoom_axis: RwSignal<CategoryId>,
    /// 既定のズーム軸（taxonomy の先頭根）。None⇄既定の対応に使う。URL には既定を出さない。
    default_zoom_axis: StoredValue<CategoryId>,
}

impl State {
    pub fn new(client: Client) -> Self {
        // 既定軸は taxonomy の先頭根。空なら使われないダミー id。
        let default_zoom_axis = client.read().with_untracked(|s| {
            s.taxonomy
                .roots()
                .first()
                .map(|d| d.id.clone())
                .unwrap_or_else(Platform::new_node_id)
        });
        Self {
            view: RwSignal::new(View::default()),
            filters: RwSignal::new(Filters::default()),
            zoom_axis: RwSignal::new(default_zoom_axis.clone()),
            default_zoom_axis: StoredValue::new(default_zoom_axis),
        }
    }

    /// URL → state: [`QueryState`] の値を signal へ反映する（同値ガードで余計な再描画を避ける）。
    /// None のフィールドは既定（facet / 先頭根）を表す。
    pub fn apply(&self, query: QueryState) {
        let view = query.view.unwrap_or_default();
        if self.view.get_untracked() != view {
            self.view.set(view);
        }
        if self.filters.get_untracked() != query.filters {
            self.filters.set(query.filters);
        }
        let zoom_axis = query.zoom_axis.unwrap_or_else(|| self.default_zoom_axis.get_value());
        if self.zoom_axis.get_untracked() != zoom_axis {
            self.zoom_axis.set(zoom_axis);
        }
    }

    /// state → URL: `base` に signal の現在値を載せた [`QueryState`] を返す。
    /// 既定値は URL を汚さぬよう None（省略）にする。`base` の rest（外部キー）等はそのまま残す。
    pub fn overlay(&self, mut base: QueryState) -> QueryState {
        let view = self.view.get();
        base.view = (view != View::default()).then_some(view);
        base.filters = self.filters.get();
        let zoom_axis = self.zoom_axis.get();
        base.zoom_axis = (zoom_axis != self.default_zoom_axis.get_value()).then_some(zoom_axis);
        base
    }
}
