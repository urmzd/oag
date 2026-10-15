//! A copy of a built-in pack left in `.oag/packs/` by an older oag (v0.20
//! downloaded every missing pack there) must not silently override the pack
//! built into this binary: its templates can render code that does not
//! compile against this engine.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SPEC: &str = r#"
openapi: "3.2.0"
info: { title: T, version: "1" }
paths:
  /pets:
    get:
      operationId: listPets
      parameters:
        - { name: limit, in: query, schema: { type: integer } }
      responses:
        "200": { description: ok, content: { application/json: { schema: { type: string } } } }
"#;

const CONFIG: &str = "input: spec.yaml\ngenerators:\n  node-client:\n    output: out\n";

fn oag(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oag"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run oag")
}

/// Copy the working-tree node-client pack into `.oag/packs/` without an
/// install record, as oag v0.20 left it.
fn plant_stale_pack(dir: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/node-client");
    let dst = dir.join(".oag/packs/node-client/templates");
    fs::create_dir_all(&dst).unwrap();
    fs::copy(
        src.join("oag.pack.toml"),
        dst.parent().unwrap().join("oag.pack.toml"),
    )
    .unwrap();
    for entry in fs::read_dir(src.join("templates")).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, dst.join(path.file_name().unwrap())).unwrap();
    }
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("spec.yaml"), SPEC).unwrap();
    fs::write(dir.path().join("oag.yaml"), CONFIG).unwrap();
    dir
}

#[test]
fn stale_local_copy_of_a_built_in_pack_is_refused() {
    let dir = project();
    plant_stale_pack(dir.path());

    let out = oag(dir.path(), &["generate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "generate should refuse, stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("has no install record"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("delete"),
        "names the fix, stderr:\n{stderr}"
    );
    assert!(!dir.path().join("out").exists(), "nothing generated");

    // Explicit opt-in keeps the local copy.
    let out = oag(dir.path(), &["generate", "--allow-stale-packs"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn reinstalling_the_pack_records_it_and_unblocks_generation() {
    let dir = project();
    plant_stale_pack(dir.path());

    let out = oag(dir.path(), &["packs", "install", "--id", "node-client"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = oag(dir.path(), &["generate"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn without_a_local_copy_the_built_in_pack_is_used() {
    let dir = project();
    let out = oag(dir.path(), &["generate"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("built into oag"), "{stderr}");
    let client = fs::read_to_string(dir.path().join("out/client.ts")).unwrap();
    assert!(client.contains("params?: ListPetsParams"), "{client}");
}
