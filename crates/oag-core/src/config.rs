use std::fmt;
use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::de;
use serde::{Deserialize, Deserializer};

/// A tool setting that can be a named tool or explicitly disabled.
///
/// In YAML: `"biome"` → `Named("biome")`, `false` → `Disabled`.
/// `true` or absent → treated as "use default" (represented as `None` at the Option level).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSetting {
    Named(String),
    Disabled,
}

impl ToolSetting {
    /// Resolve with a default: `None` → `Some(default)`, `Named(s)` → `Some(s)`, `Disabled` → `None`.
    pub fn resolve<'a>(setting: Option<&'a Self>, default: &'a str) -> Option<&'a str> {
        match setting {
            None => Some(default),
            Some(ToolSetting::Named(s)) => Some(s.as_str()),
            Some(ToolSetting::Disabled) => None,
        }
    }
}

impl<'de> Deserialize<'de> for ToolSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;
        match value {
            serde_json::Value::String(s) => Ok(ToolSetting::Named(s)),
            serde_json::Value::Bool(false) => Ok(ToolSetting::Disabled),
            serde_json::Value::Bool(true) => {
                // true means "use default" — caller should treat as absent
                Err(de::Error::custom(
                    "use `false` to disable or a string to name the tool; `true` is treated as default (omit the field)",
                ))
            }
            _ => Err(de::Error::custom(
                "expected a tool name string or `false` to disable",
            )),
        }
    }
}

/// Top-level project configuration loaded from `.urmzd.oag.yaml`.
#[derive(Debug, Clone)]
pub struct OagConfig {
    pub input: String,
    pub naming: NamingConfig,
    pub generators: IndexMap<GeneratorId, GeneratorConfig>,
}

impl Default for OagConfig {
    fn default() -> Self {
        Self {
            input: "openapi.yaml".to_string(),
            naming: NamingConfig::default(),
            generators: IndexMap::new(),
        }
    }
}

/// A generator plugin identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GeneratorId {
    NodeClient,
    ReactSwrClient,
    FastapiServer,
}

impl GeneratorId {
    pub fn as_str(&self) -> &'static str {
        match self {
            GeneratorId::NodeClient => "node-client",
            GeneratorId::ReactSwrClient => "react-swr-client",
            GeneratorId::FastapiServer => "fastapi-server",
        }
    }
}

impl fmt::Display for GeneratorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'de> Deserialize<'de> for GeneratorId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "node-client" => Ok(GeneratorId::NodeClient),
            "react-swr-client" => Ok(GeneratorId::ReactSwrClient),
            "fastapi-server" => Ok(GeneratorId::FastapiServer),
            other => Err(de::Error::unknown_variant(
                other,
                &["node-client", "react-swr-client", "fastapi-server"],
            )),
        }
    }
}

/// Configuration for a single generator.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneratorConfig {
    pub output: String,
    pub layout: OutputLayout,
    pub split_by: Option<SplitBy>,
    pub base_url: Option<String>,
    pub no_jsdoc: Option<bool>,
    /// Subdirectory for generated source files. Default `"src"`.
    /// Empty string `""` places files at the output root.
    pub source_dir: String,
    /// Opaque scaffold config — each generator defines and parses its own struct.
    pub scaffold: Option<serde_json::Value>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            output: "src/generated".to_string(),
            layout: OutputLayout::Modular,
            split_by: None,
            base_url: None,
            no_jsdoc: None,
            source_dir: "src".to_string(),
            scaffold: None,
        }
    }
}

/// How generated files are laid out on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLayout {
    /// All files concatenated into one output file + scaffold.
    Bundled,
    /// Current behavior: types.ts, client.ts, sse.ts, index.ts.
    Modular,
    /// Per-group files with shared base.
    Split,
}

/// How to split operations into groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitBy {
    /// Group by operation (one file per operation).
    Operation,
    /// Group by tag (reuse IrModule).
    Tag,
    /// Group by path prefix.
    Route,
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

// --- Backward-compatible deserialization ---
// Old format had: input, output, target, naming, output_options, client
// New format has: input, naming, generators (map of GeneratorId -> GeneratorConfig)
// We support both.

/// Internal legacy config format for backward compat parsing.
#[derive(Deserialize)]
struct LegacyConfig {
    #[serde(default = "default_input")]
    input: String,
    #[serde(default = "default_output")]
    output: String,
    #[serde(default)]
    target: LegacyTargetKind,
    #[serde(default)]
    naming: NamingConfig,
    #[serde(default)]
    output_options: LegacyOutputOptions,
    #[serde(default)]
    client: LegacyClientConfig,
}

fn default_input() -> String {
    "openapi.yaml".to_string()
}
fn default_output() -> String {
    "src/generated".to_string()
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LegacyTargetKind {
    Typescript,
    React,
    #[default]
    All,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct LegacyOutputOptions {
    layout: LegacyOutputLayout,
    index: bool,
    biome: bool,
    tsdown: bool,
    package_name: Option<String>,
    repository: Option<String>,
}

impl Default for LegacyOutputOptions {
    fn default() -> Self {
        Self {
            layout: LegacyOutputLayout::Single,
            index: true,
            biome: true,
            tsdown: true,
            package_name: None,
            repository: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LegacyOutputLayout {
    #[default]
    Single,
    Split,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct LegacyClientConfig {
    base_url: Option<String>,
    no_jsdoc: bool,
}

/// Internal new-format config for forward parsing.
#[derive(Deserialize)]
struct NewConfig {
    #[serde(default = "default_input")]
    input: String,
    #[serde(default)]
    naming: NamingConfig,
    generators: IndexMap<GeneratorId, GeneratorConfig>,
}

impl<'de> Deserialize<'de> for OagConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // We deserialize into a generic map first to detect the format.
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;

        // Check if the config has a "generators" key — that's the new format.
        if value.get("generators").is_some() {
            let new_cfg: NewConfig = serde_json::from_value(value).map_err(de::Error::custom)?;
            Ok(OagConfig {
                input: new_cfg.input,
                naming: new_cfg.naming,
                generators: new_cfg.generators,
            })
        } else {
            // Legacy format
            let legacy: LegacyConfig = serde_json::from_value(value).map_err(de::Error::custom)?;
            Ok(convert_legacy(legacy))
        }
    }
}

fn convert_legacy(legacy: LegacyConfig) -> OagConfig {
    let scaffold = Some(serde_json::json!({
        "package_name": legacy.output_options.package_name,
        "repository": legacy.output_options.repository,
        "index": legacy.output_options.index,
        "formatter": if legacy.output_options.biome { serde_json::Value::String("biome".into()) } else { serde_json::Value::Bool(false) },
        "bundler": if legacy.output_options.tsdown { serde_json::Value::String("tsdown".into()) } else { serde_json::Value::Bool(false) },
        "test_runner": serde_json::Value::String("vitest".into()),
    }));

    let base_gen_config = |output: String| GeneratorConfig {
        output,
        layout: OutputLayout::Modular,
        split_by: None,
        base_url: legacy.client.base_url.clone(),
        no_jsdoc: Some(legacy.client.no_jsdoc),
        source_dir: "src".to_string(),
        scaffold: scaffold.clone(),
    };

    let mut generators = IndexMap::new();

    match (&legacy.target, &legacy.output_options.layout) {
        (LegacyTargetKind::Typescript, _) => {
            generators.insert(
                GeneratorId::NodeClient,
                base_gen_config(legacy.output.clone()),
            );
        }
        (LegacyTargetKind::React, _) => {
            generators.insert(
                GeneratorId::ReactSwrClient,
                base_gen_config(legacy.output.clone()),
            );
        }
        (LegacyTargetKind::All, LegacyOutputLayout::Single) => {
            // In single layout with target=all, the old behavior was to put
            // everything together using the React generator (which includes TS files).
            // Map this to a single react-swr-client generator.
            generators.insert(
                GeneratorId::ReactSwrClient,
                base_gen_config(legacy.output.clone()),
            );
        }
        (LegacyTargetKind::All, LegacyOutputLayout::Split) => {
            let ts_output = format!("{}/typescript", legacy.output);
            let react_output = format!("{}/react", legacy.output);
            generators.insert(GeneratorId::NodeClient, base_gen_config(ts_output));
            generators.insert(GeneratorId::ReactSwrClient, base_gen_config(react_output));
        }
    }

    OagConfig {
        input: legacy.input,
        naming: legacy.naming,
        generators,
    }
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

    // Parse YAML to serde_json::Value first, then use our custom Deserialize impl
    let yaml_value: serde_json::Value = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("failed to parse config {}: {}", path.display(), e))?;
    let config: OagConfig = serde_json::from_value(yaml_value)
        .map_err(|e| format!("failed to parse config {}: {}", path.display(), e))?;
    Ok(Some(config))
}

/// Generate the default config file content (new format).
pub fn default_config_content() -> &'static str {
    include_str!("../default-config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OagConfig::default();
        assert_eq!(config.input, "openapi.yaml");
        assert_eq!(config.naming.strategy, NamingStrategy::UseOperationId);
        assert!(config.naming.aliases.is_empty());
        assert!(config.generators.is_empty());
    }

    #[test]
    fn test_parse_new_format() {
        let yaml = r#"
input: spec.yaml

naming:
  strategy: use_route_based
  aliases:
    createChatCompletion: chat

generators:
  node-client:
    output: out/node
    layout: modular
    base_url: https://api.example.com
    scaffold:
      package_name: "@myorg/client"
      formatter: biome
      bundler: tsdown
  react-swr-client:
    output: out/react
    layout: split
    split_by: tag
"#;
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.input, "spec.yaml");
        assert_eq!(config.naming.strategy, NamingStrategy::UseRouteBased);
        assert_eq!(config.generators.len(), 2);

        let node = &config.generators[&GeneratorId::NodeClient];
        assert_eq!(node.output, "out/node");
        assert_eq!(node.layout, OutputLayout::Modular);
        assert_eq!(node.base_url, Some("https://api.example.com".to_string()));
        assert!(node.scaffold.is_some());
        let scaffold = node.scaffold.as_ref().unwrap();
        assert_eq!(scaffold["package_name"], "@myorg/client");
        assert_eq!(scaffold["formatter"], "biome");
        assert_eq!(scaffold["bundler"], "tsdown");

        let react = &config.generators[&GeneratorId::ReactSwrClient];
        assert_eq!(react.output, "out/react");
        assert_eq!(react.layout, OutputLayout::Split);
        assert_eq!(react.split_by, Some(SplitBy::Tag));
    }

    #[test]
    fn test_parse_legacy_typescript() {
        let yaml = r#"
input: spec.yaml
output: out
target: typescript
naming:
  strategy: use_operation_id
  aliases: {}
output_options:
  layout: single
  biome: true
  tsdown: true
client:
  base_url: https://api.example.com
  no_jsdoc: true
"#;
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.input, "spec.yaml");
        assert_eq!(config.generators.len(), 1);
        assert!(config.generators.contains_key(&GeneratorId::NodeClient));

        let node_gen = &config.generators[&GeneratorId::NodeClient];
        assert_eq!(node_gen.output, "out");
        assert_eq!(
            node_gen.base_url,
            Some("https://api.example.com".to_string())
        );
        assert_eq!(node_gen.no_jsdoc, Some(true));
    }

    #[test]
    fn test_parse_legacy_react() {
        let yaml = r#"
input: spec.yaml
output: out
target: react
"#;
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.generators.len(), 1);
        assert!(config.generators.contains_key(&GeneratorId::ReactSwrClient));
    }

    #[test]
    fn test_parse_legacy_all_single() {
        let yaml = r#"
input: spec.yaml
output: out
target: all
output_options:
  layout: single
"#;
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        // Single layout with "all" maps to react-swr-client (which includes TS)
        assert_eq!(config.generators.len(), 1);
        assert!(config.generators.contains_key(&GeneratorId::ReactSwrClient));
    }

    #[test]
    fn test_parse_legacy_all_split() {
        let yaml = r#"
input: spec.yaml
output: out
target: all
output_options:
  layout: split
"#;
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.generators.len(), 2);
        assert!(config.generators.contains_key(&GeneratorId::NodeClient));
        assert!(config.generators.contains_key(&GeneratorId::ReactSwrClient));
        assert_eq!(
            config.generators[&GeneratorId::NodeClient].output,
            "out/typescript"
        );
        assert_eq!(
            config.generators[&GeneratorId::ReactSwrClient].output,
            "out/react"
        );
    }

    #[test]
    fn test_tool_setting_resolve() {
        assert_eq!(ToolSetting::resolve(None, "biome"), Some("biome"));
        assert_eq!(
            ToolSetting::resolve(Some(&ToolSetting::Named("ruff".into())), "biome"),
            Some("ruff")
        );
        assert_eq!(
            ToolSetting::resolve(Some(&ToolSetting::Disabled), "biome"),
            None
        );
    }

    #[test]
    fn test_tool_setting_deserialize() {
        let named: ToolSetting = serde_json::from_value(serde_json::json!("biome")).unwrap();
        assert_eq!(named, ToolSetting::Named("biome".into()));

        let disabled: ToolSetting = serde_json::from_value(serde_json::json!(false)).unwrap();
        assert_eq!(disabled, ToolSetting::Disabled);

        let err = serde_json::from_value::<ToolSetting>(serde_json::json!(true));
        assert!(err.is_err());
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = "input: api.yaml\n";
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let config: OagConfig = serde_json::from_value(value).unwrap();
        assert_eq!(config.input, "api.yaml");
        // Legacy format with defaults: target=all, layout=single -> react-swr-client
        assert_eq!(config.generators.len(), 1);
    }
}
