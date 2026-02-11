use std::collections::HashSet;

use minijinja::{Environment, context};
use oag_core::ir::{IrObjectSchema, IrReturnType, IrSchema, IrSpec};

use crate::type_mapper::ir_type_to_ts;

/// Escape `*/` sequences that would prematurely close JSDoc comment blocks.
fn escape_jsdoc(value: String) -> String {
    value.replace("*/", "*\\/")
}

/// Emit `types.ts` containing all interfaces, enums, aliases, and SSE event union types.
pub fn emit_types(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.add_filter("escape_jsdoc", escape_jsdoc);
    env.add_template("types.ts.j2", include_str!("../../templates/types.ts.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("types.ts.j2").unwrap();

    let schemas: Vec<_> = ir.schemas.iter().map(schema_to_ctx).collect();
    let schema_names: HashSet<String> = ir
        .schemas
        .iter()
        .map(|s| match s {
            IrSchema::Object(o) => o.name.pascal_case.clone(),
            IrSchema::Enum(e) => e.name.pascal_case.clone(),
            IrSchema::Alias(a) => a.name.pascal_case.clone(),
            IrSchema::Union(u) => u.name.pascal_case.clone(),
        })
        .collect();
    let sse_event_types = collect_sse_event_types(ir, &schema_names);

    tmpl.render(context! {
        schemas => schemas,
        sse_event_types => sse_event_types,
    })
    .expect("render should succeed")
}

fn schema_to_ctx(schema: &IrSchema) -> minijinja::Value {
    match schema {
        IrSchema::Object(obj) => object_to_ctx(obj),
        IrSchema::Enum(e) => {
            let variants: Vec<String> = e.variants.iter().map(|v| format!("\"{v}\"")).collect();
            context! {
                kind => "enum",
                name => e.name.pascal_case.clone(),
                description => e.description.clone(),
                variants => variants,
            }
        }
        IrSchema::Alias(a) => {
            context! {
                kind => "alias",
                name => a.name.pascal_case.clone(),
                description => a.description.clone(),
                target => ir_type_to_ts(&a.target),
            }
        }
        IrSchema::Union(u) => {
            let variants: Vec<String> = u.variants.iter().map(ir_type_to_ts).collect();
            context! {
                kind => "union",
                name => u.name.pascal_case.clone(),
                description => u.description.clone(),
                variants => variants,
            }
        }
    }
}

fn object_to_ctx(obj: &IrObjectSchema) -> minijinja::Value {
    let fields: Vec<minijinja::Value> = obj
        .fields
        .iter()
        .map(|f| {
            context! {
                name => f.name.camel_case.clone(),
                original_name => f.original_name.clone(),
                type => ir_type_to_ts(&f.field_type),
                required => f.required,
                description => f.description.clone(),
            }
        })
        .collect();

    let additional = obj.additional_properties.as_ref().map(ir_type_to_ts);

    context! {
        kind => "object",
        name => obj.name.pascal_case.clone(),
        description => obj.description.clone(),
        fields => fields,
        additional_properties => additional,
    }
}

fn collect_sse_event_types(ir: &IrSpec, schema_names: &HashSet<String>) -> Vec<minijinja::Value> {
    let mut event_types = Vec::new();
    let mut seen = HashSet::new();
    for op in &ir.operations {
        if let IrReturnType::Sse(sse) = &op.return_type
            && let Some(ref event_name) = sse.event_type_name
        {
            if seen.contains(event_name) || schema_names.contains(event_name) {
                continue;
            }
            let variants: Vec<String> = sse.variants.iter().map(ir_type_to_ts).collect();
            if !variants.is_empty() {
                seen.insert(event_name.clone());
                event_types.push(context! {
                    name => event_name.clone(),
                    variants => variants,
                });
            }
        }
    }
    event_types
}
