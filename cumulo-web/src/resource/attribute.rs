use cumulo_model::{Id, Resource};
use serde::{Deserialize, Serialize};

use crate::category::CategoryAttribute;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceAttribute {
    pub console_url: String,
    pub created_at: Option<String>,
    pub freq: u32,
}

pub type ResourceId = Id<Resource<ResourceAttribute, CategoryAttribute>>;
