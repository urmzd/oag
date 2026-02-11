use minijinja::{Environment, context};
use oag_core::ir::{IrObjectSchema, IrSchema, IrSpec};

use crate::type_mapper::{ir_type_to_python, ir_type_to_python_field};

/// Emit `models.py` — Pydantic v2 BaseModel classes from IrSchema.
pub fn emit_models(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_template("models.py.j2", include_str!("../../templates/models.py.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("models.py.j2").unwrap();

    let schemas: Vec<_> = ir.schemas.iter().map(schema_to_ctx).collect();

    tmpl.render(context! {
        schemas => schemas,
    })
    .expect("render should succeed")
}

fn schema_to_ctx(schema: &IrSchema) -> minijinja::Value {
    match schema {
        IrSchema::Object(obj) => object_to_ctx(obj),
        IrSchema::Enum(e) => {
            let variants: Vec<minijinja::Value> = e
                .variants
                .iter()
                .map(|v| {
                    context! {
                        name => heck::AsUpperCamelCase(v).to_string(),
                        value => v.clone(),
                    }
                })
                .collect();
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
                target => ir_type_to_python(&a.target),
            }
        }
        IrSchema::Union(u) => {
            let variants: Vec<String> = u.variants.iter().map(ir_type_to_python).collect();
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
                name => f.name.snake_case.clone(),
                original_name => f.original_name.clone(),
                type_str => ir_type_to_python_field(&f.field_type, f.required),
                required => f.required,
                description => f.description.clone(),
                needs_alias => f.name.snake_case != f.original_name,
            }
        })
        .collect();

    let has_additional_properties = obj.additional_properties.is_some();

    context! {
        kind => "object",
        name => obj.name.pascal_case.clone(),
        description => obj.description.clone(),
        fields => fields,
        has_additional_properties => has_additional_properties,
    }
}
