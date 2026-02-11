pub mod components;
pub mod media_type;
pub mod operation;
pub mod parameter;
pub mod ref_resolve;
pub mod request_body;
pub mod response;
pub mod schema;
pub mod security;
pub mod server;
pub mod spec;

use crate::error::ParseError;
use spec::OpenApiSpec;

/// Parse an OpenAPI spec from YAML.
pub fn from_yaml(input: &str) -> Result<OpenApiSpec, ParseError> {
    let spec: OpenApiSpec = serde_yaml_ng::from_str(input)?;
    validate_version(&spec)?;
    Ok(spec)
}

/// Parse an OpenAPI spec from JSON.
pub fn from_json(input: &str) -> Result<OpenApiSpec, ParseError> {
    let spec: OpenApiSpec = serde_json::from_str(input)?;
    validate_version(&spec)?;
    Ok(spec)
}

fn validate_version(spec: &OpenApiSpec) -> Result<(), ParseError> {
    if !spec.openapi.starts_with("3.") {
        return Err(ParseError::UnsupportedVersion(spec.openapi.clone()));
    }
    Ok(())
}
