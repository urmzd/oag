use std::fs;
use std::process::Command;

use oag_core::config::GeneratorConfig;
use oag_core::{parse, transform, CodeGenerator};
use oag_react_swr_client::ReactSwrClientGenerator;

const SSE_CHAT: &str = include_str!("../../crates/oag-core/tests/fixtures/sse-chat.yaml");

#[test]
#[ignore] // Requires Node.js + TypeScript + React installed
fn generated_react_compiles() {
    let spec = parse::from_yaml(SSE_CHAT).unwrap();
    let ir = transform::transform(&spec).unwrap();

    let config = GeneratorConfig::default();
    let files = ReactSwrClientGenerator.generate(&ir, &config).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Write generated files
    for file in &files {
        fs::write(dir.join(&file.path), &file.content).unwrap();
    }

    // Initialize package.json and install deps
    let package_json = r#"{
  "private": true,
  "devDependencies": {
    "typescript": "^5",
    "swr": "^2",
    "react": "^18",
    "@types/react": "^18"
  }
}"#;
    fs::write(dir.join("package.json"), package_json).unwrap();

    // Write tsconfig
    let tsconfig = r#"{
  "compilerOptions": {
    "strict": true,
    "target": "ES2020",
    "module": "ES2020",
    "moduleResolution": "bundler",
    "lib": ["ES2020", "DOM"],
    "jsx": "react-jsx",
    "noEmit": true,
    "skipLibCheck": true
  },
  "include": ["*.ts", "*.tsx"]
}"#;
    fs::write(dir.join("tsconfig.json"), tsconfig).unwrap();

    // Install dependencies
    let install = Command::new("npm")
        .args(["install", "--no-audit", "--no-fund"])
        .current_dir(dir)
        .output()
        .expect("failed to run npm install");

    if !install.status.success() {
        let stderr = String::from_utf8_lossy(&install.stderr);
        panic!("npm install failed: {}", stderr);
    }

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
