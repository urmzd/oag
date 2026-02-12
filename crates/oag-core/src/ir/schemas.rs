use super::types::NormalizedName;

/// A resolved schema in the IR.
#[derive(Debug, Clone)]
pub enum IrSchema {
    Object(IrObjectSchema),
    Enum(IrEnumSchema),
    Alias(IrAliasSchema),
    Union(IrUnionSchema),
}

impl IrSchema {
    pub fn name(&self) -> &NormalizedName {
        match self {
            IrSchema::Object(o) => &o.name,
            IrSchema::Enum(e) => &e.name,
            IrSchema::Alias(a) => &a.name,
            IrSchema::Union(u) => &u.name,
        }
    }
}

/// An object schema with typed fields.
#[derive(Debug, Clone)]
pub struct IrObjectSchema {
    pub name: NormalizedName,
    pub description: Option<String>,
    pub fields: Vec<IrField>,
    pub additional_properties: Option<IrType>,
}

/// A field on an object schema.
#[derive(Debug, Clone)]
pub struct IrField {
    pub name: NormalizedName,
    pub original_name: String,
    pub field_type: IrType,
    pub required: bool,
    pub description: Option<String>,
    pub read_only: bool,
    pub write_only: bool,
}

/// A string enum schema.
#[derive(Debug, Clone)]
pub struct IrEnumSchema {
    pub name: NormalizedName,
    pub description: Option<String>,
    pub variants: Vec<String>,
}

/// A type alias (e.g., `type Foo = string`).
#[derive(Debug, Clone)]
pub struct IrAliasSchema {
    pub name: NormalizedName,
    pub description: Option<String>,
    pub target: IrType,
}

/// A union type (oneOf / anyOf).
#[derive(Debug, Clone)]
pub struct IrUnionSchema {
    pub name: NormalizedName,
    pub description: Option<String>,
    pub variants: Vec<IrType>,
    pub discriminator: Option<IrDiscriminator>,
}

/// Discriminator for union types.
#[derive(Debug, Clone)]
pub struct IrDiscriminator {
    pub property_name: String,
    pub mapping: Vec<(String, String)>,
}

/// A resolved type reference.
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    String,
    StringLiteral(String),
    Number,
    Integer,
    Boolean,
    Null,
    Array(Box<IrType>),
    Object(Vec<(String, IrType, bool)>), // inline object: (name, type, required)
    Map(Box<IrType>),                    // Record<string, T>
    Ref(String),                         // reference to a named schema (PascalCase)
    Union(Vec<IrType>),
    Any,
    Void,
    DateTime,
    Binary,
}
