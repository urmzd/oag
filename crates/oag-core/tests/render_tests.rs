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

/// Render a pack from the working tree and return the concatenated source files.
fn render(pack_rel: &str) -> Vec<oag_core::GeneratedFile> {
    let spec = parse::from_yaml(LITERAL_DEFAULT).expect("fixture should parse");
    let ir = transform::transform(&spec).expect("fixture should transform");
    let pack_dir = format!("{}/../../packs/{pack_rel}", env!("CARGO_MANIFEST_DIR"));
    let pack = TemplatePack::from_dir(std::path::Path::new(&pack_dir))
        .unwrap_or_else(|e| panic!("loading pack {pack_rel}: {e}"));
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
