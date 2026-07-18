use crate::category::{CategoryAttribute, CategoryId};
use crate::i18n::*;
use crate::platform::Platform;
use crate::resource::ResourceAttribute;
use crate::storage::{DynStore, SaveError};
use cumulo_model::{Bipartite, Forest};
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct Client {
    bipartite: RwSignal<Bipartite<ResourceAttribute, CategoryAttribute>>,
    store: &'static DynStore,
    toast: RwSignal<Option<String>>,
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

    pub fn toast(&self) -> RwSignal<Option<String>> {
        self.toast
    }

    pub fn notify(&self, message: impl Into<String>) {
        self.toast.set(Some(message.into()));
    }

    pub fn save(&self) {
        if let Err(e) = self.store.save(&self.bipartite.get_untracked()) {
            web_sys::console::warn_1(&format!("[cumulo] {e}").into());
            let i18n = use_i18n();
            self.notify(match e {
                SaveError::Invalid(_) => t_string!(i18n, save_failed_invalid),
                SaveError::Storage(_) => t_string!(i18n, save_failed_storage),
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
