use minijinja::{Environment, context};
use oag_core::GeneratedFile;

/// Options controlling which scaffold files to generate.
#[derive(Debug, Clone)]
pub struct ScaffoldOptions {
    /// Spec title, used as fallback for package name.
    pub name: String,
    /// Custom package name override (if None, derives from spec title).
    pub package_name: Option<String>,
    /// Repository URL for package.json.
    pub repository: Option<String>,
    /// Generate `biome.json`.
    pub biome: bool,
    /// Generate `tsdown.config.ts`.
    pub tsdown: bool,
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
    if options.biome {
        files.push(GeneratedFile {
            path: "biome.json".to_string(),
            content: emit_biome(),
        });
    }

    // tsdown.config.ts (optional)
    if options.tsdown {
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

    tmpl.render(context! {
        name => pkg_name,
        repository => options.repository,
        react => options.react,
        biome => options.biome,
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
            biome: true,
            tsdown: true,
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
            biome: false,
            tsdown: false,
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
            biome: false,
            tsdown: false,
            react: false,
        };
        let files = emit_scaffold(&options);
        let pkg = files.iter().find(|f| f.path == "package.json").unwrap();
        assert!(pkg.content.contains("@myorg/api-client"));
    }
}
