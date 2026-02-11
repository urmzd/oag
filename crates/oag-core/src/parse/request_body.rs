use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::media_type::MediaType;

/// A request body definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub content: IndexMap<String, MediaType>,

    #[serde(default)]
    pub required: bool,
}

/// A reference or inline request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestBodyOrRef {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    RequestBody(RequestBody),
}
