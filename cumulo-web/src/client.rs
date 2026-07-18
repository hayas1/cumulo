use crate::category::{CategoryAttribute, CategoryId};
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use crate::shared::Toast;
use crate::storage::{DynStore, SaveError};
use cumulo_model::{Bipartite, Forest};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct Client {
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    store: &'static DynStore,
    toast: RwSignal<Option<Toast>>,
}

impl Client {
    pub fn new(store: &'static DynStore) -> Self {
        let bipartite = RwSignal::new(store.load());
        Self {
            bipartite,
            store,
            toast: RwSignal::new(None),
        }
    }

    pub fn read(&self) -> ReadSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite.read_only()
    }

    pub fn signal(&self) -> RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>> {
        self.bipartite
    }

    pub fn toast(&self) -> RwSignal<Option<Toast>> {
        self.toast
    }

    pub fn notify(&self, toast: Toast) {
        self.toast.set(Some(toast));
    }

    pub fn save(&self) {
        if let Err(e) = self.store.save(&self.bipartite.get_untracked()) {
            web_sys::console::warn_1(&format!("[cumulo] {e}").into());
            self.notify(match e {
                SaveError::Invalid(_) => Toast::SaveFailedInvalid,
                SaveError::Storage(_) => Toast::SaveFailedStorage,
            });
        }
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
