use oag_core::ir::{IrParameterLocation, IrReturnType, IrSchema, IrType};
use oag_core::parse;
use oag_core::transform;

const SSE_CHAT: &str = include_str!("fixtures/sse-chat.yaml");
const PETSTORE: &str = include_str!("fixtures/petstore-3.2.yaml");
const MIXED: &str = include_str!("fixtures/mixed-endpoints.yaml");
const ANTHROPIC: &str = include_str!("fixtures/anthropic-messages.yaml");

#[test]
fn transform_sse_chat() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    assert_eq!(ir.info.title, "AI Chat API");
    assert!(!ir.schemas.is_empty());
    assert!(!ir.operations.is_empty());

    // Check that we have SSE operations
    let stream_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "createChatCompletionStream")
        .expect("should have createChatCompletionStream");

    match &stream_op.return_type {
        IrReturnType::Sse(sse) => {
            assert!(sse.event_type_name.is_some(), "should have event type name");
            assert_eq!(sse.variants.len(), 2, "should have 2 SSE event variants");
            assert!(
                !sse.also_has_json,
                "stream-only endpoint should not have JSON"
            );
        }
        _ => panic!("expected SSE return type"),
    }

    // Check dual endpoint
    let dual_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "createChatCompletion")
        .expect("should have createChatCompletion");

    match &dual_op.return_type {
        IrReturnType::Sse(sse) => {
            assert!(sse.also_has_json, "dual endpoint should have JSON");
            assert!(sse.json_response.is_some());
        }
        _ => panic!("expected SSE return type for dual endpoint"),
    }
}

#[test]
fn transform_petstore() {
    let spec = parse::from_yaml(PETSTORE).unwrap();
    let ir = transform::transform(&spec).unwrap();

    assert_eq!(ir.info.title, "Petstore");

    // Check schema types
    let pet = ir.schemas.iter().find(|s| s.name().pascal_case == "Pet");
    assert!(pet.is_some(), "should have Pet schema");
    match pet.unwrap() {
        IrSchema::Object(obj) => {
            assert!(obj.fields.len() >= 3);
        }
        _ => panic!("Pet should be an object schema"),
    }

    let status = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "PetStatus");
    assert!(status.is_some(), "should have PetStatus schema");
    match status.unwrap() {
        IrSchema::Enum(e) => {
            assert_eq!(e.variants.len(), 3);
        }
        _ => panic!("PetStatus should be an enum"),
    }

    // Check operations
    assert!(!ir.operations.is_empty());
    let list_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "listPets")
        .expect("should have listPets");
    assert_eq!(list_op.parameters.len(), 2); // limit, status

    let delete_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "deletePet")
        .expect("should have deletePet");
    match &delete_op.return_type {
        IrReturnType::Void => {}
        _ => panic!("deletePet should return void"),
    }
}

#[test]
fn transform_mixed() {
    let spec = parse::from_yaml(MIXED).unwrap();
    let ir = transform::transform(&spec).unwrap();

    // Check SSE endpoint without itemSchema (fallback to schema)
    let stream_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "streamEvents")
        .expect("should have streamEvents");

    match &stream_op.return_type {
        IrReturnType::Sse(sse) => {
            assert!(!sse.also_has_json);
            // Event type should be resolved from schema
            assert!(
                sse.event_type_name.is_none(),
                "no itemSchema oneOf, so no event type name"
            );
        }
        _ => panic!("expected SSE return type"),
    }
}

#[test]
fn transform_modules_grouping() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    assert!(
        ir.modules.len() >= 2,
        "should have at least chat and models modules"
    );

    let chat_module = ir
        .modules
        .iter()
        .find(|m| m.name.original == "chat")
        .expect("should have chat module");
    assert!(!chat_module.operations.is_empty());

    let models_module = ir
        .modules
        .iter()
        .find(|m| m.name.original == "models")
        .expect("should have models module");
    assert!(!models_module.operations.is_empty());
}

#[test]
fn transform_request_body() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    let create_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "createChatCompletion")
        .expect("should have createChatCompletion");

    let body = create_op
        .request_body
        .as_ref()
        .expect("should have request body");
    assert!(body.required);
    assert_eq!(body.content_type, "application/json");
}

#[test]
fn transform_void_response() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    let feedback_op = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "submitFeedback")
        .expect("should have submitFeedback");

    match &feedback_op.return_type {
        IrReturnType::Void => {}
        _ => panic!("submitFeedback should return void"),
    }
}

#[test]
fn transform_anthropic_messages() {
    let spec = parse::from_yaml(ANTHROPIC).unwrap();
    let ir = transform::transform(&spec).unwrap();

    assert_eq!(ir.info.title, "Anthropic Messages API");

    // --- ContentBlock: discriminated union with 4 variants ---
    let content_block = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "ContentBlock")
        .expect("should have ContentBlock schema");
    match content_block {
        IrSchema::Union(u) => {
            assert_eq!(u.variants.len(), 4, "ContentBlock should have 4 variants");
            let disc = u.discriminator.as_ref().expect("should have discriminator");
            assert_eq!(disc.property_name, "type");
            assert_eq!(disc.mapping.len(), 4);
        }
        _ => panic!("ContentBlock should be a Union"),
    }

    // --- StreamDelta: discriminated union with 2 variants ---
    let stream_delta = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "StreamDelta")
        .expect("should have StreamDelta schema");
    match stream_delta {
        IrSchema::Union(u) => {
            assert_eq!(u.variants.len(), 2, "StreamDelta should have 2 variants");
            let disc = u.discriminator.as_ref().expect("should have discriminator");
            assert_eq!(disc.property_name, "type");
        }
        _ => panic!("StreamDelta should be a Union"),
    }

    // --- ToolResultContent: anyOf union with NO discriminator ---
    let tool_result_content = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "ToolResultContent")
        .expect("should have ToolResultContent schema");
    match tool_result_content {
        IrSchema::Union(u) => {
            assert_eq!(
                u.variants.len(),
                2,
                "ToolResultContent should have 2 variants"
            );
            assert!(
                u.discriminator.is_none(),
                "anyOf should have no discriminator"
            );
        }
        _ => panic!("ToolResultContent should be a Union"),
    }

    // --- MessageStartEvent: allOf merged into Object ---
    let msg_start = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "MessageStartEvent")
        .expect("should have MessageStartEvent schema");
    match msg_start {
        IrSchema::Object(obj) => {
            assert!(
                obj.fields.iter().any(|f| f.original_name == "type"),
                "should have type field"
            );
            assert!(
                obj.fields.iter().any(|f| f.original_name == "message"),
                "should have message field"
            );
        }
        _ => panic!("MessageStartEvent should be an Object (merged allOf)"),
    }

    // --- TextBlock.type is StringLiteral("text") ---
    let text_block = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "TextBlock")
        .expect("should have TextBlock schema");
    match text_block {
        IrSchema::Object(obj) => {
            let type_field = obj
                .fields
                .iter()
                .find(|f| f.original_name == "type")
                .expect("TextBlock should have type field");
            assert_eq!(
                type_field.field_type,
                IrType::StringLiteral("text".to_string()),
                "TextBlock.type should be StringLiteral(\"text\")"
            );
        }
        _ => panic!("TextBlock should be an Object"),
    }

    // --- MessageResponse.id has read_only: true ---
    let msg_resp = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "MessageResponse")
        .expect("should have MessageResponse schema");
    match msg_resp {
        IrSchema::Object(obj) => {
            let id_field = obj
                .fields
                .iter()
                .find(|f| f.original_name == "id")
                .expect("MessageResponse should have id field");
            assert!(id_field.read_only, "MessageResponse.id should be readOnly");
        }
        _ => panic!("MessageResponse should be an Object"),
    }

    // --- createMessage has anthropic-version header parameter ---
    let create_msg = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "createMessage")
        .expect("should have createMessage operation");
    let header_param = create_msg
        .parameters
        .iter()
        .find(|p| p.original_name == "anthropic-version")
        .expect("should have anthropic-version parameter");
    assert_eq!(header_param.location, IrParameterLocation::Header);

    // --- createMessage SSE return has 8 variants, also_has_json: true ---
    match &create_msg.return_type {
        IrReturnType::Sse(sse) => {
            assert_eq!(sse.variants.len(), 8, "should have 8 SSE event variants");
            assert!(sse.also_has_json, "dual endpoint should have JSON");
            assert!(sse.json_response.is_some());
        }
        _ => panic!("createMessage should have SSE return type"),
    }

    // --- cancelBatch returns Void ---
    let cancel_batch = ir
        .operations
        .iter()
        .find(|op| op.name.camel_case == "cancelBatch")
        .expect("should have cancelBatch operation");
    match &cancel_batch.return_type {
        IrReturnType::Void => {}
        _ => panic!("cancelBatch should return Void"),
    }

    // --- StopReason is Enum with 4 variants ---
    let stop_reason = ir
        .schemas
        .iter()
        .find(|s| s.name().pascal_case == "StopReason")
        .expect("should have StopReason schema");
    match stop_reason {
        IrSchema::Enum(e) => {
            assert_eq!(e.variants.len(), 4, "StopReason should have 4 variants");
        }
        _ => panic!("StopReason should be an Enum"),
    }

    // --- At least 3 modules ---
    assert!(
        ir.modules.len() >= 3,
        "should have at least 3 modules (messages, models, and tokens or batches), got {}",
        ir.modules.len()
    );
}
