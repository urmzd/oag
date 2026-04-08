pub mod bundled;
pub mod context;
pub mod pack;
pub mod resolve;
pub mod type_map;

use minijinja::Environment;

use crate::config::{GeneratorConfig, OutputLayout, SplitBy};
use crate::ir::{IrSpec, group_operations};
use crate::{GenerateOutput, GeneratedFile, GeneratorError, normalize_generated};

use self::pack::{FilterConfig, TemplatePack};

/// Generate code files from an IR spec using a template pack.
///
/// Returns a [`GenerateOutput`] with source files (always overwritten) and
/// scaffold files (write-once by default).
pub fn generate(
    ir: &IrSpec,
    config: &GeneratorConfig,
    pack: &TemplatePack,
) -> Result<GenerateOutput, GeneratorError> {
    let tm = &pack.manifest.type_map;
    let field_casing = &pack.manifest.pack.field_casing;
    let operation_casing = &pack.manifest.pack.operation_casing;

    // Build universal context
    let ctx = context::build_context(ir, tm, field_casing, operation_casing);

    // Build minijinja environment
    let mut env = Environment::new();
    env.set_trim_blocks(true);

    // Register templates
    for (name, content) in &pack.templates {
        env.add_template(name, content)
            .map_err(|e| GeneratorError::Render(format!("template '{name}': {e}")))?;
    }

    // Register filters
    for (name, filter_config) in &pack.manifest.filters {
        register_filter(&mut env, name, filter_config);
    }

    // Build render context with config values included
    let scaffold_ctx = config
        .scaffold
        .as_ref()
        .map(minijinja::Value::from_serialize)
        .unwrap_or(minijinja::Value::UNDEFINED);

    let source_dir = &config.source_dir;
    let no_jsdoc = config.no_jsdoc.unwrap_or(false);

    let config_ctx = minijinja::context! {
        scaffold => scaffold_ctx,
        source_dir => source_dir,
        no_jsdoc => no_jsdoc,
        base_url => config.base_url.clone(),
    };

    // Merge: base context first, then config overlay
    let render_ctx = minijinja::context! {
        ..ctx,
        ..config_ctx,
    };

    let mut source_files = match config.layout {
        OutputLayout::Bundled => render_bundled(&env, pack, &render_ctx, source_dir)?,
        OutputLayout::Modular => render_modular(&env, pack, &render_ctx, source_dir)?,
        OutputLayout::Split => {
            let split_by = config.split_by.unwrap_or(SplitBy::Tag);
            render_split(&env, pack, &render_ctx, source_dir, ir, no_jsdoc, split_by)?
        }
    };

    let mut scaffold_files = render_scaffold(&env, pack, &render_ctx, source_dir)?;

    // Normalize whitespace
    for file in &mut source_files {
        file.content = normalize_generated(&file.content);
    }
    for file in &mut scaffold_files {
        file.content = normalize_generated(&file.content);
    }

    Ok(GenerateOutput {
        source_files,
        scaffold_files,
    })
}

fn register_filter(env: &mut Environment, name: &str, config: &FilterConfig) {
    let replace = config.replace.clone();
    let with = config.with.clone();
    env.add_filter(name.to_string(), move |value: String| -> String {
        value.replace(&replace, &with)
    });
}

fn render_modular(
    env: &Environment,
    pack: &TemplatePack,
    ctx: &minijinja::Value,
    source_dir: &str,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let layout =
        pack.manifest.layouts.modular.as_ref().ok_or_else(|| {
            GeneratorError::Other("pack has no modular layout defined".to_string())
        })?;

    let mut files = Vec::new();
    for file_def in &layout.files {
        let path = file_def.path.replace("{source_dir}", source_dir);
        let path = normalize_path(&path);

        // Check `when` condition if present
        if let Some(ref when) = file_def.when
            && !eval_condition(env, when, ctx)
        {
            continue;
        }

        // Some templates are static (not jinja), render them as-is
        let content = if env.get_template(&file_def.template).is_ok() {
            let tmpl = env.get_template(&file_def.template).unwrap();
            tmpl.render(ctx)
                .map_err(|e| GeneratorError::Render(format!("{}: {e}", file_def.template)))?
        } else {
            // Template not found — it might be a static file
            pack.templates
                .get(&file_def.template)
                .cloned()
                .unwrap_or_default()
        };

        files.push(GeneratedFile { path, content });
    }

    Ok(files)
}

fn render_bundled(
    env: &Environment,
    pack: &TemplatePack,
    ctx: &minijinja::Value,
    source_dir: &str,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let layout =
        pack.manifest.layouts.bundled.as_ref().ok_or_else(|| {
            GeneratorError::Other("pack has no bundled layout defined".to_string())
        })?;

    let mut rendered_sections = Vec::new();
    for section in &layout.sections {
        let tmpl = env
            .get_template(&section.template)
            .map_err(|e| GeneratorError::Render(format!("{}: {e}", section.template)))?;
        let content = tmpl
            .render(ctx)
            .map_err(|e| GeneratorError::Render(format!("{}: {e}", section.template)))?;
        rendered_sections.push((section.label.as_str(), content));
    }

    let bundled_content = bundled::bundle_sections(layout, rendered_sections);
    let path = layout.output_path.replace("{source_dir}", source_dir);
    let path = normalize_path(&path);

    Ok(vec![GeneratedFile {
        path,
        content: bundled_content,
    }])
}

fn render_split(
    env: &Environment,
    pack: &TemplatePack,
    ctx: &minijinja::Value,
    source_dir: &str,
    ir: &IrSpec,
    _no_jsdoc: bool,
    split_by: SplitBy,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let layout = pack
        .manifest
        .layouts
        .split
        .as_ref()
        .ok_or_else(|| GeneratorError::Other("pack has no split layout defined".to_string()))?;

    let mut files = Vec::new();

    // Render shared files
    for file_def in &layout.shared_files {
        let path = file_def.path.replace("{source_dir}", source_dir);
        let path = normalize_path(&path);
        let tmpl = env
            .get_template(&file_def.template)
            .map_err(|e| GeneratorError::Render(format!("{}: {e}", file_def.template)))?;
        let content = tmpl
            .render(ctx)
            .map_err(|e| GeneratorError::Render(format!("{}: {e}", file_def.template)))?;
        files.push(GeneratedFile { path, content });
    }

    // Also render client.ts (the full client is always needed for split)
    // This is included in shared_files by the pack manifest

    // Group operations and create per-group files
    let groups = group_operations(ir, split_by);
    let mut group_names = Vec::new();

    for group in &groups {
        let group_file_name =
            normalize_path(&format!("{}/{}.ts", source_dir, group.name.snake_case));

        // Build a simple re-export file for the group
        let op_names: Vec<&str> = group
            .operation_indices
            .iter()
            .map(|&i| ir.operations[i].name.camel_case.as_str())
            .collect();

        let mut lines = Vec::new();
        lines.push("// Auto-generated by oag — do not edit".to_string());
        lines.push(format!("// Operations group: {}", group.name.original));
        lines.push(String::new());
        lines.push("// This group contains the following operations:".to_string());
        for name in &op_names {
            lines.push(format!("//   - {name}"));
        }
        lines.push(String::new());
        lines.push("// Import the client and call the relevant methods:".to_string());
        lines.push("// import { ApiClient } from \"./client\";".to_string());
        lines.push(String::new());
        lines.push("export { ApiClient } from \"./client\";".to_string());
        lines.push("export * from \"./types\";".to_string());

        group_names.push(group.name.snake_case.clone());
        files.push(GeneratedFile {
            path: group_file_name,
            content: lines.join("\n") + "\n",
        });
    }

    // Split index
    let mut index_lines = vec![
        "// Auto-generated by oag — do not edit".to_string(),
        "export * from \"./types\";".to_string(),
        "export * from \"./guards\";".to_string(),
        "export { ApiClient, type ClientConfig, type RequestOptions } from \"./client\";"
            .to_string(),
        "export { streamSse, SSEError, type SSEOptions } from \"./sse\";".to_string(),
    ];
    for name in &group_names {
        index_lines.push(format!("export * from \"./{name}\";"));
    }
    let index_path = normalize_path(&format!("{}/index.ts", source_dir));
    files.push(GeneratedFile {
        path: index_path,
        content: index_lines.join("\n") + "\n",
    });

    Ok(files)
}

fn render_scaffold(
    env: &Environment,
    pack: &TemplatePack,
    ctx: &minijinja::Value,
    source_dir: &str,
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    // Only render scaffold files if scaffold config is present
    let has_scaffold = ctx
        .get_attr("scaffold")
        .ok()
        .is_some_and(|v| !v.is_undefined());
    if !has_scaffold {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    // Scaffold files
    for file_def in &pack.manifest.scaffold.files {
        if let Some(ref when) = file_def.when
            && !eval_condition(env, when, ctx)
        {
            continue;
        }
        let path = file_def.path.replace("{source_dir}", source_dir);
        let path = normalize_path(&path);

        let content = if let Ok(tmpl) = env.get_template(&file_def.template) {
            tmpl.render(ctx)
                .map_err(|e| GeneratorError::Render(format!("{}: {e}", file_def.template)))?
        } else {
            pack.templates
                .get(&file_def.template)
                .cloned()
                .unwrap_or_default()
        };

        files.push(GeneratedFile { path, content });
    }

    // Test files
    for file_def in &pack.manifest.scaffold.test_files {
        if let Some(ref when) = file_def.when
            && !eval_condition(env, when, ctx)
        {
            continue;
        }
        let path = file_def.path.replace("{source_dir}", source_dir);
        let path = normalize_path(&path);

        let content = if let Ok(tmpl) = env.get_template(&file_def.template) {
            tmpl.render(ctx)
                .map_err(|e| GeneratorError::Render(format!("{}: {e}", file_def.template)))?
        } else {
            pack.templates
                .get(&file_def.template)
                .cloned()
                .unwrap_or_default()
        };

        files.push(GeneratedFile { path, content });
    }

    Ok(files)
}

/// Evaluate a condition string as a minijinja expression.
fn eval_condition(env: &Environment, condition: &str, ctx: &minijinja::Value) -> bool {
    // Create a temporary template that just evaluates the expression
    let expr_template = format!("{{% if {condition} %}}true{{% endif %}}");
    let mut temp_env = env.clone();
    if temp_env
        .add_template("__condition__", &expr_template)
        .is_err()
    {
        return false;
    }
    let Ok(tmpl) = temp_env.get_template("__condition__") else {
        return false;
    };
    tmpl.render(ctx).ok().is_some_and(|s| s.trim() == "true")
}

/// Normalize a file path: remove leading `./` and double `/`.
fn normalize_path(path: &str) -> String {
    let path = path.replace("//", "/");
    let path = path.strip_prefix("./").unwrap_or(&path);
    if let Some(stripped) = path.strip_prefix('/') {
        stripped.to_string()
    } else {
        path.to_string()
    }
}
