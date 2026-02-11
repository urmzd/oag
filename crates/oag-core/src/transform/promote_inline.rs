use std::collections::HashSet;

use heck::ToPascalCase;

use crate::ir::{IrField, IrObjectSchema, IrSchema, IrSpec, IrType};

use super::name_normalizer::normalize_name;

/// Promote inline `IrType::Object(fields)` with non-empty fields into named
/// `IrSchema::Object` entries, replacing them with `IrType::Ref`.
///
/// This benefits all generators: Python gets proper Pydantic models instead of
/// `dict[str, Any]`, and TypeScript gets named interfaces instead of inline types.
pub fn promote_inline_objects(ir: &mut IrSpec) {
    let mut used_names: HashSet<String> = ir
        .schemas
        .iter()
        .map(|s| s.name().pascal_case.clone())
        .collect();

    let mut new_schemas: Vec<IrSchema> = Vec::new();

    // Phase 1: Walk existing schemas and promote inline objects in their fields
    for schema in &mut ir.schemas {
        if let IrSchema::Object(obj) = schema {
            let context = obj.name.pascal_case.clone();
            for field in &mut obj.fields {
                let field_context = format!("{}{}", context, field.name.pascal_case);
                promote_type(
                    &field_context,
                    &mut field.field_type,
                    &mut new_schemas,
                    &mut used_names,
                );
            }
        }
    }

    // Phase 2: Walk operations and promote inline objects in return types, request bodies, and parameters
    for op in &mut ir.operations {
        let op_pascal = op.name.pascal_case.clone();

        // Return type
        match &mut op.return_type {
            crate::ir::IrReturnType::Standard(resp) => {
                let ctx = format!("{}Response", op_pascal);
                promote_type(
                    &ctx,
                    &mut resp.response_type,
                    &mut new_schemas,
                    &mut used_names,
                );
            }
            crate::ir::IrReturnType::Sse(sse) => {
                let ctx = format!("{}Event", op_pascal);
                promote_type(&ctx, &mut sse.event_type, &mut new_schemas, &mut used_names);
                for variant in &mut sse.variants {
                    promote_type(&ctx, variant, &mut new_schemas, &mut used_names);
                }
                if let Some(ref mut json_resp) = sse.json_response {
                    let json_ctx = format!("{}Response", op_pascal);
                    promote_type(
                        &json_ctx,
                        &mut json_resp.response_type,
                        &mut new_schemas,
                        &mut used_names,
                    );
                }
            }
            crate::ir::IrReturnType::Void => {}
        }

        // Request body
        if let Some(ref mut body) = op.request_body {
            let ctx = format!("{}Body", op_pascal);
            promote_type(&ctx, &mut body.body_type, &mut new_schemas, &mut used_names);
        }

        // Parameters
        for param in &mut op.parameters {
            let ctx = format!("{}{}", op_pascal, param.name.pascal_case);
            promote_type(
                &ctx,
                &mut param.param_type,
                &mut new_schemas,
                &mut used_names,
            );
        }
    }

    ir.schemas.extend(new_schemas);
}

/// Recursively walk an `IrType`, promoting any `IrType::Object(fields)` with
/// non-empty fields into a named schema and replacing it with `IrType::Ref`.
fn promote_type(
    context_name: &str,
    ir_type: &mut IrType,
    new_schemas: &mut Vec<IrSchema>,
    used_names: &mut HashSet<String>,
) {
    match ir_type {
        IrType::Object(fields) if !fields.is_empty() => {
            let name = unique_name(context_name, used_names);

            // Convert (String, IrType, bool) tuples to IrField
            let mut ir_fields: Vec<IrField> = fields
                .drain(..)
                .map(|(field_name, field_type, required)| IrField {
                    name: normalize_name(&field_name),
                    original_name: field_name,
                    field_type,
                    required,
                    description: None,
                    read_only: false,
                    write_only: false,
                })
                .collect();

            // Recurse into each field's type
            let schema_name = name.clone();
            for field in &mut ir_fields {
                let field_ctx = format!("{}{}", schema_name, field.name.pascal_case);
                promote_type(&field_ctx, &mut field.field_type, new_schemas, used_names);
            }

            new_schemas.push(IrSchema::Object(IrObjectSchema {
                name: normalize_name(&name),
                description: None,
                fields: ir_fields,
                additional_properties: None,
            }));

            *ir_type = IrType::Ref(name);
        }
        IrType::Array(inner) => {
            let item_ctx = format!("{}Item", context_name);
            promote_type(&item_ctx, inner, new_schemas, used_names);
        }
        IrType::Map(inner) => {
            let value_ctx = format!("{}Value", context_name);
            promote_type(&value_ctx, inner, new_schemas, used_names);
        }
        IrType::Union(variants) => {
            for (i, variant) in variants.iter_mut().enumerate() {
                let variant_ctx = format!("{}Variant{}", context_name, i + 1);
                promote_type(&variant_ctx, variant, new_schemas, used_names);
            }
        }
        _ => {}
    }
}

/// Generate a unique PascalCase name, appending numeric suffixes if needed.
fn unique_name(base: &str, used_names: &mut HashSet<String>) -> String {
    let pascal = base.to_pascal_case();
    if used_names.insert(pascal.clone()) {
        return pascal;
    }
    let mut i = 2;
    loop {
        let candidate = format!("{}{}", pascal, i);
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn make_spec_with_inline_object() -> IrSpec {
        IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![IrSchema::Object(IrObjectSchema {
                name: normalize_name("Pet"),
                description: None,
                fields: vec![IrField {
                    name: normalize_name("owner"),
                    original_name: "owner".to_string(),
                    field_type: IrType::Object(vec![
                        ("name".to_string(), IrType::String, true),
                        ("age".to_string(), IrType::Integer, false),
                    ]),
                    required: true,
                    description: None,
                    read_only: false,
                    write_only: false,
                }],
                additional_properties: None,
            })],
            operations: vec![],
            modules: vec![],
        }
    }

    #[test]
    fn promotes_inline_object_in_schema_field() {
        let mut ir = make_spec_with_inline_object();
        promote_inline_objects(&mut ir);

        // The Pet schema's owner field should now be a Ref
        let pet = match &ir.schemas[0] {
            IrSchema::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert!(matches!(&pet.fields[0].field_type, IrType::Ref(name) if name == "PetOwner"));

        // A new schema PetOwner should exist
        assert_eq!(ir.schemas.len(), 2);
        let owner = match &ir.schemas[1] {
            IrSchema::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert_eq!(owner.name.pascal_case, "PetOwner");
        assert_eq!(owner.fields.len(), 2);
        assert_eq!(owner.fields[0].original_name, "name");
        assert_eq!(owner.fields[1].original_name, "age");
    }

    #[test]
    fn promotes_inline_object_in_operation_return_type() {
        let mut ir = IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![],
            operations: vec![IrOperation {
                name: normalize_name("getPet"),
                method: HttpMethod::Get,
                path: "/pet".to_string(),
                summary: None,
                description: None,
                tags: vec![],
                parameters: vec![],
                request_body: None,
                return_type: IrReturnType::Standard(IrResponse {
                    response_type: IrType::Object(vec![
                        ("id".to_string(), IrType::Integer, true),
                        ("name".to_string(), IrType::String, true),
                    ]),
                    description: None,
                }),
                deprecated: false,
            }],
            modules: vec![],
        };

        promote_inline_objects(&mut ir);

        // Return type should be promoted to a Ref
        match &ir.operations[0].return_type {
            IrReturnType::Standard(resp) => {
                assert!(matches!(&resp.response_type, IrType::Ref(n) if n == "GetPetResponse"));
            }
            _ => panic!("expected standard return"),
        }
        assert_eq!(ir.schemas.len(), 1);
    }

    #[test]
    fn promotes_nested_array_items() {
        let mut ir = IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![IrSchema::Object(IrObjectSchema {
                name: normalize_name("Response"),
                description: None,
                fields: vec![IrField {
                    name: normalize_name("items"),
                    original_name: "items".to_string(),
                    field_type: IrType::Array(Box::new(IrType::Object(vec![(
                        "id".to_string(),
                        IrType::Integer,
                        true,
                    )]))),
                    required: true,
                    description: None,
                    read_only: false,
                    write_only: false,
                }],
                additional_properties: None,
            })],
            operations: vec![],
            modules: vec![],
        };

        promote_inline_objects(&mut ir);

        let resp = match &ir.schemas[0] {
            IrSchema::Object(o) => o,
            _ => panic!("expected object"),
        };
        // Should be Array(Ref("ResponseItemsItem"))
        match &resp.fields[0].field_type {
            IrType::Array(inner) => {
                assert!(matches!(inner.as_ref(), IrType::Ref(n) if n == "ResponseItemsItem"));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn does_not_promote_empty_objects() {
        let mut ir = IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![IrSchema::Object(IrObjectSchema {
                name: normalize_name("Config"),
                description: None,
                fields: vec![IrField {
                    name: normalize_name("metadata"),
                    original_name: "metadata".to_string(),
                    field_type: IrType::Object(vec![]),
                    required: false,
                    description: None,
                    read_only: false,
                    write_only: false,
                }],
                additional_properties: None,
            })],
            operations: vec![],
            modules: vec![],
        };

        promote_inline_objects(&mut ir);

        // Empty objects should remain as IrType::Object([])
        let config = match &ir.schemas[0] {
            IrSchema::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert!(matches!(&config.fields[0].field_type, IrType::Object(f) if f.is_empty()));
        assert_eq!(ir.schemas.len(), 1); // No new schemas added
    }

    #[test]
    fn deduplicates_names() {
        let mut ir = IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![
                // Existing schema named "PetOwner"
                IrSchema::Object(IrObjectSchema {
                    name: normalize_name("PetOwner"),
                    description: None,
                    fields: vec![],
                    additional_properties: None,
                }),
                // Pet schema with inline owner field that would normally be "PetOwner"
                IrSchema::Object(IrObjectSchema {
                    name: normalize_name("Pet"),
                    description: None,
                    fields: vec![IrField {
                        name: normalize_name("owner"),
                        original_name: "owner".to_string(),
                        field_type: IrType::Object(vec![(
                            "name".to_string(),
                            IrType::String,
                            true,
                        )]),
                        required: true,
                        description: None,
                        read_only: false,
                        write_only: false,
                    }],
                    additional_properties: None,
                }),
            ],
            operations: vec![],
            modules: vec![],
        };

        promote_inline_objects(&mut ir);

        // Should get "PetOwner2" since "PetOwner" already exists
        let pet = match &ir.schemas[1] {
            IrSchema::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert!(matches!(&pet.fields[0].field_type, IrType::Ref(n) if n == "PetOwner2"));
    }

    #[test]
    fn promotes_request_body_inline_object() {
        let mut ir = IrSpec {
            info: IrInfo {
                title: "Test".to_string(),
                description: None,
                version: "1.0".to_string(),
            },
            servers: vec![],
            schemas: vec![],
            operations: vec![IrOperation {
                name: normalize_name("createPet"),
                method: HttpMethod::Post,
                path: "/pet".to_string(),
                summary: None,
                description: None,
                tags: vec![],
                parameters: vec![],
                request_body: Some(IrRequestBody {
                    body_type: IrType::Object(vec![("name".to_string(), IrType::String, true)]),
                    required: true,
                    content_type: "application/json".to_string(),
                    description: None,
                }),
                return_type: IrReturnType::Void,
                deprecated: false,
            }],
            modules: vec![],
        };

        promote_inline_objects(&mut ir);

        match &ir.operations[0].request_body {
            Some(body) => {
                assert!(matches!(&body.body_type, IrType::Ref(n) if n == "CreatePetBody"));
            }
            None => panic!("expected request body"),
        }
        assert_eq!(ir.schemas.len(), 1);
    }
}
