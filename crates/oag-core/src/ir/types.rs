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

impl IrSpec {
    /// Look up a named schema by its PascalCase name.
    pub fn find_schema(&self, name: &str) -> Option<&IrSchema> {
        self.schemas.iter().find(|s| s.name().pascal_case == name)
    }

    /// Follow `IrType::Ref` (and aliases of refs) to the type it stands for.
    ///
    /// A `$ref` to a string enum or to an alias of a primitive is the same
    /// wire value as the inline schema, so callers that decide how to
    /// serialize a value (query parameters, mock values) must look through the
    /// reference instead of treating every `Ref` as an object. Enums resolve to
    /// a union of their literal values; objects and unions stay as the `Ref`.
    pub fn resolve_type<'a>(&'a self, ty: &'a IrType) -> std::borrow::Cow<'a, IrType> {
        use std::borrow::Cow;
        let mut current = Cow::Borrowed(ty);
        // Bounded walk so a self-referential alias cannot loop forever.
        for _ in 0..16 {
            let IrType::Ref(name) = current.as_ref() else {
                return current;
            };
            match self.find_schema(name) {
                Some(IrSchema::Alias(a)) => current = Cow::Borrowed(&a.target),
                Some(IrSchema::Enum(e)) => {
                    let variants: Vec<IrType> = e
                        .variants
                        .iter()
                        .map(|v| match v {
                            IrEnumVariant::String(s) => IrType::StringLiteral(s.clone()),
                            IrEnumVariant::Integer(i) => IrType::IntegerLiteral(*i),
                        })
                        .collect();
                    return Cow::Owned(match variants.len() {
                        0 => IrType::String,
                        1 => variants.into_iter().next().unwrap(),
                        _ => IrType::Union(variants),
                    });
                }
                _ => return current,
            }
        }
        current
    }
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
