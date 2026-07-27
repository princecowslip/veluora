use serde::{Deserialize, Serialize};

use crate::ids::TagId;

/// Tag namespaces, per `docs/13-data-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagNamespace {
    Creator,
    Character,
    Series,
    Genre,
    Format,
    Language,
    Source,
    User,
    Technical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: TagId,
    pub namespace: TagNamespace,
    pub normalized_value: String,
    pub display_value: String,
    pub aliases: Vec<String>,
    pub safety_classification: Option<String>,
}

impl Tag {
    pub fn new(
        namespace: TagNamespace,
        normalized_value: impl Into<String>,
        display_value: impl Into<String>,
    ) -> Self {
        Self {
            id: TagId::new(),
            namespace,
            normalized_value: normalized_value.into(),
            display_value: display_value.into(),
            aliases: Vec::new(),
            safety_classification: None,
        }
    }
}
