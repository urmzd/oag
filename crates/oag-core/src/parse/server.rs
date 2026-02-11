use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A server variable for URL templates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerVariable {
    pub default: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

/// A server URL definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Server {
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub variables: IndexMap<String, ServerVariable>,
}
