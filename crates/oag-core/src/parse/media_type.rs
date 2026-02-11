use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::schema::SchemaOrRef;

/// Encoding object for multipart requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Encoding {
    #[serde(rename = "contentType", skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explode: Option<bool>,
}

/// A media type object. The `item_schema` field is the OpenAPI 3.2 addition
/// for `text/event-stream` content types, defining the schema of individual
/// SSE events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaOrRef>,

    /// OpenAPI 3.2: Schema for individual items in streaming responses.
    /// Used with `text/event-stream` to define the shape of each SSE event.
    #[serde(rename = "itemSchema", skip_serializing_if = "Option::is_none")]
    pub item_schema: Option<SchemaOrRef>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub encoding: IndexMap<String, Encoding>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub examples: IndexMap<String, serde_json::Value>,
}
