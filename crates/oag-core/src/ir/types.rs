use std::fmt;

/// A fully resolved, generator-ready intermediate representation of an OpenAPI spec.
#[derive(Debug, Clone)]
pub struct IrSpec {
    pub info: IrInfo,
    pub servers: Vec<IrServer>,
    pub schemas: Vec<IrSchema>,
    pub operations: Vec<IrOperation>,
    pub modules: Vec<IrModule>,
}

/// API metadata.
#[derive(Debug, Clone)]
pub struct IrInfo {
    pub title: String,
    pub description: Option<String>,
    pub version: String,
}

/// A server URL.
#[derive(Debug, Clone)]
pub struct IrServer {
    pub url: String,
    pub description: Option<String>,
}

/// A module groups operations by tag.
#[derive(Debug, Clone)]
pub struct IrModule {
    pub name: NormalizedName,
    pub operations: Vec<usize>, // indices into IrSpec.operations
}

/// A name with multiple casing variants pre-computed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedName {
    pub original: String,
    pub pascal_case: String,
    pub camel_case: String,
    pub snake_case: String,
    pub screaming_snake: String,
}

impl fmt::Display for NormalizedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

// Re-export schema and operation types for convenience
pub use super::operations::*;
pub use super::schemas::*;
