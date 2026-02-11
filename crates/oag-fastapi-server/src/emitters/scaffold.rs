use minijinja::{Environment, context};
use oag_core::GeneratedFile;
use oag_core::config::ToolSetting;
use serde::Deserialize;

/// FastAPI-specific scaffold configuration, parsed from the opaque `serde_json::Value`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FastapiScaffoldConfig {
    pub package_name: Option<String>,
    pub formatter: Option<ToolSetting>,
    pub test_runner: Option<ToolSetting>,
}

/// Emit scaffold files for the FastAPI server (pyproject.toml, optionally ruff.toml).
pub fn emit_scaffold(config: &FastapiScaffoldConfig) -> Vec<GeneratedFile> {
    let mut files = Vec::new();

    let name = config.package_name.as_deref().unwrap_or("generated-server");
    let ruff = ToolSetting::resolve(config.formatter.as_ref(), "ruff") == Some("ruff");
    let pytest = ToolSetting::resolve(config.test_runner.as_ref(), "pytest") == Some("pytest");

    // pyproject.toml
    let mut env = Environment::new();
    env.add_template(
        "pyproject.toml.j2",
        include_str!("../../templates/pyproject.toml.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("pyproject.toml.j2").unwrap();

    files.push(GeneratedFile {
        path: "pyproject.toml".to_string(),
        content: tmpl
            .render(context! { name => name, pytest => pytest, ruff => ruff })
            .expect("render should succeed"),
    });

    // ruff.toml (optional)
    if ruff {
        files.push(GeneratedFile {
            path: "ruff.toml".to_string(),
            content: include_str!("../../templates/ruff.toml.j2").to_string(),
        });
    }

    files
}
