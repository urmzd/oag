use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::Deserialize;

/// Top-level project configuration loaded from `.urmzd.oag.yaml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OagConfig {
    pub input: String,
    pub output: String,
    pub target: TargetKind,
    pub naming: NamingConfig,
    pub output_options: OutputOptions,
    pub client: ClientConfig,
}

impl Default for OagConfig {
    fn default() -> Self {
        Self {
            input: "openapi.yaml".to_string(),
            output: "src/generated".to_string(),
            target: TargetKind::All,
            naming: NamingConfig::default(),
            output_options: OutputOptions::default(),
            client: ClientConfig::default(),
        }
    }
}

/// Which generators to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Typescript,
    React,
    All,
}

/// Naming strategy and aliases.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NamingConfig {
    pub strategy: NamingStrategy,
    /// Map from resolved operation name (operationId or route-derived) to custom alias.
    #[serde(default)]
    pub aliases: IndexMap<String, String>,
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            strategy: NamingStrategy::UseOperationId,
            aliases: IndexMap::new(),
        }
    }
}

/// How operation names are derived.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamingStrategy {
    #[default]
    UseOperationId,
    UseRouteBased,
}

/// Output structure and tooling options.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OutputOptions {
    pub layout: OutputLayout,
    pub index: bool,
    pub biome: bool,
    pub tsdown: bool,
    /// Custom package name for package.json (defaults to slugified spec title).
    pub package_name: Option<String>,
    /// Repository URL for package.json.
    pub repository: Option<String>,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            layout: OutputLayout::Single,
            index: true,
            biome: true,
            tsdown: true,
            package_name: None,
            repository: None,
        }
    }
}

/// How generated files are laid out on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLayout {
    /// All files in one output directory.
    Single,
    /// `typescript/` and `react/` subdirectories.
    Split,
}

/// Client generation options.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ClientConfig {
    pub base_url: Option<String>,
    pub no_jsdoc: bool,
}

/// Default config file name.
pub const CONFIG_FILE_NAME: &str = ".urmzd.oag.yaml";

/// Load config from a YAML file. Returns `None` if the file doesn't exist.
pub fn load_config(path: &Path) -> Result<Option<OagConfig>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config {}: {}", path.display(), e))?;
    let config: OagConfig = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("failed to parse config {}: {}", path.display(), e))?;
    Ok(Some(config))
}

/// Generate the default config file content.
pub fn default_config_content() -> &'static str {
    r#"# oag configuration — https://github.com/urmzd/openapi-generator
input: openapi.yaml
output: src/generated
target: all           # typescript | react | all

naming:
  strategy: use_operation_id  # use_operation_id | use_route_based
  aliases: {}
    # createChatCompletion: chat     # operationId → custom name
    # listModels: models

output_options:
  layout: single        # single | split (split = typescript/ + react/ subdirs)
  index: true           # generate index.ts barrel exports
  biome: true           # generate biome.json and format output
  tsdown: true          # generate tsdown.config.ts
  # package_name: my-api-client   # custom npm package name (defaults to slugified spec title)
  # repository: https://github.com/you/your-repo

client:
  # base_url: https://api.example.com
  no_jsdoc: false
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OagConfig::default();
        assert_eq!(config.input, "openapi.yaml");
        assert_eq!(config.output, "src/generated");
        assert_eq!(config.target, TargetKind::All);
        assert_eq!(config.naming.strategy, NamingStrategy::UseOperationId);
        assert!(config.naming.aliases.is_empty());
        assert_eq!(config.output_options.layout, OutputLayout::Single);
        assert!(config.output_options.biome);
        assert!(config.output_options.tsdown);
    }

    #[test]
    fn test_parse_config_yaml() {
        let yaml = r#"
input: spec.yaml
output: out
target: typescript
naming:
  strategy: use_route_based
  aliases:
    createChatCompletion: chat
    listModels: models
output_options:
  layout: split
  biome: false
  tsdown: false
client:
  base_url: https://api.example.com
  no_jsdoc: true
"#;
        let config: OagConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.input, "spec.yaml");
        assert_eq!(config.output, "out");
        assert_eq!(config.target, TargetKind::Typescript);
        assert_eq!(config.naming.strategy, NamingStrategy::UseRouteBased);
        assert_eq!(config.naming.aliases.len(), 2);
        assert_eq!(config.naming.aliases["createChatCompletion"], "chat");
        assert_eq!(config.output_options.layout, OutputLayout::Split);
        assert!(!config.output_options.biome);
        assert_eq!(
            config.client.base_url,
            Some("https://api.example.com".to_string())
        );
        assert!(config.client.no_jsdoc);
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = "input: api.yaml\n";
        let config: OagConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.input, "api.yaml");
        // Defaults applied
        assert_eq!(config.output, "src/generated");
        assert_eq!(config.target, TargetKind::All);
    }
}
