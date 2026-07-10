use crate::category::CategoryAttribute;
use crate::resource::ResourceAttribute;
use cumulo_model::Bipartite;
use cumulo_model::ExportData;
use gloo_storage::{LocalStorage, Storage as GlooStorage};

const STORAGE_KEY: &str = "cumulo_store";

/// 二部グラフの永続化 backend。`Client` はこの trait 越しにしか保存先を触らないので、
/// localStorage・chrome.storage・server API などを差し替える 1 点になる。
/// `Client` を `Copy` に保つため実装は `&'static dyn Store` として持ち回る（値は状態を持たない前提）。
pub trait Store {
    fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute>;
    fn save(&self, bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>);
    fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute>;
}

/// `Client` が持ち回る store の型。`Client` は Leptos の Callback（`Send + Sync` 要求）へ
/// 載るので、trait object にも `Send + Sync` を課す（store は状態なしなので自明に満たす）。
pub type DynStore = dyn Store + Send + Sync;

/// localStorage を backend とする既定の [`Store`]。Pages 版・拡張版の全画面アプリが使う。
pub struct LocalStore;

/// アプリ既定の store。エントリ（`mount`）から `&LOCAL_STORE` として渡す。
pub static LOCAL_STORE: LocalStore = LocalStore;

impl LocalStore {
    fn demo() -> Bipartite<ResourceAttribute, CategoryAttribute> {
        ExportData::<ResourceAttribute, CategoryAttribute>::parse(cumulo_model::demo::CLOUD)
            .expect("demo data must be valid")
    }
}

impl Store for LocalStore {
    fn load(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
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

    fn save(&self, bipartite: &Bipartite<ResourceAttribute, CategoryAttribute>) {
        if let Err(e) = LocalStorage::set(STORAGE_KEY, bipartite) {
            web_sys::console::error_1(&format!("[cumulo] save failed: {e:?}").into());
        }
    }

    fn clear(&self) -> Bipartite<ResourceAttribute, CategoryAttribute> {
        LocalStorage::delete(STORAGE_KEY);
        Self::demo()
    }
}
