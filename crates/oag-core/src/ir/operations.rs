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
}

/// A resolved response.
#[derive(Debug, Clone)]
pub struct IrResponse {
    pub response_type: IrType,
    pub description: Option<String>,
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
