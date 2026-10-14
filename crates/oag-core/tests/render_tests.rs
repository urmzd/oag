//! Render-level golden tests.
//!
//! These load the real working-tree packs (node-client, fastapi-server) and run
//! the engine end-to-end on the `literal-default` fixture, asserting that a
//! literal field with a `default` is rendered as a present (never optional)
//! field carrying its literal value:
//!   - TypeScript: `type: "message";`            (required literal, no `?`)
//!   - Pydantic:   `type: Literal["message"] = "message"`
//!
//! This complements the IR-level test in `ir_tests.rs` by exercising the actual
//! pack templates (the additive `is_literal` / `default_value` context keys).

use oag_core::config::{GeneratorConfig, OutputLayout};
use oag_core::engine::generate;
use oag_core::engine::pack::TemplatePack;
use oag_core::{parse, transform};

const LITERAL_DEFAULT: &str = include_str!("fixtures/literal-default.yaml");
const EVENT_STREAM: &str = include_str!("fixtures/event-stream.yaml");

fn config_for(output: &str) -> GeneratorConfig {
    GeneratorConfig {
        output: output.to_string(),
        layout: OutputLayout::Modular,
        split_by: None,
        base_url: None,
        source_dir: ".".to_string(),
        scaffold: None,
        no_jsdoc: None,
    }
}

/// Render a pack from the working tree against the `literal-default` fixture.
fn render(pack_rel: &str) -> Vec<oag_core::GeneratedFile> {
    render_spec(pack_rel, LITERAL_DEFAULT)
}

/// Load a working-tree pack, resolving `extends` the way the CLI does.
fn load_pack(pack_rel: &str) -> TemplatePack {
    let pack_dir = format!("{}/../../packs/{pack_rel}", env!("CARGO_MANIFEST_DIR"));
    let pack = TemplatePack::from_dir(std::path::Path::new(&pack_dir))
        .unwrap_or_else(|e| panic!("loading pack {pack_rel}: {e}"));
    match pack.manifest.pack.extends.clone() {
        Some(base_id) => {
            let mut base = load_pack(&base_id);
            base.merge_from(&pack);
            base
        }
        None => pack,
    }
}

/// Render a working-tree pack against an arbitrary spec and return its source files.
fn render_spec(pack_rel: &str, spec_yaml: &str) -> Vec<oag_core::GeneratedFile> {
    let spec = parse::from_yaml(spec_yaml).expect("fixture should parse");
    let ir = transform::transform(&spec).expect("fixture should transform");
    let pack = load_pack(pack_rel);
    let out = generate(&ir, &config_for("out"), &pack).expect("generation should succeed");
    out.source_files
}

/// Return the content of the rendered file whose path ends with `suffix`.
fn file_ending_with(files: &[oag_core::GeneratedFile], suffix: &str) -> String {
    files
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no rendered file ending in {suffix}"))
        .content
        .clone()
}

#[test]
fn node_client_renders_literal_default_as_required_literal() {
    let files = render("node-client");
    let types = file_ending_with(&files, "types.ts");

    // Literal-with-default fields are present (no `?`) carrying the literal.
    assert!(
        types.contains("type: \"message\";"),
        "expected required `type: \"message\";`, got:\n{types}"
    );
    assert!(
        types.contains("objectType: \"list\";"),
        "expected required `objectType: \"list\";`, got:\n{types}"
    );
    assert!(
        types.contains("version: 1;"),
        "expected required `version: 1;`, got:\n{types}"
    );
    // A literal field must never be optional.
    assert!(
        !types.contains("type?:"),
        "literal `type` must not be optional, got:\n{types}"
    );

    // Const literal that is also in `required` stays required.
    assert!(
        types.contains("type: \"text\";"),
        "expected `type: \"text\";` for TextBlock, got:\n{types}"
    );

    // Non-literal optional fields keep the `?`.
    assert!(
        types.contains("temperature?:"),
        "non-literal `temperature` should stay optional, got:\n{types}"
    );
    assert!(
        types.contains("role?:"),
        "multi-value enum `role` should stay optional, got:\n{types}"
    );
    // Plain required field is present without `?`.
    assert!(
        types.contains("content: string;"),
        "required `content` should be present, got:\n{types}"
    );
}

#[test]
fn fastapi_server_renders_literal_default_as_defaulted_literal() {
    let files = render("fastapi-server");
    let models = file_ending_with(&files, "models.py");

    // Literal-with-default → typed as the literal, defaulted to its value.
    assert!(
        models.contains("type: Literal[\"message\"] = \"message\""),
        "expected `type: Literal[\"message\"] = \"message\"`, got:\n{models}"
    );
    assert!(
        models.contains(
            "object_type: Literal[\"list\"] = Field(default=\"list\", alias=\"objectType\")"
        ),
        "expected aliased literal default for object_type, got:\n{models}"
    );
    assert!(
        models.contains("version: Literal[1] = 1"),
        "expected `version: Literal[1] = 1`, got:\n{models}"
    );
    // Const literal in `required` is still a defaulted literal.
    assert!(
        models.contains("type: Literal[\"text\"] = \"text\""),
        "expected `type: Literal[\"text\"] = \"text\"` for TextBlock, got:\n{models}"
    );

    // The `Literal` symbol must be imported.
    assert!(
        models.contains("from typing import Any, Literal"),
        "expected `Literal` import, got:\n{models}"
    );

    // A literal field must never be `| None` and must never assign a type as a value.
    assert!(
        !models.contains("Literal[\"message\"] | None"),
        "literal field must not be optional, got:\n{models}"
    );
    assert!(
        !models.contains("= Literal["),
        "must not assign a type as a value, got:\n{models}"
    );

    // Non-literal optional fields stay `... | None = ...`.
    assert!(
        models.contains("temperature: float | None = None"),
        "non-literal `temperature` should stay optional, got:\n{models}"
    );
    // Plain required field is present.
    assert!(
        models.contains("content: str"),
        "required `content` should be present, got:\n{models}"
    );
}

// ─── event-stream fixture (issues #27-#36) ───────────────────────────────

/// Extract the body of a generated TypeScript method (from its signature to the
/// next method), so assertions do not match code from a neighbouring method.
fn ts_method(source: &str, signature_prefix: &str) -> String {
    let start = source
        .find(signature_prefix)
        .unwrap_or_else(|| panic!("no method starting with `{signature_prefix}` in:\n{source}"));
    let rest = &source[start..];
    let end = rest[1..]
        .find("\n  async ")
        .map(|i| i + 1)
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

#[test]
fn node_client_serializes_ref_enum_query_param_as_primitive() {
    // #27: a `$ref` to a string enum must serialize like the inline enum
    // (`?state=open`), not be walked with `Object.entries` (`?0=o&1=p...`).
    let files = render_spec("node-client", EVENT_STREAM);
    let client = file_ending_with(&files, "client.ts");
    let method = ts_method(&client, "async listInbox(");
    assert!(
        method.contains(r#"_q.set("state", String("#),
        "ref enum query param should be set as a primitive, got:\n{method}"
    );
    assert!(
        method.contains(r#"_q.set("kind", String("#),
        "ref enum query param should be set as a primitive, got:\n{method}"
    );
    assert!(
        !method.contains("Object.entries"),
        "no query param of listInbox is an object, got:\n{method}"
    );
}

#[test]
fn bodyless_204_with_default_error_renders_as_void() {
    // #28: the success type comes from 2xx only; `default` shapes errors.
    let node = render_spec("node-client", EVENT_STREAM);
    let client = file_ending_with(&node, "client.ts");
    let method = ts_method(&client, "async deleteInboxItem(");
    assert!(
        method.contains("Promise<void>"),
        "deleteInboxItem should resolve to void, got:\n{method}"
    );

    let py = render_spec("fastapi-server", EVENT_STREAM);
    let routes = file_ending_with(&py, "routes.py");
    assert!(
        routes.contains("@router.delete(\"/inbox/{itemId}\", status_code=204)"),
        "fastapi stub should declare 204, got:\n{routes}"
    );
    assert!(
        !routes.contains("-> ErrorBody"),
        "no route returns the error body as its success type, got:\n{routes}"
    );
}

#[test]
fn shared_discriminator_mapping_emits_one_guard_per_schema() {
    // #29: two mapping values for `LifecycleEvent` used to emit two
    // `isLifecycleEvent` functions (TS2393). Expect one guard checking both.
    let files = render_spec("node-client", EVENT_STREAM);
    let guards = file_ending_with(&files, "guards.ts");
    assert_eq!(
        guards.matches("export function isLifecycleEvent(").count(),
        1,
        "expected exactly one isLifecycleEvent guard, got:\n{guards}"
    );
    assert!(
        guards.contains(r#"v.kind === "lifecycle.started" || v.kind === "lifecycle.completed""#),
        "guard should accept every mapped value, got:\n{guards}"
    );
    assert_eq!(
        guards.matches("export function isDeltaEvent(").count(),
        1,
        "got:\n{guards}"
    );
}

#[test]
fn binary_responses_are_read_as_blobs() {
    // #30: octet-stream bodies were read with `response.text()` but typed Blob.
    let files = render_spec("node-client", EVENT_STREAM);
    let client = file_ending_with(&files, "client.ts");
    let method = ts_method(&client, "async downloadFile(");
    assert!(method.contains("Promise<Blob>"), "got:\n{method}");
    assert!(method.contains(r#"parseAs: "blob""#), "got:\n{method}");
    assert!(
        client.contains("await response.blob()"),
        "runtime should read blobs, got:\n{client}"
    );
    // JSON endpoints keep the default JSON parsing.
    let list = ts_method(&client, "async listInbox(");
    assert!(!list.contains("parseAs"), "got:\n{list}");

    let py = render_spec("fastapi-server", EVENT_STREAM);
    let routes = file_ending_with(&py, "routes.py");
    assert!(
        routes.contains(r#"@router.get("/files/{fileId}/content", response_class=Response)"#),
        "fastapi should return raw bytes, got:\n{routes}"
    );
}
