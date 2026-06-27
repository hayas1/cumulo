use crate::category::CategoryAttribute;
use crate::resource::ResourceAttribute;
use crate::storage::StorageClient;
use cumulo_model::Bipartite;
use leptos::prelude::*;

/// データ源（メモリ上の二部グラフ signal）と永続化 backend [`StorageClient`] を束ねた
/// prop-drill 用ハンドル。`Copy` なので Leptos の prop として signal と同じ感覚で配れる。
/// storage を内部に隠すことで、server 化時の差し替え点を `Client` の中だけに閉じ込める。
#[derive(Clone, Copy)]
pub struct Client {
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    storage: StorageClient,
}

impl Client {
    /// 永続化層から初期データを読み込んで生成する。
    pub fn load() -> Self {
        let storage = StorageClient;
        let bipartite = RwSignal::new(storage.load());
        Self { bipartite, storage }
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
        self.storage.save(&self.bipartite.get_untracked());
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
        self.bipartite.set(self.storage.clear());
    }
}
