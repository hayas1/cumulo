use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// エンティティや属性の ID を表すファントム型付き newtype。
/// T はマーカーとして機能し、異なる種類の ID の混在をコンパイル時に防ぐ。
/// `fn() -> T` を使うことで T: Send + Sync なしに Id<T>: Send + Sync となる。
/// Clone/Debug は derive ではなく手動実装 — derive は T: Clone/Debug 境界を生成するが、
/// T は phantom marker なので T のトレイト境界を Id<T> に波及させるべきではないため。
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id<T>(pub String, #[serde(skip)] PhantomData<fn() -> T>);

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id(self.0.clone(), PhantomData)
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}

impl<T> Id<T> {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Id(String::new(), PhantomData)
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> From<String> for Id<T> {
    fn from(s: String) -> Self {
        Id(s, PhantomData)
    }
}

impl<T> From<&str> for Id<T> {
    fn from(s: &str) -> Self {
        Id(s.to_string(), PhantomData)
    }
}

impl<T> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> std::ops::Deref for Id<T> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<T> std::borrow::Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        &self.0
    }
}
