use minijinja::{Environment, context};
use oag_core::ir::{IrObjectSchema, IrReturnType, IrSchema, IrSpec};

use crate::type_mapper::ir_type_to_ts;

/// Emit `types.ts` containing all interfaces, enums, aliases, and SSE event union types.
pub fn emit_types(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_template("types.ts.j2", include_str!("../../templates/types.ts.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("types.ts.j2").unwrap();

    let schemas: Vec<_> = ir.schemas.iter().map(schema_to_ctx).collect();
    let sse_event_types = collect_sse_event_types(ir);

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

fn collect_sse_event_types(ir: &IrSpec) -> Vec<minijinja::Value> {
    let mut event_types = Vec::new();
    for op in &ir.operations {
        if let IrReturnType::Sse(sse) = &op.return_type
            && let Some(ref event_name) = sse.event_type_name
        {
            let variants: Vec<String> = sse.variants.iter().map(ir_type_to_ts).collect();
            if !variants.is_empty() {
                event_types.push(context! {
                    name => event_name.clone(),
                    variants => variants,
                });
            }
        }
    }
    event_types
}
