use minijinja::{Environment, context};
use oag_core::ir::{IrEnumVariant, IrObjectSchema, IrSchema, IrSpec};

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
            let is_integer = !e.variants.is_empty()
                && e.variants
                    .iter()
                    .all(|v| matches!(v, IrEnumVariant::Integer(_)));
            let variants: Vec<minijinja::Value> = e
                .variants
                .iter()
                .map(|v| match v {
                    IrEnumVariant::String(s) => context! {
                        name => heck::AsUpperCamelCase(s).to_string(),
                        value => format!("\"{}\"", s),
                    },
                    IrEnumVariant::Integer(i) => {
                        let name = if *i < 0 {
                            format!("Neg{}", i.unsigned_abs())
                        } else {
                            format!("Value{i}")
                        };
                        context! {
                            name => name,
                            value => i.to_string(),
                        }
                    }
                })
                .collect();
            context! {
                kind => "enum",
                name => e.name.pascal_case.clone(),
                description => e.description.clone(),
                variants => variants,
                is_integer => is_integer,
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

#[cfg(test)]
mod tests {
    use oag_core::ir::{IrEnumSchema, IrEnumVariant, IrInfo, IrSchema, IrSpec, NormalizedName};

    use super::*;

    fn make_enum_spec(variants: Vec<IrEnumVariant>) -> IrSpec {
        IrSpec {
            info: IrInfo {
                title: "test".into(),
                version: "0.0.0".into(),
                description: None,
            },
            servers: vec![],
            schemas: vec![IrSchema::Enum(IrEnumSchema {
                name: NormalizedName {
                    original: "Status".into(),
                    pascal_case: "Status".into(),
                    snake_case: "status".into(),
                    camel_case: "status".into(),
                    screaming_snake: "STATUS".into(),
                },
                description: None,
                variants,
            })],
            operations: vec![],
            modules: vec![],
        }
    }

    #[test]
    fn all_integer_enum_is_integer_true() {
        let spec = make_enum_spec(vec![
            IrEnumVariant::Integer(0),
            IrEnumVariant::Integer(1),
            IrEnumVariant::Integer(2),
        ]);
        let output = emit_models(&spec);
        assert!(
            output.contains("int, Enum"),
            "all-integer enum should inherit from (int, Enum)"
        );
        assert!(
            !output.contains("str, Enum"),
            "all-integer enum should NOT inherit from (str, Enum)"
        );
    }

    #[test]
    fn mixed_enum_is_integer_false() {
        let spec = make_enum_spec(vec![
            IrEnumVariant::String("active".into()),
            IrEnumVariant::Integer(1),
        ]);
        let output = emit_models(&spec);
        assert!(
            output.contains("str, Enum"),
            "mixed enum should inherit from (str, Enum)"
        );
        assert!(
            !output.contains("int, Enum"),
            "mixed enum should NOT inherit from (int, Enum)"
        );
    }

    #[test]
    fn all_string_enum_is_integer_false() {
        let spec = make_enum_spec(vec![
            IrEnumVariant::String("active".into()),
            IrEnumVariant::String("inactive".into()),
        ]);
        let output = emit_models(&spec);
        assert!(
            output.contains("str, Enum"),
            "string enum should inherit from (str, Enum)"
        );
        assert!(
            !output.contains("int, Enum"),
            "string enum should NOT inherit from (int, Enum)"
        );
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
