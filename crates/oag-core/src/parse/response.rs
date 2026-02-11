use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::media_type::MediaType;

/// A response definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub description: String,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub content: IndexMap<String, MediaType>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, serde_json::Value>,
}

/// A reference or inline response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseOrRef {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Response(Response),
}
