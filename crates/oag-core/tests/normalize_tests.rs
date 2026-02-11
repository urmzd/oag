use oag_core::transform::name_normalizer::normalize_name;

#[test]
fn test_camel_case_input() {
    let n = normalize_name("createChatCompletion");
    assert_eq!(n.pascal_case, "CreateChatCompletion");
    assert_eq!(n.camel_case, "createChatCompletion");
    assert_eq!(n.snake_case, "create_chat_completion");
    assert_eq!(n.screaming_snake, "CREATE_CHAT_COMPLETION");
}

#[test]
fn test_pascal_case_input() {
    let n = normalize_name("ChatMessage");
    assert_eq!(n.pascal_case, "ChatMessage");
    assert_eq!(n.camel_case, "chatMessage");
    assert_eq!(n.snake_case, "chat_message");
}

#[test]
fn test_snake_case_input() {
    let n = normalize_name("chat_message");
    assert_eq!(n.pascal_case, "ChatMessage");
    assert_eq!(n.camel_case, "chatMessage");
}

#[test]
fn test_kebab_case_input() {
    let n = normalize_name("pet-store-api");
    assert_eq!(n.pascal_case, "PetStoreApi");
    assert_eq!(n.camel_case, "petStoreApi");
}

#[test]
fn test_path_like_input() {
    let n = normalize_name("/pets/{petId}");
    assert_eq!(n.pascal_case, "PetsPetId");
}

#[test]
fn test_single_word() {
    let n = normalize_name("pets");
    assert_eq!(n.pascal_case, "Pets");
    assert_eq!(n.camel_case, "pets");
    assert_eq!(n.snake_case, "pets");
}
