//! resource ドメインの web 層の値型（属性・ID）。

use cumulo_model::{Id, Resource};
use serde::{Deserialize, Serialize};

use crate::category::CategoryAttribute;

/// Web 層が Resource に付与する値。
/// `#[serde(flatten)]` で JSON にインライン展開されるため、既存データと後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceAttribute {
    pub console_url: String,
    pub created_at: Option<String>,
    pub freq: u32,
}

pub type ResourceId = Id<Resource<ResourceAttribute, CategoryAttribute>>;
