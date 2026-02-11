use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A JSON Schema type keyword value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
    Null,
}

/// The `type` field can be a single type or an array of types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeSet {
    Single(SchemaType),
    Multiple(Vec<SchemaType>),
}

/// A reference or inline schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaOrRef {
    Ref {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    Schema(Box<Schema>),
}

/// Discriminator for polymorphic schemas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Discriminator {
    #[serde(rename = "propertyName")]
    pub property_name: String,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub mapping: IndexMap<String, String>,
}

/// A JSON Schema object (OpenAPI 3.2 superset).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<TypeSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    // Object properties
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, SchemaOrRef>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,

    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<AdditionalProperties>,

    // Array items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaOrRef>>,

    // Composition
    #[serde(rename = "allOf", default, skip_serializing_if = "Vec::is_empty")]
    pub all_of: Vec<SchemaOrRef>,

    #[serde(rename = "oneOf", default, skip_serializing_if = "Vec::is_empty")]
    pub one_of: Vec<SchemaOrRef>,

    #[serde(rename = "anyOf", default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<SchemaOrRef>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<Discriminator>,

    // Enum values
    #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<serde_json::Value>,

    // Numeric constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "exclusiveMinimum", skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<f64>,
    #[serde(rename = "exclusiveMaximum", skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<f64>,

    // String constraints
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u64>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    // Array constraints
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    #[serde(rename = "uniqueItems", skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,

    // Read/Write only
    #[serde(rename = "readOnly", skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(rename = "writeOnly", skip_serializing_if = "Option::is_none")]
    pub write_only: Option<bool>,

    // Const
    #[serde(rename = "const", skip_serializing_if = "Option::is_none")]
    pub const_value: Option<serde_json::Value>,

    // Example
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

/// `additionalProperties` can be a boolean or a schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdditionalProperties {
    Bool(bool),
    Schema(Box<SchemaOrRef>),
}
