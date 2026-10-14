use crate::ir::{
    IrContent, IrResponse, IrResponseFormat, IrReturnType, IrSseEvent, IrSseReturn, IrType,
};
use crate::parse::media_type::MediaType;
use crate::parse::response::ResponseOrRef;
use crate::parse::schema::{Schema, SchemaOrRef, SchemaType, TypeSet};

use super::name_normalizer::normalize_name;
use super::schema_resolver::{schema_content, schema_or_ref_to_ir_type};

use indexmap::IndexMap;

/// Component schemas, used to look through `$ref`s when inspecting an
/// `itemSchema`.
pub type ComponentSchemas = IndexMap<String, SchemaOrRef>;

/// Detect whether an operation's responses include SSE streaming.
/// Returns the appropriate `IrReturnType`.
pub fn detect_return_type(
    operation_id: &str,
    responses: &IndexMap<String, ResponseOrRef>,
) -> IrReturnType {
    detect_return_type_with_schemas(operation_id, responses, &ComponentSchemas::new())
}

/// Like [`detect_return_type`], resolving `$ref`s in an SSE `itemSchema`
/// against the given component schemas.
pub fn detect_return_type_with_schemas(
    operation_id: &str,
    responses: &IndexMap<String, ResponseOrRef>,
    schemas: &ComponentSchemas,
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
            let sse_return = build_sse_return(operation_id, sse_mt, json_mt, status_code, schemas);
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
    schemas: &ComponentSchemas,
) -> IrSseReturn {
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

    // OpenAPI 3.2: `itemSchema` describes the whole SSE event (`event`,
    // `data`, `id`, `retry`). Streams whose item schema has that shape are
    // typed as a union keyed by the event name.
    if let Some(events) = sse_mt
        .item_schema
        .as_ref()
        .and_then(|item| extract_sse_events(item, schemas))
    {
        return IrSseReturn {
            // The stream yields the event union (named below), never a bare
            // payload; per-event data types live in `events`.
            event_type: IrType::Any,
            variants: vec![],
            event_type_name: Some(event_union_name(operation_id, schemas)),
            also_has_json: json_response.is_some(),
            json_response,
            events,
        };
    }

    // Otherwise `itemSchema` (or the legacy `schema`) is the `data` payload.
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

    IrSseReturn {
        event_type,
        variants,
        event_type_name,
        also_has_json: json_response.is_some(),
        json_response,
        events: vec![],
    }
}

/// Name of the generated event union, avoiding a clash with a component schema.
fn event_union_name(operation_id: &str, schemas: &ComponentSchemas) -> String {
    let pascal = normalize_name(operation_id).pascal_case;
    let taken = |n: &str| schemas.keys().any(|k| normalize_name(k).pascal_case == n);
    let name = format!("{pascal}StreamEvent");
    if taken(&name) {
        format!("{pascal}StreamEventUnion")
    } else {
        name
    }
}

/// The SSE event fields defined by the HTML standard. An item schema whose
/// properties include `data` and are all among these describes the whole
/// event rather than its payload.
const SSE_EVENT_FIELDS: [&str; 4] = ["event", "data", "id", "retry"];

/// Follow `$ref`s to the referenced component schema.
fn resolve_schema<'a>(
    mut schema: &'a SchemaOrRef,
    schemas: &'a ComponentSchemas,
) -> Option<&'a Schema> {
    for _ in 0..16 {
        match schema {
            SchemaOrRef::Schema(s) => return Some(s),
            SchemaOrRef::Ref { ref_path } => {
                let name = ref_path.strip_prefix("#/components/schemas/")?;
                schema = schemas.get(name)?;
            }
        }
    }
    None
}

fn is_sse_event_shape(schema: &Schema) -> bool {
    schema.properties.contains_key("data")
        && schema
            .properties
            .keys()
            .all(|k| SSE_EVENT_FIELDS.contains(&k.as_str()))
}

/// Read an `itemSchema` as whole SSE events. Returns `None` unless the item
/// schema (or every `oneOf` / `anyOf` variant of it) has the event shape, so
/// specs that use `itemSchema` for the `data` payload keep their meaning.
fn extract_sse_events(item: &SchemaOrRef, schemas: &ComponentSchemas) -> Option<Vec<IrSseEvent>> {
    let schema = resolve_schema(item, schemas)?;
    let variants: Vec<&SchemaOrRef> = if !schema.one_of.is_empty() {
        schema.one_of.iter().collect()
    } else if !schema.any_of.is_empty() {
        schema.any_of.iter().collect()
    } else {
        vec![item]
    };
    variants
        .into_iter()
        .map(|v| {
            let s = resolve_schema(v, schemas)?;
            is_sse_event_shape(s).then(|| sse_event_from_schema(s, schemas))
        })
        .collect()
}

fn sse_event_from_schema(schema: &Schema, schemas: &ComponentSchemas) -> IrSseEvent {
    let names = match schema
        .properties
        .get("event")
        .and_then(|e| resolve_schema(e, schemas))
    {
        // No `event` property: the variant is sent without an `event:` line,
        // which the SSE standard dispatches as `message`.
        None if !schema.properties.contains_key("event") => vec!["message".to_string()],
        None => vec![],
        Some(e) => {
            if let Some(v) = e.const_value.as_ref().and_then(|v| v.as_str()) {
                vec![v.to_string()]
            } else {
                e.enum_values
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            }
        }
    };

    let data = schema.properties.get("data").expect("event shape has data");
    let (data_type, data_codec) = sse_data_type(data, schemas);

    IrSseEvent {
        names,
        data_type,
        data_codec,
        description: schema.description.clone(),
    }
}

/// The decoded type of an SSE `data` field and the codec that produces it.
fn sse_data_type(data: &SchemaOrRef, schemas: &ComponentSchemas) -> (IrType, String) {
    let SchemaOrRef::Schema(inline) = data else {
        // `data: { $ref: Payload }`: a JSON document of that schema. A ref to a
        // plain string schema is still raw text.
        let is_string = resolve_schema(data, schemas).is_some_and(is_plain_string);
        let codec = if is_string { "text" } else { "json" };
        return (schema_or_ref_to_ir_type(data), codec.to_string());
    };
    if let Some(content) = schema_content(inline) {
        return (content.schema.clone(), content.codec().to_string());
    }
    let json_media = inline
        .content_media_type
        .as_deref()
        .is_some_and(|mt| IrContent::from_keywords(None, Some(mt), Some(IrType::Any)).is_some());
    if json_media {
        return (IrType::Any, "json".to_string());
    }
    if is_plain_string(inline) {
        return (IrType::String, "text".to_string());
    }
    (schema_or_ref_to_ir_type(data), "json".to_string())
}

fn is_plain_string(schema: &Schema) -> bool {
    matches!(
        schema.schema_type,
        Some(TypeSet::Single(SchemaType::String))
    ) && schema.enum_values.is_empty()
        && schema.const_value.is_none()
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
