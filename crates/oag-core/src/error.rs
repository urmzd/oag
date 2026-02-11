use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unsupported OpenAPI version: {0}")]
    UnsupportedVersion(String),

    #[error("missing required field: {0}")]
    MissingField(String),
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("unresolved reference: {0}")]
    UnresolvedRef(String),

    #[error("circular reference detected: {0}")]
    CircularRef(String),

    #[error("invalid reference format: {0}")]
    InvalidRefFormat(String),

    #[error("reference target not found: {0}")]
    RefTargetNotFound(String),
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("resolve error: {0}")]
    Resolve(#[from] ResolveError),

    #[error("transform failed: {0}")]
    Other(String),
}
