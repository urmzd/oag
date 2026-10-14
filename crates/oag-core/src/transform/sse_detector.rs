use crate::ir::{IrResponse, IrResponseFormat, IrReturnType, IrSseReturn, IrType};
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
    let Some((status_code, response)) = success_response else {
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
            let sse_return = build_sse_return(operation_id, sse_mt, json_mt, status_code);
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
                status_code,
                format: IrResponseFormat::Json,
            })
        }
        (None, None) => {
            // Any other content type: classify it so binary bodies are read as
            // bytes (and typed as such) instead of being decoded as text.
            if let Some((ct, mt)) = content.first() {
                let schema_type = mt.schema.as_ref().map(schema_or_ref_to_ir_type);
                let format = IrResponseFormat::classify(ct, schema_type.as_ref());
                let response_type = match (format, schema_type) {
                    (IrResponseFormat::Binary, _) => IrType::Binary,
                    (IrResponseFormat::Text, Some(IrType::Any) | None) => IrType::String,
                    (_, Some(t)) => t,
                    (_, None) => IrType::Any,
                };
                IrReturnType::Standard(IrResponse {
                    response_type,
                    description: None,
                    status_code,
                    format,
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
    status_code: u16,
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
            status_code,
            format: IrResponseFormat::Json,
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

fn find_success_response(
    responses: &IndexMap<String, ResponseOrRef>,
) -> Option<(u16, &ResponseOrRef)> {
    // The success type is chosen from 2xx responses only, in priority order:
    // the common explicit codes first, then any other 2xx code (declared
    // order), then 2XX wildcards.
    //
    // OpenAPI 3.2 allows any 2xx response to carry a body — e.g. 200 OK, but
    // also 202 Accepted returning the queued job or 203 Non-Authoritative
    // Information. Earlier versions only looked at 200/201, so a 202-only
    // operation was mapped to a void return even when it declared a body.
    //
    // `default` (like 4xx/5xx) describes errors whenever a 2xx is declared, so
    // it never competes with a 2xx: a bodyless 204 next to a `default` error
    // body is a void operation, not one returning the error type. `default` is
    // only the success response when the operation declares no 2xx at all.
    const PREFERRED: [&str; 4] = ["200", "201", "202", "203"];
    let ordered: Vec<(&String, &ResponseOrRef)> = PREFERRED
        .iter()
        .filter_map(|code| responses.get_key_value(*code))
        .chain(
            responses
                .iter()
                .filter(|(code, _)| is_2xx(code) && !PREFERRED.contains(&code.as_str())),
        )
        .chain(responses.get_key_value("2XX"))
        .chain(responses.get_key_value("2xx"))
        .collect();

    if ordered.is_empty() {
        return responses
            .get_key_value("default")
            .map(|(code, response)| (status_from_code(code), response));
    }

    // Prefer a candidate that actually declares a body, so a body on a lower
    // priority code (e.g. a 202) is never dropped in favour of an empty higher
    // priority code (e.g. a bodyless 200). Fall back to the highest priority
    // candidate, which yields a void return when no 2xx carries a body.
    ordered
        .iter()
        .find(|(_, response)| has_body(response))
        .or(ordered.first())
        .map(|(code, response)| (status_from_code(code), *response))
}

/// Parse a response key into an HTTP status code. Wildcards (`2XX`) and
/// `default` collapse to `200`.
fn status_from_code(code: &str) -> u16 {
    code.parse().unwrap_or(200)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn responses(yaml: &str) -> IndexMap<String, ResponseOrRef> {
        serde_yaml_ng::from_str(yaml).expect("responses should parse")
    }

    #[test]
    fn bodyless_204_with_default_error_is_void() {
        // #28: `default` must not override a bodyless 2xx as the success type.
        let r = responses(
            r##"
"204": { description: Deleted }
default:
  description: Error
  content: { application/json: { schema: { $ref: "#/components/schemas/ErrorBody" } } }
"##,
        );
        assert!(matches!(
            detect_return_type("deleteItem", &r),
            IrReturnType::Void
        ));
    }

    #[test]
    fn error_codes_never_shape_the_success_type() {
        let r = responses(
            r##"
"200": { description: OK }
"404":
  description: Not found
  content: { application/json: { schema: { type: string } } }
"##,
        );
        assert!(matches!(detect_return_type("op", &r), IrReturnType::Void));
    }

    #[test]
    fn default_is_the_success_response_when_no_2xx_is_declared() {
        let r = responses(
            r##"
default:
  description: Anything
  content: { application/json: { schema: { type: string } } }
"##,
        );
        match detect_return_type("op", &r) {
            IrReturnType::Standard(resp) => {
                assert_eq!(resp.response_type, IrType::String);
                assert_eq!(resp.status_code, 200);
            }
            other => panic!("expected a standard return, got {other:?}"),
        }
    }
}
