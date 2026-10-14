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
    /// The literal default value (from `const`, a single-value `enum`, or a
    /// scalar `default`), carried as a literal `IrType` so packs can render it
    /// via the existing type-map literal templates. `None` when the property has
    /// no default. Required-ness is unaffected by this value.
    pub default_repr: Option<IrType>,
    /// A string field whose value is encoded content with a known schema
    /// (`contentEncoding` / `contentMediaType` / `contentSchema`), e.g. a
    /// base64-encoded JSON document. The field stays a string on the wire;
    /// packs use this to emit typed decode/encode helpers.
    pub content: Option<IrContent>,
}

/// Encoded content carried inside a string (JSON Schema 2020-12 content keywords).
#[derive(Debug, Clone, PartialEq)]
pub struct IrContent {
    /// `contentEncoding`, e.g. `base64` or `base64url`. `None` means the string
    /// is the content itself (e.g. a JSON document in a string).
    pub encoding: Option<String>,
    /// `contentMediaType`, e.g. `application/json`.
    pub media_type: Option<String>,
    /// The type of the decoded value (`contentSchema`).
    pub schema: IrType,
}

impl IrContent {
    /// The codec packs use to turn the wire string into the decoded value:
    /// `json`, `base64-json` or `base64url-json`.
    pub fn codec(&self) -> &'static str {
        match self
            .encoding
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("base64") => "base64-json",
            Some("base64url") => "base64url-json",
            _ => "json",
        }
    }

    /// Build content info from the content keywords of a string schema.
    ///
    /// Only JSON content with a `contentSchema` gets typed helpers: other media
    /// types have no schema to decode into. A `contentEncoding` other than
    /// base64/base64url is not decodable portably, so it yields `None` too.
    pub fn from_keywords(
        encoding: Option<&str>,
        media_type: Option<&str>,
        schema: Option<IrType>,
    ) -> Option<Self> {
        let schema = schema?;
        let json = media_type.is_none_or(|mt| {
            let mt = mt
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            mt == "application/json" || mt.ends_with("+json")
        });
        let decodable = encoding
            .is_none_or(|e| matches!(e.to_ascii_lowercase().as_str(), "base64" | "base64url"));
        (json && decodable).then(|| IrContent {
            encoding: encoding.map(str::to_string),
            media_type: media_type.map(str::to_string),
            schema,
        })
    }
}

/// A single variant value in an enum schema.
#[derive(Debug, Clone)]
pub enum IrEnumVariant {
    String(String),
    Integer(i64),
}

/// An enum schema (string or integer variants).
#[derive(Debug, Clone)]
pub struct IrEnumSchema {
    pub name: NormalizedName,
    pub description: Option<String>,
    pub variants: Vec<IrEnumVariant>,
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
    IntegerLiteral(i64),
    Union(Vec<IrType>),
    Intersection(Vec<IrType>),
    Any,
    Void,
    DateTime,
    Binary,
}
