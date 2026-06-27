use crate::category::CategoryAttribute;
use crate::resource::ResourceAttribute;
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

/// localStorage を backend とする永続化クライアント。
/// 現状フィールドを持たないが、static ではなく値（メソッドレシーバ）として持ち回ることで、
/// server 化時に接続情報などを載せて差し替える 1 点にする。
#[derive(Clone, Copy)]
pub struct StorageClient;

impl StorageClient {
    pub fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
        match LocalStorage::get::<Bipartite<ResourceAttribute, CategoryAttribute>>(STORAGE_KEY) {
            Ok(bipartite) => match bipartite.validated() {
                Ok(b) => b,
                Err(errs) => {
                    web_sys::console::warn_1(
                        &format!("[cumulo] loaded data is invalid ({errs}), using demo").into(),
                    );
                    Self::demo()
                }
            },
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[cumulo] load failed ({e:?}), using demo").into(),
                );
                Self::demo()
            }
        }
    }

    pub fn save(&self, bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
    }

    pub fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }

    fn demo() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        ExportData::<ResourceAttribute, CategoryAttribute>::parse(cumulo_model::demo::CLOUD)
            .expect("demo data must be valid")
    }
}
