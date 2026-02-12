use oag_core::parse;

const SSE_CHAT: &str = include_str!("fixtures/sse-chat.yaml");
const PETSTORE: &str = include_str!("fixtures/petstore-3.2.yaml");
const MIXED: &str = include_str!("fixtures/mixed-endpoints.yaml");
const ANTHROPIC: &str = include_str!("fixtures/anthropic-messages.yaml");
const PETSTORE_POLY: &str = include_str!("fixtures/petstore-polymorphic.yaml");

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

#[test]
fn parse_anthropic_messages_yaml() {
    let spec = parse::from_yaml(ANTHROPIC).expect("should parse anthropic-messages.yaml");
    assert_eq!(spec.openapi, "3.2.0");
    assert_eq!(spec.info.title, "Anthropic Messages API");
    assert_eq!(spec.paths.len(), 5);

    let components = spec.components.as_ref().expect("should have components");
    // Schemas include enums, unions, object types, request/response schemas
    assert!(
        components.schemas.len() >= 25,
        "should have at least 25 schemas, got {}",
        components.schemas.len()
    );

    // Verify security schemes
    let security_schemes = &components.security_schemes;
    assert!(
        security_schemes.contains_key("apiKeyAuth"),
        "should have apiKeyAuth"
    );
    assert!(
        security_schemes.contains_key("bearerAuth"),
        "should have bearerAuth"
    );
}

#[test]
fn parse_anthropic_discriminator() {
    let spec = parse::from_yaml(ANTHROPIC).unwrap();
    let components = spec.components.as_ref().unwrap();

    let content_block = components.schemas.get("ContentBlock").unwrap();
    match content_block {
        oag_core::parse::schema::SchemaOrRef::Schema(s) => {
            assert_eq!(
                s.one_of.len(),
                4,
                "ContentBlock should have 4 oneOf variants"
            );
            let disc = s.discriminator.as_ref().expect("should have discriminator");
            assert_eq!(disc.property_name, "type");
            assert_eq!(disc.mapping.len(), 4);
        }
        _ => panic!("expected inline schema for ContentBlock"),
    }
}

#[test]
fn parse_petstore_polymorphic() {
    let spec = parse::from_yaml(PETSTORE_POLY).expect("should parse petstore-polymorphic.yaml");
    assert_eq!(spec.openapi, "3.2.0");
    assert_eq!(spec.info.title, "Petstore (Polymorphic)");
    assert_eq!(spec.paths.len(), 2);

    let components = spec.components.as_ref().expect("should have components");
    // Pet, Cat, Dog, ErrorModel, ExtendedErrorModel
    assert_eq!(
        components.schemas.len(),
        5,
        "should have 5 schemas, got {}",
        components.schemas.len()
    );

    // Pet should be oneOf with discriminator
    let pet = components.schemas.get("Pet").unwrap();
    match pet {
        oag_core::parse::schema::SchemaOrRef::Schema(s) => {
            assert_eq!(s.one_of.len(), 2, "Pet should have 2 oneOf variants");
            let disc = s.discriminator.as_ref().expect("should have discriminator");
            assert_eq!(disc.property_name, "petType");
            assert_eq!(disc.mapping.len(), 2);
        }
        _ => panic!("expected inline schema for Pet"),
    }

    // ExtendedErrorModel should use allOf
    let ext_err = components.schemas.get("ExtendedErrorModel").unwrap();
    match ext_err {
        oag_core::parse::schema::SchemaOrRef::Schema(s) => {
            assert_eq!(
                s.all_of.len(),
                2,
                "ExtendedErrorModel should have 2 allOf entries"
            );
        }
        _ => panic!("expected inline schema for ExtendedErrorModel"),
    }

    // Security schemes
    let security_schemes = &components.security_schemes;
    assert!(
        security_schemes.contains_key("petstore_auth"),
        "should have petstore_auth"
    );
    assert!(
        security_schemes.contains_key("api_key"),
        "should have api_key"
    );
}

#[test]
fn parse_anthropic_sse_item_schema() {
    let spec = parse::from_yaml(ANTHROPIC).unwrap();
    let path = spec.paths.get("/v1/messages").unwrap();
    let post = path.post.as_ref().unwrap();
    let resp = post.responses.get("200").unwrap();
    match resp {
        oag_core::parse::response::ResponseOrRef::Response(r) => {
            let sse = r.content.get("text/event-stream").unwrap();
            let item_schema = sse.item_schema.as_ref().unwrap();
            match item_schema {
                oag_core::parse::schema::SchemaOrRef::Schema(s) => {
                    assert_eq!(s.one_of.len(), 8, "itemSchema should have 8 oneOf variants");
                    let disc = s
                        .discriminator
                        .as_ref()
                        .expect("SSE itemSchema should have discriminator");
                    assert_eq!(disc.property_name, "type");
                    assert_eq!(disc.mapping.len(), 8);
                }
                _ => panic!("expected inline itemSchema"),
            }
        }
        _ => panic!("expected inline response"),
    }
}
