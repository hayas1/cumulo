include!(concat!(env!("OUT_DIR"), "/i18n/mod.rs"));

mod app;
mod category;
mod client;
mod platform;
mod query;
mod resource;
mod shared;
mod storage;
mod views;

pub use app::{Root, RootLocalStore};
pub use category::{CategoryAttribute, CategoryId};
pub use client::Client;
pub use platform::Platform;
pub use resource::{ResourceAttribute, ResourceId};
pub use storage::{DynStore, LocalStore, Store, LOCAL_STORE};
