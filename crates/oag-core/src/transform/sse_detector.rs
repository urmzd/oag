use crate::ir::{IrResponse, IrReturnType, IrSseReturn, IrType};
use crate::parse::media_type::MediaType;
use crate::parse::response::ResponseOrRef;
use crate::parse::schema::SchemaOrRef;

use super::name_normalizer::normalize_name;
use super::schema_resolver::schema_or_ref_to_ir_type;

use indexmap::IndexMap;

/// Detect whether an operation's responses include SSE streaming.
/// Returns the appropriate `IrReturnType`.
pub fn detect_return_type(
    operation_id: &str,
    responses: &IndexMap<String, ResponseOrRef>,
) -> IrReturnType {
    let success_response = find_success_response(responses);
    let Some(response) = success_response else {
        return IrReturnType::Void;
    };

    let content = match response {
        ResponseOrRef::Response(r) => &r.content,
        ResponseOrRef::Ref { .. } => return IrReturnType::Void,
    };

    if content.is_empty() {
        return IrReturnType::Void;
    }

    let sse = content.get("text/event-stream");
    let json = content.get("application/json");

    match (sse, json) {
        (Some(sse_mt), json_mt) => {
            // SSE endpoint (possibly dual)
            let sse_return = build_sse_return(operation_id, sse_mt, json_mt);
            IrReturnType::Sse(sse_return)
        }
        (None, Some(json_mt)) => {
            // Standard JSON response
            let response_type = match &json_mt.schema {
                Some(s) => schema_or_ref_to_ir_type(s),
                None => IrType::Any,
            };
            let description = match response {
                ResponseOrRef::Response(r) => Some(r.description.clone()),
                _ => None,
            };
            IrReturnType::Standard(IrResponse {
                response_type,
                description,
            })
        }
        (None, None) => {
            // Try any other content type
            if let Some((_ct, mt)) = content.first() {
                let response_type = match &mt.schema {
                    Some(s) => schema_or_ref_to_ir_type(s),
                    None => IrType::Any,
                };
                IrReturnType::Standard(IrResponse {
                    response_type,
                    description: None,
                })
            } else {
                IrReturnType::Void
            }
        }
    }
}

fn build_sse_return(
    operation_id: &str,
    sse_mt: &MediaType,
    json_mt: Option<&MediaType>,
) -> IrSseReturn {
    // Extract event type from itemSchema (OpenAPI 3.2)
    let (event_type, variants, event_type_name) = match &sse_mt.item_schema {
        Some(item_schema) => extract_event_info(operation_id, item_schema),
        None => {
            // Fallback: try the schema field
            match &sse_mt.schema {
                Some(s) => (schema_or_ref_to_ir_type(s), vec![], None),
                None => (IrType::Any, vec![], None),
            }
        }
    };

    let json_response = json_mt.map(|mt| {
        let response_type = match &mt.schema {
            Some(s) => schema_or_ref_to_ir_type(s),
            None => IrType::Any,
        };
        IrResponse {
            response_type,
            description: None,
        }
    });

    IrSseReturn {
        event_type,
        variants,
        event_type_name,
        also_has_json: json_response.is_some(),
        json_response,
    }
}

fn extract_event_info(
    operation_id: &str,
    item_schema: &SchemaOrRef,
) -> (IrType, Vec<IrType>, Option<String>) {
    match item_schema {
        SchemaOrRef::Ref { .. } => {
            let ir_type = schema_or_ref_to_ir_type(item_schema);
            (ir_type, vec![], None)
        }
        SchemaOrRef::Schema(schema) => {
            if !schema.one_of.is_empty() {
                // Union of event types
                let variants: Vec<IrType> =
                    schema.one_of.iter().map(schema_or_ref_to_ir_type).collect();
                let event_name = format!("{}StreamEvent", normalize_name(operation_id).pascal_case);
                let event_type = IrType::Union(variants.clone());
                (event_type, variants, Some(event_name))
            } else {
                let ir_type = schema_or_ref_to_ir_type(item_schema);
                (ir_type, vec![], None)
            }
        }
    }
}

fn find_success_response(responses: &IndexMap<String, ResponseOrRef>) -> Option<&ResponseOrRef> {
    // Build the success candidates in priority order: the common explicit codes
    // first, then any other 2xx code (declared order), then 2XX wildcards, then
    // default.
    //
    // OpenAPI 3.2 allows any 2xx response to carry a body — e.g. 200 OK, but
    // also 202 Accepted returning the queued job or 203 Non-Authoritative
    // Information. Earlier versions only looked at 200/201, so a 202-only
    // operation was mapped to a void return even when it declared a body.
    const PREFERRED: [&str; 4] = ["200", "201", "202", "203"];
    let ordered = PREFERRED
        .iter()
        .filter_map(|code| responses.get(*code))
        .chain(
            responses
                .iter()
                .filter(|(code, _)| is_2xx(code) && !PREFERRED.contains(&code.as_str()))
                .map(|(_, r)| r),
        )
        .chain(responses.get("2XX"))
        .chain(responses.get("2xx"))
        .chain(responses.get("default"));

    // Prefer a candidate that actually declares a body, so a body on a lower
    // priority code (e.g. a 202) is never dropped in favour of an empty higher
    // priority code (e.g. a bodyless 200). Fall back to the highest priority
    // candidate, which yields a void return when nothing carries a body.
    let mut first = None;
    for response in ordered {
        if first.is_none() {
            first = Some(response);
        }
        if has_body(response) {
            return Some(response);
        }
    }
    first
}

/// Whether a response declares a body. References are treated as bodyless since
/// their content cannot be inspected here.
fn has_body(response: &ResponseOrRef) -> bool {
    match response {
        ResponseOrRef::Response(r) => !r.content.is_empty(),
        ResponseOrRef::Ref { .. } => false,
    }
}

/// Whether a response key is an explicit 2xx status code (e.g. `"204"`).
fn is_2xx(code: &str) -> bool {
    code.len() == 3 && code.starts_with('2') && code.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}
