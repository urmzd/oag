use oag_core::ir::{IrReturnType, IrSchema};
use oag_core::parse;
use oag_core::transform;

const SSE_CHAT: &str = include_str!("fixtures/sse-chat.yaml");
const PETSTORE: &str = include_str!("fixtures/petstore-3.2.yaml");
const MIXED: &str = include_str!("fixtures/mixed-endpoints.yaml");

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
