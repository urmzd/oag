use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::parameter::ParameterOrRef;
use super::request_body::RequestBodyOrRef;
use super::response::ResponseOrRef;
use super::schema::SchemaOrRef;
use super::security::SecurityScheme;

/// Components object holding reusable definitions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Components {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub schemas: IndexMap<String, SchemaOrRef>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub responses: IndexMap<String, ResponseOrRef>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub parameters: IndexMap<String, ParameterOrRef>,

    #[serde(
        rename = "requestBodies",
        default,
        skip_serializing_if = "IndexMap::is_empty"
    )]
    pub request_bodies: IndexMap<String, RequestBodyOrRef>,

    #[serde(
        rename = "securitySchemes",
        default,
        skip_serializing_if = "IndexMap::is_empty"
    )]
    pub security_schemes: IndexMap<String, SecurityScheme>,
}
