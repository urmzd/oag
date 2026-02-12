use std::fs;
use std::process::Command;

use oag_core::config::GeneratorConfig;
use oag_core::{CodeGenerator, parse, transform};
use oag_react_swr_client::ReactSwrClientGenerator;

const PETSTORE: &str = include_str!("../../oag-core/tests/fixtures/petstore-3.2.yaml");
const SSE_CHAT: &str = include_str!("../../oag-core/tests/fixtures/sse-chat.yaml");
const ANTHROPIC: &str = include_str!("../../oag-core/tests/fixtures/anthropic-messages.yaml");

fn scaffold_config() -> GeneratorConfig {
    GeneratorConfig {
        scaffold: Some(serde_json::json!({
            "package_name": "@test/react-client",
            "formatter": "biome",
            "bundler": false,
            "test_runner": false,
        })),
        ..GeneratorConfig::default()
    }
}

fn compile_react(yaml: &str) {
    let spec = parse::from_yaml(yaml).unwrap();
    let ir = transform::transform(&spec).unwrap();

    let config = scaffold_config();
    let files = ReactSwrClientGenerator.generate(&ir, &config).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    for file in &files {
        let dest = dir.join(&file.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&dest, &file.content).unwrap();
    }

    let install = Command::new("npm")
        .args(["install", "--no-audit", "--no-fund"])
        .current_dir(dir)
        .output()
        .expect("failed to run npm install");
    if !install.status.success() {
        panic!(
            "npm install failed:\n{}",
            String::from_utf8_lossy(&install.stderr)
        );
    }

    let tsc = Command::new("npx")
        .args(["tsc", "--noEmit"])
        .current_dir(dir)
        .output()
        .expect("failed to run tsc");
    if !tsc.status.success() {
        panic!(
            "tsc failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&tsc.stdout),
            String::from_utf8_lossy(&tsc.stderr),
        );
    }

    // Apply safe auto-fixes (formatting, import ordering) then verify.
    let biome_fix = Command::new("npx")
        .args(["@biomejs/biome", "check", "--write", "."])
        .current_dir(dir)
        .output()
        .expect("failed to run biome check --write");
    if !biome_fix.status.success() {
        panic!(
            "biome check --write failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&biome_fix.stdout),
            String::from_utf8_lossy(&biome_fix.stderr),
        );
    }

    let biome = Command::new("npx")
        .args(["@biomejs/biome", "check", "."])
        .current_dir(dir)
        .output()
        .expect("failed to run biome check");
    if !biome.status.success() {
        panic!(
            "biome check failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&biome.stdout),
            String::from_utf8_lossy(&biome.stderr),
        );
    }
}

#[test]
#[ignore] // Requires Node.js
fn generated_react_petstore_compiles() {
    compile_react(PETSTORE);
}

#[test]
#[ignore] // Requires Node.js — also has known duplicate-identifier issue in SSE dual endpoints
fn generated_react_sse_chat_compiles() {
    compile_react(SSE_CHAT);
}

#[test]
#[ignore] // Requires Node.js
fn generated_react_anthropic_compiles() {
    compile_react(ANTHROPIC);
}
