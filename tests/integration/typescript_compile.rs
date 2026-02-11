use std::fs;
use std::process::Command;

use oag_core::config::GeneratorConfig;
use oag_core::{parse, transform, CodeGenerator};
use oag_node_client::NodeClientGenerator;

const SSE_CHAT: &str = include_str!("../../crates/oag-core/tests/fixtures/sse-chat.yaml");

#[test]
#[ignore] // Requires Node.js + TypeScript installed
fn generated_typescript_compiles() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    let config = GeneratorConfig::default();
    let files = NodeClientGenerator.generate(&ir, &config).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Write generated files
    for file in &files {
        fs::write(dir.join(&file.path), &file.content).unwrap();
    }

    // Write tsconfig
    let tsconfig = r#"{
  "compilerOptions": {
    "strict": true,
    "target": "ES2020",
    "module": "ES2020",
    "moduleResolution": "bundler",
    "lib": ["ES2020", "DOM"],
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["*.ts"]
}"#;
    fs::write(dir.join("tsconfig.json"), tsconfig).unwrap();

    // Run tsc
    let output = Command::new("npx")
        .args(["tsc", "--noEmit"])
        .current_dir(dir)
        .output()
        .expect("failed to run tsc");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "TypeScript compilation failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}
