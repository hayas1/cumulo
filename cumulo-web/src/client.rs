use crate::category::{CategoryAttribute, CategoryId};
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use crate::storage::DynStore;
use cumulo_model::{Bipartite, Forest};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct Client {
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    store: &'static DynStore,
}

impl Client {
    pub fn new(store: &'static DynStore) -> Self {
        let bipartite = RwSignal::new(store.load());
        Self { bipartite, store }
    }

    pub fn read(&self) -> ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite.read_only()
    }

    pub fn signal(&self) -> RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite
    }

    pub fn save(&self) {
        self.store.save(&self.bipartite.get_untracked());
    }

    pub fn update(&self, f: impl FnOnce(&mut Bipartite<ResourceAttribute, CategoryAttribute>)) {
        self.bipartite.update(f);
        self.save();
    }

    pub fn set(&self, bipartite: Bipartite<ResourceAttribute, CategoryAttribute>) {
        self.bipartite.set(bipartite);
        self.save();
    }

    pub fn clear(&self) {
        self.bipartite.set(self.store.clear());
    }

    pub fn default_zoom_axis(&self) -> CategoryId {
        self.bipartite
            .with_untracked(|s| s.taxonomy.roots().first().map(|d| d.id.clone()))
            .unwrap_or_else(Platform::new_node_id)
    }
}
