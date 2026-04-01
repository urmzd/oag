use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::GeneratorError;

/// A loaded template pack ready for code generation.
#[derive(Debug, Clone)]
pub struct TemplatePack {
    pub manifest: PackManifest,
    /// Template name → template content.
    pub templates: HashMap<String, String>,
}

/// The `oag.pack.toml` manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct PackManifest {
    pub pack: PackMeta,
    #[serde(default)]
    pub type_map: TypeMapConfig,
    #[serde(default)]
    pub filters: HashMap<String, FilterConfig>,
    #[serde(default)]
    pub layouts: LayoutsConfig,
    #[serde(default)]
    pub scaffold: ScaffoldConfig,
    #[serde(default)]
    pub formatters: HashMap<String, FormatterConfig>,
    #[serde(default)]
    pub validators: HashMap<String, ValidatorConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackMeta {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// Inherit templates, type_map, filters, layouts from another pack.
    pub extends: Option<String>,
    /// Field name casing: "camel", "snake", "original".
    #[serde(default = "default_camel")]
    pub field_casing: String,
    /// Operation name casing: "camel", "snake", "pascal".
    #[serde(default = "default_camel")]
    pub operation_casing: String,
}

fn default_camel() -> String {
    "camel".to_string()
}

/// Declarative type mapping from IR types to language type strings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TypeMapConfig {
    // Primitives
    pub string: String,
    pub number: String,
    pub integer: String,
    pub boolean: String,
    pub null: String,
    pub datetime: String,
    pub binary: String,
    pub any: String,
    pub void: String,

    // Parameterized
    pub string_literal: String,
    pub integer_literal: String,
    #[serde(rename = "ref")]
    pub ref_type: String,
    pub array: String,
    /// Used when inner type is a union; falls back to `array` if absent.
    pub array_union: Option<String>,
    pub map: String,
    pub object: String,
    pub object_empty: String,
    pub object_field_required: String,
    pub object_field_optional: String,
    #[serde(default = "default_semicolon_sep")]
    pub object_field_separator: String,
    #[serde(default = "default_pipe_sep")]
    pub union_separator: String,
    #[serde(default = "default_ampersand_sep")]
    pub intersection_separator: String,

    /// Suffix for optional fields (e.g. " | None = None" for Python).
    #[serde(default)]
    pub optional_suffix: String,
}

fn default_semicolon_sep() -> String {
    "; ".to_string()
}
fn default_pipe_sep() -> String {
    " | ".to_string()
}
fn default_ampersand_sep() -> String {
    " & ".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterConfig {
    pub replace: String,
    pub with: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LayoutsConfig {
    pub modular: Option<ModularLayout>,
    pub bundled: Option<BundledLayout>,
    pub split: Option<SplitLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModularLayout {
    #[serde(default)]
    pub files: Vec<LayoutFile>,
    /// Additional files appended when pack extends another (used by react-swr-client).
    #[serde(default)]
    pub extra_files: Vec<LayoutFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutFile {
    pub path: String,
    pub template: String,
    /// Optional condition expression (evaluated as minijinja expression).
    pub when: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundledLayout {
    pub output_path: String,
    #[serde(default)]
    pub sections: Vec<BundledSection>,
    #[serde(default)]
    pub strip_patterns: Vec<String>,
    #[serde(default)]
    pub strip_import_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundledSection {
    pub label: String,
    pub template: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SplitLayout {
    #[serde(default)]
    pub shared_files: Vec<LayoutFile>,
    pub group_template: Option<String>,
    pub index_template: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ScaffoldConfig {
    #[serde(default)]
    pub files: Vec<LayoutFile>,
    #[serde(default)]
    pub test_files: Vec<LayoutFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormatterConfig {
    pub detect: String,
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidatorConfig {
    pub detect: String,
    pub command: String,
    /// Optional setup command to run before validation (e.g. `npm install`).
    pub setup: Option<String>,
}

impl TemplatePack {
    /// Load a template pack from a directory containing `oag.pack.toml` and `templates/`.
    pub fn from_dir(dir: &Path) -> Result<Self, GeneratorError> {
        let manifest_path = dir.join("oag.pack.toml");
        let manifest_str = std::fs::read_to_string(&manifest_path).map_err(|e| {
            GeneratorError::Other(format!("failed to read {}: {e}", manifest_path.display()))
        })?;
        let manifest: PackManifest = toml::from_str(&manifest_str).map_err(|e| {
            GeneratorError::Other(format!("failed to parse {}: {e}", manifest_path.display()))
        })?;

        let templates_dir = dir.join("templates");
        let mut templates = HashMap::new();
        if templates_dir.is_dir() {
            for entry in std::fs::read_dir(&templates_dir).map_err(|e| {
                GeneratorError::Other(format!("failed to read {}: {e}", templates_dir.display()))
            })? {
                let entry = entry.map_err(|e| GeneratorError::Other(e.to_string()))?;
                let path = entry.path();
                if path.is_file() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let content = std::fs::read_to_string(&path).map_err(|e| {
                        GeneratorError::Other(format!("failed to read {}: {e}", path.display()))
                    })?;
                    templates.insert(name, content);
                }
            }
        }

        Ok(Self {
            manifest,
            templates,
        })
    }

    /// Merge another pack into this one (for `extends` support).
    /// The extending pack's entries override the base.
    pub fn merge_from(&mut self, extending: &TemplatePack) {
        // Override templates
        for (name, content) in &extending.templates {
            self.templates.insert(name.clone(), content.clone());
        }

        // Override type_map if the extending pack defines non-empty entries
        // (we use the extending manifest wholesale since TOML defaults fill in)
        if !extending.manifest.type_map.string.is_empty() {
            self.manifest.type_map = extending.manifest.type_map.clone();
        }

        // Merge filters
        for (name, filter) in &extending.manifest.filters {
            self.manifest.filters.insert(name.clone(), filter.clone());
        }

        // Merge layouts: extra_files for modular
        if let Some(ref ext_modular) = extending.manifest.layouts.modular {
            if let Some(ref mut base_modular) = self.manifest.layouts.modular {
                base_modular.files.extend(ext_modular.extra_files.clone());
                // Also add the extending pack's own files list
                base_modular.files.extend(ext_modular.files.clone());
            } else {
                self.manifest.layouts.modular = Some(ext_modular.clone());
            }
        }

        // Override bundled/split if provided
        if extending.manifest.layouts.bundled.is_some() {
            self.manifest.layouts.bundled = extending.manifest.layouts.bundled.clone();
        }
        if extending.manifest.layouts.split.is_some() {
            self.manifest.layouts.split = extending.manifest.layouts.split.clone();
        }

        // Merge scaffold
        if !extending.manifest.scaffold.files.is_empty() {
            self.manifest.scaffold = extending.manifest.scaffold.clone();
        }

        // Merge formatters
        for (name, fmt) in &extending.manifest.formatters {
            self.manifest.formatters.insert(name.clone(), fmt.clone());
        }

        // Merge validators
        for (name, val) in &extending.manifest.validators {
            self.manifest.validators.insert(name.clone(), val.clone());
        }

        // Update pack metadata
        self.manifest.pack = extending.manifest.pack.clone();
    }
}
