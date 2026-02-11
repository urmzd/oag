use oag_core::parse;

const SSE_CHAT: &str = include_str!("fixtures/sse-chat.yaml");
const PETSTORE: &str = include_str!("fixtures/petstore-3.2.yaml");
const MIXED: &str = include_str!("fixtures/mixed-endpoints.yaml");

#[test]
fn parse_sse_chat_yaml() {
    let spec = parse::from_yaml(SSE_CHAT).expect("should parse sse-chat.yaml");
    assert_eq!(spec.openapi, "3.2.0");
    assert_eq!(spec.info.title, "AI Chat API");
    assert_eq!(spec.paths.len(), 5);

    // Check that the chat completions endpoint has both JSON and SSE responses
    let chat_path = spec
        .paths
        .get("/chat/completions")
        .expect("should have /chat/completions");
    let post = chat_path.post.as_ref().expect("should have POST");
    assert_eq!(post.operation_id.as_deref(), Some("createChatCompletion"));

    let responses = &post.responses;
    let ok = responses.get("200").expect("should have 200 response");
    match ok {
        oag_core::parse::response::ResponseOrRef::Response(r) => {
            assert!(r.content.contains_key("application/json"));
            assert!(r.content.contains_key("text/event-stream"));

            // Check itemSchema on SSE
            let sse = r.content.get("text/event-stream").unwrap();
            assert!(sse.item_schema.is_some(), "SSE should have itemSchema");
        }
        _ => panic!("expected inline response"),
    }
}

#[test]
fn parse_petstore_yaml() {
    let spec = parse::from_yaml(PETSTORE).expect("should parse petstore");
    assert_eq!(spec.openapi, "3.2.0");
    assert_eq!(spec.info.title, "Petstore");
    assert_eq!(spec.paths.len(), 3);

    let components = spec.components.as_ref().expect("should have components");
    assert_eq!(components.schemas.len(), 4);
}

#[test]
fn parse_mixed_31_yaml() {
    let spec = parse::from_yaml(MIXED).expect("should parse 3.1 spec");
    assert_eq!(spec.openapi, "3.1.0");
    assert_eq!(spec.paths.len(), 3);
}

#[test]
fn parse_invalid_version() {
    let yaml = r#"
openapi: "2.0.0"
info:
  title: Test
  version: "1.0"
paths: {}
"#;
    let result = parse::from_yaml(yaml);
    assert!(result.is_err());
}

#[test]
fn parse_components_schemas() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let components = spec.components.as_ref().unwrap();

    // Check ChatMessage enum
    let chat_msg = components.schemas.get("ChatMessage").unwrap();
    match chat_msg {
        oag_core::parse::schema::SchemaOrRef::Schema(s) => {
            let role_prop = s.properties.get("role").unwrap();
            match role_prop {
                oag_core::parse::schema::SchemaOrRef::Schema(role_schema) => {
                    assert_eq!(role_schema.enum_values.len(), 3);
                }
                _ => panic!("expected inline schema for role"),
            }
        }
        _ => panic!("expected inline schema"),
    }
}

#[test]
fn parse_item_schema_one_of() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let path = spec.paths.get("/chat/completions/stream").unwrap();
    let post = path.post.as_ref().unwrap();
    let resp = post.responses.get("200").unwrap();
    match resp {
        oag_core::parse::response::ResponseOrRef::Response(r) => {
            let sse = r.content.get("text/event-stream").unwrap();
            let item_schema = sse.item_schema.as_ref().unwrap();
            match item_schema {
                oag_core::parse::schema::SchemaOrRef::Schema(s) => {
                    assert_eq!(s.one_of.len(), 2, "itemSchema should have 2 oneOf variants");
                }
                _ => panic!("expected inline itemSchema"),
            }
        }
        _ => panic!("expected inline response"),
    }
}
