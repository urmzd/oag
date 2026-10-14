use super::schemas::IrType;
use super::types::NormalizedName;

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
    Trace,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Head => "HEAD",
            HttpMethod::Trace => "TRACE",
        }
    }
}

/// A fully resolved API operation.
#[derive(Debug, Clone)]
pub struct IrOperation {
    pub name: NormalizedName,
    pub method: HttpMethod,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<IrParameter>,
    pub request_body: Option<IrRequestBody>,
    pub return_type: IrReturnType,
    pub deprecated: bool,
}

/// What an operation returns.
#[derive(Debug, Clone)]
pub enum IrReturnType {
    /// Standard JSON response.
    Standard(IrResponse),
    /// Server-Sent Events stream.
    Sse(IrSseReturn),
    /// No response body (204, etc).
    Void,
}

/// SSE return type with event schema info.
#[derive(Debug, Clone)]
pub struct IrSseReturn {
    /// The type of each event yielded by the stream.
    pub event_type: IrType,
    /// If the itemSchema has oneOf, these are the individual variant types.
    pub variants: Vec<IrType>,
    /// The union type name for the stream event (e.g., `CreateChatCompletionStreamEvent`).
    pub event_type_name: Option<String>,
    /// Whether the endpoint also has a JSON response (dual endpoint).
    pub also_has_json: bool,
    /// The JSON response type if this is a dual endpoint.
    pub json_response: Option<IrResponse>,
    /// OpenAPI 3.2: when `itemSchema` describes the whole SSE event (an object
    /// with `data` and optionally `event`, `id`, `retry`), one entry per event
    /// variant, keyed by the SSE `event` name. Empty when `itemSchema` (or the
    /// legacy `schema`) describes only the `data` payload.
    pub events: Vec<IrSseEvent>,
}

/// One variant of an SSE stream whose `itemSchema` describes the whole event.
#[derive(Debug, Clone)]
pub struct IrSseEvent {
    /// The `event:` names this variant matches. `["message"]` for events sent
    /// without an `event:` line; empty when any name matches (the `event`
    /// property is a plain string).
    pub names: Vec<String>,
    /// The decoded type of the `data` field.
    pub data_type: IrType,
    /// How `data` is decoded: `text` (the raw string), `json`, `base64-json` or
    /// `base64url-json` (see `IrContent::codec`).
    pub data_codec: String,
    pub description: Option<String>,
}

/// A resolved response.
#[derive(Debug, Clone)]
pub struct IrResponse {
    pub response_type: IrType,
    pub description: Option<String>,
    /// The HTTP status code this response is declared under (e.g. `202`).
    /// Wildcards (`2XX`) and `default` collapse to `200`. Server stubs use this
    /// to declare the correct status so a 202-with-body endpoint isn't emitted
    /// as a 200.
    pub status_code: u16,
    /// How the success body is encoded on the wire, so packs read it with the
    /// matching API (`response.json()`, `.text()`, `.blob()`).
    pub format: IrResponseFormat,
}

/// Wire format of a response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IrResponseFormat {
    /// JSON (`application/json`, `*/*+json`).
    #[default]
    Json,
    /// Text (`text/*`, or a string schema under a non-binary media type).
    Text,
    /// Raw bytes (`application/octet-stream`, `image/*`, `format: binary`, ...).
    Binary,
}

impl IrResponseFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            IrResponseFormat::Json => "json",
            IrResponseFormat::Text => "text",
            IrResponseFormat::Binary => "binary",
        }
    }

    /// Classify a media type and its schema.
    pub fn classify(content_type: &str, schema_type: Option<&IrType>) -> Self {
        let ct = content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if ct == "application/json" || ct.ends_with("+json") || ct.ends_with("/json") {
            return IrResponseFormat::Json;
        }
        if matches!(schema_type, Some(IrType::Binary)) {
            return IrResponseFormat::Binary;
        }
        if ct.starts_with("text/")
            || ct.ends_with("+xml")
            || ct == "application/xml"
            || ct == "application/x-www-form-urlencoded"
        {
            return IrResponseFormat::Text;
        }
        match schema_type {
            // A non-binary string under an opaque media type is still text.
            Some(IrType::String) => IrResponseFormat::Text,
            // Anything else declared with a JSON-ish schema keeps JSON parsing.
            Some(t) if !matches!(t, IrType::Any) => IrResponseFormat::Json,
            // octet-stream, image/*, application/pdf, */*, ... with no schema.
            _ => IrResponseFormat::Binary,
        }
    }
}

/// A resolved path/query/header parameter.
#[derive(Debug, Clone)]
pub struct IrParameter {
    pub name: NormalizedName,
    pub original_name: String,
    pub location: IrParameterLocation,
    pub param_type: IrType,
    pub required: bool,
    pub description: Option<String>,
    pub style: Option<String>,
    pub explode: Option<bool>,
}

/// Parameter location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

/// Encoding metadata for a single field in a multipart request body.
#[derive(Debug, Clone)]
pub struct IrFieldEncoding {
    pub field_name: String,
    pub content_type: Option<String>,
}

/// A resolved request body.
#[derive(Debug, Clone)]
pub struct IrRequestBody {
    pub body_type: IrType,
    pub required: bool,
    pub content_type: String,
    pub description: Option<String>,
    pub encoding: Option<Vec<IrFieldEncoding>>,
}
