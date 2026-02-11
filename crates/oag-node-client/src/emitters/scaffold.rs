use minijinja::{Environment, context};
use oag_core::GeneratedFile;
use oag_core::config::ToolSetting;
use serde::Deserialize;

/// Node/TS-specific scaffold configuration, parsed from the opaque `serde_json::Value`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct NodeScaffoldConfig {
    pub package_name: Option<String>,
    pub repository: Option<String>,
    pub index: Option<bool>,
    pub formatter: Option<ToolSetting>,
    pub test_runner: Option<ToolSetting>,
    pub bundler: Option<ToolSetting>,
}

/// Options controlling which scaffold files to generate.
#[derive(Debug, Clone)]
pub struct ScaffoldOptions {
    /// Spec title, used as fallback for package name.
    pub name: String,
    /// Custom package name override (if None, derives from spec title).
    pub package_name: Option<String>,
    /// Repository URL for package.json.
    pub repository: Option<String>,
    /// Formatter tool name (e.g. "biome") or None if disabled.
    pub formatter: Option<String>,
    /// Test runner tool name (e.g. "vitest") or None if disabled.
    pub test_runner: Option<String>,
    /// Bundler tool name (e.g. "tsdown") or None if disabled.
    pub bundler: Option<String>,
    /// Whether React target is included.
    pub react: bool,
}

/// Generate project scaffold files (package.json, tsconfig.json, biome.json, tsdown.config.ts).
pub fn emit_scaffold(options: &ScaffoldOptions) -> Vec<GeneratedFile> {
    let mut files = Vec::new();

    // package.json
    files.push(GeneratedFile {
        path: "package.json".to_string(),
        content: emit_package_json(options),
    });

    // tsconfig.json
    files.push(GeneratedFile {
        path: "tsconfig.json".to_string(),
        content: emit_tsconfig(options),
    });

    // biome.json (optional)
    if options.formatter.as_deref() == Some("biome") {
        files.push(GeneratedFile {
            path: "biome.json".to_string(),
            content: emit_biome(),
        });
    }

    // tsdown.config.ts (optional)
    if options.bundler.as_deref() == Some("tsdown") {
        files.push(GeneratedFile {
            path: "tsdown.config.ts".to_string(),
            content: emit_tsdown(),
        });
    }

    files
}

fn emit_package_json(options: &ScaffoldOptions) -> String {
    let mut env = Environment::new();
    env.add_template(
        "package.json.j2",
        include_str!("../../templates/package.json.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("package.json.j2").unwrap();

    let pkg_name = options
        .package_name
        .clone()
        .unwrap_or_else(|| slugify(&options.name));

    let biome = options.formatter.as_deref() == Some("biome");
    let vitest = options.test_runner.as_deref() == Some("vitest");
    let tsdown = options.bundler.as_deref() == Some("tsdown");

    tmpl.render(context! {
        name => pkg_name,
        repository => options.repository,
        react => options.react,
        biome => biome,
        vitest => vitest,
        tsdown => tsdown,
    })
    .expect("render should succeed")
}

fn emit_tsconfig(options: &ScaffoldOptions) -> String {
    let mut env = Environment::new();
    env.add_template(
        "tsconfig.json.j2",
        include_str!("../../templates/tsconfig.json.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("tsconfig.json.j2").unwrap();

    tmpl.render(context! {
        react => options.react,
    })
    .expect("render should succeed")
}

fn emit_biome() -> String {
    include_str!("../../templates/biome.json.j2").to_string()
}

fn emit_tsdown() -> String {
    include_str!("../../templates/tsdown.config.ts.j2").to_string()
}

/// Convert a title to a kebab-case package name.
fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive dashes and trim
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash && !result.is_empty() {
                result.push('-');
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    result.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My API Service"), "my-api-service");
        assert_eq!(slugify("SSE Chat API"), "sse-chat-api");
        assert_eq!(slugify("Petstore - OpenAPI 3.2"), "petstore-openapi-3-2");
    }

    #[test]
    fn test_emit_scaffold_with_all_options() {
        let options = ScaffoldOptions {
            name: "Test API".to_string(),
            package_name: None,
            repository: Some("https://github.com/test/repo".to_string()),
            formatter: Some("biome".to_string()),
            bundler: Some("tsdown".to_string()),
            test_runner: Some("vitest".to_string()),
            react: true,
        };
        let files = emit_scaffold(&options);
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|f| f.path == "package.json"));
        assert!(files.iter().any(|f| f.path == "tsconfig.json"));
        assert!(files.iter().any(|f| f.path == "biome.json"));
        assert!(files.iter().any(|f| f.path == "tsdown.config.ts"));
    }

    #[test]
    fn test_emit_scaffold_minimal() {
        let options = ScaffoldOptions {
            name: "Test".to_string(),
            package_name: None,
            repository: None,
            formatter: None,
            bundler: None,
            test_runner: None,
            react: false,
        };
        let files = emit_scaffold(&options);
        assert_eq!(files.len(), 2); // Only package.json + tsconfig.json
    }

    #[test]
    fn test_custom_package_name() {
        let options = ScaffoldOptions {
            name: "Some API".to_string(),
            package_name: Some("@myorg/api-client".to_string()),
            repository: None,
            formatter: None,
            bundler: None,
            test_runner: None,
            react: false,
        };
        let files = emit_scaffold(&options);
        let pkg = files.iter().find(|f| f.path == "package.json").unwrap();
        assert!(pkg.content.contains("@myorg/api-client"));
    }
}
