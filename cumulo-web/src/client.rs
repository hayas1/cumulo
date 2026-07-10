use crate::category::{CategoryAttribute, CategoryId};
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use crate::storage::DynStore;
use cumulo_model::{Bipartite, Forest};
use leptos::prelude::*;

/// データ源（メモリ上の二部グラフ signal）と永続化 backend [`Store`] を束ねた
/// prop-drill 用ハンドル。`Copy` なので Leptos の prop として signal と同じ感覚で配れる。
/// store を trait 越しに持つことで、永続化先の差し替え点を `Client` の中だけに閉じ込める。
#[derive(Clone, Copy)]
pub struct Client {
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    store: &'static DynStore,
}

impl Client {
    /// 与えた [`Store`] から初期データを読み込んで生成する。保存先はこの `store` に閉じる。
    pub fn new(store: &'static DynStore) -> Self {
        let bipartite = RwSignal::new(store.load());
        Self { bipartite, store }
    }

    /// 読み取りのみの箇所向け。read-only であることを型で表明する。
    pub fn read(&self) -> ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite.read_only()
    }

    /// 更新サイトが多くインラインで `update` したい箇所向けに生 signal を渡す。
    /// 永続化は呼び出し側が [`Client::save`] で行う。
    pub fn signal(&self) -> RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite
    }

    /// 現在の状態を永続化する。
    pub fn save(&self) {
        self.store.save(&self.bipartite.get_untracked());
    }

    /// 変更を適用して永続化する（update と save を 1 箇所に集約）。
    pub fn update(&self, f: impl FnOnce(&mut Bipartite<ResourceAttribute, CategoryAttribute>)) {
        self.bipartite.update(f);
        self.save();
    }

    /// 丸ごと差し替えて永続化する（import 等）。
    pub fn set(&self, bipartite: Bipartite<ResourceAttribute, CategoryAttribute>) {
        self.bipartite.set(bipartite);
        self.save();
    }

    /// 永続化データを消去し、初期データに戻す。
    pub fn clear(&self) {
        self.bipartite.set(self.store.clear());
    }

    /// 既定のズーム軸（taxonomy の先頭根）。root が無ければダミー id。
    /// URL に zoom_axis が無い/未解決のときのフォールバックに使う。
    pub fn default_zoom_axis(&self) -> CategoryId {
        self.bipartite
            .with_untracked(|s| s.taxonomy.roots().first().map(|d| d.id.clone()))
            .unwrap_or_else(Platform::new_node_id)
    }
}
