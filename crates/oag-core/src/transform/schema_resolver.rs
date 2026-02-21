use indexmap::IndexMap;

use crate::error::TransformError;
use crate::ir::{
    IrAliasSchema, IrDiscriminator, IrEnumSchema, IrField, IrObjectSchema, IrSchema, IrType,
    IrUnionSchema,
};
use crate::parse::schema::{AdditionalProperties, Schema, SchemaOrRef, SchemaType, TypeSet};

use super::name_normalizer::normalize_name;

/// Convert a parsed `SchemaOrRef` to an `IrType`.
pub fn schema_or_ref_to_ir_type(schema_or_ref: &SchemaOrRef) -> IrType {
    match schema_or_ref {
        SchemaOrRef::Ref { ref_path } => {
            let name = ref_path.rsplit('/').next().unwrap_or("Unknown");
            IrType::Ref(normalize_name(name).pascal_case)
        }
        SchemaOrRef::Schema(schema) => schema_to_ir_type(schema),
    }
}

/// Convert a parsed `Schema` to an `IrType`.
pub fn schema_to_ir_type(schema: &Schema) -> IrType {
    // Handle composition first
    if !schema.one_of.is_empty() {
        let variants: Vec<IrType> = schema.one_of.iter().map(schema_or_ref_to_ir_type).collect();
        return IrType::Union(variants);
    }
    if !schema.any_of.is_empty() {
        let variants: Vec<IrType> = schema.any_of.iter().map(schema_or_ref_to_ir_type).collect();
        return IrType::Union(variants);
    }
    if !schema.all_of.is_empty() {
        if schema.all_of.len() == 1 {
            return schema_or_ref_to_ir_type(&schema.all_of[0]);
        }
        let parts: Vec<IrType> = schema
            .all_of
            .iter()
            .map(|sub| match sub {
                SchemaOrRef::Ref { .. } => schema_or_ref_to_ir_type(sub),
                SchemaOrRef::Schema(s) => {
                    if s.properties.is_empty() {
                        schema_to_ir_type(s)
                    } else {
                        let fields: Vec<(String, IrType, bool)> = s
                            .properties
                            .iter()
                            .map(|(name, prop)| {
                                (
                                    name.clone(),
                                    schema_or_ref_to_ir_type(prop),
                                    s.required.contains(name),
                                )
                            })
                            .collect();
                        IrType::Object(fields)
                    }
                }
            })
            .collect();
        return IrType::Intersection(parts);
    }

    // Handle enum
    if !schema.enum_values.is_empty() {
        let string_variants: Vec<String> = schema
            .enum_values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if string_variants.len() == 1 {
            return IrType::StringLiteral(string_variants.into_iter().next().unwrap());
        }
        if string_variants.len() > 1 {
            return IrType::Union(
                string_variants
                    .into_iter()
                    .map(IrType::StringLiteral)
                    .collect(),
            );
        }
        return IrType::String; // fallback for non-string enums
    }

    // Handle const
    if let Some(ref val) = schema.const_value {
        if let Some(s) = val.as_str() {
            return IrType::StringLiteral(s.to_string());
        }
        return IrType::String;
    }

    // Handle type
    match &schema.schema_type {
        Some(TypeSet::Single(t)) => match t {
            SchemaType::String => match schema.format.as_deref() {
                Some("date-time" | "date") => IrType::DateTime,
                Some("binary" | "byte") => IrType::Binary,
                _ => IrType::String,
            },
            SchemaType::Number => IrType::Number,
            SchemaType::Integer => IrType::Integer,
            SchemaType::Boolean => IrType::Boolean,
            SchemaType::Null => IrType::Null,
            SchemaType::Array => match &schema.items {
                Some(items) => IrType::Array(Box::new(schema_or_ref_to_ir_type(items))),
                None => IrType::Array(Box::new(IrType::Any)),
            },
            SchemaType::Object => resolve_object_type(schema),
        },
        Some(TypeSet::Multiple(types)) => {
            let non_null: Vec<_> = types.iter().filter(|t| **t != SchemaType::Null).collect();
            let has_null = types.contains(&SchemaType::Null);
            if non_null.len() == 1 {
                let single = Schema {
                    schema_type: Some(TypeSet::Single(non_null[0].clone())),
                    ..schema.clone()
                };
                let base = schema_to_ir_type(&single);
                if has_null {
                    IrType::Union(vec![base, IrType::Null])
                } else {
                    base
                }
            } else if non_null.is_empty() && has_null {
                IrType::Null
            } else {
                // Multiple non-null types — build union of all
                let mut variants: Vec<IrType> = non_null
                    .iter()
                    .map(|t| {
                        let s = Schema {
                            schema_type: Some(TypeSet::Single((*t).clone())),
                            ..schema.clone()
                        };
                        schema_to_ir_type(&s)
                    })
                    .collect();
                if has_null {
                    variants.push(IrType::Null);
                }
                IrType::Union(variants)
            }
        }
        None => {
            // No type specified — check if it has properties (implicit object)
            if !schema.properties.is_empty() {
                resolve_object_type(schema)
            } else if schema.items.is_some() {
                match &schema.items {
                    Some(items) => IrType::Array(Box::new(schema_or_ref_to_ir_type(items))),
                    None => IrType::Array(Box::new(IrType::Any)),
                }
            } else {
                IrType::Any
            }
        }
    }
}

fn resolve_object_type(schema: &Schema) -> IrType {
    if schema.properties.is_empty() {
        match &schema.additional_properties {
            Some(AdditionalProperties::Schema(s)) => {
                IrType::Map(Box::new(schema_or_ref_to_ir_type(s)))
            }
            Some(AdditionalProperties::Bool(true)) => IrType::Map(Box::new(IrType::Any)),
            Some(AdditionalProperties::Bool(false)) | None => IrType::Any,
        }
    } else {
        let fields: Vec<(String, IrType, bool)> = schema
            .properties
            .iter()
            .map(|(name, prop)| {
                let required = schema.required.contains(name);
                (name.clone(), schema_or_ref_to_ir_type(prop), required)
            })
            .collect();
        IrType::Object(fields)
    }
}

/// Convert a named component schema to an `IrSchema`.
pub fn schema_or_ref_to_ir_schema(
    name: &str,
    schema_or_ref: &SchemaOrRef,
) -> Result<IrSchema, TransformError> {
    match schema_or_ref {
        SchemaOrRef::Ref { ref_path } => {
            let target = ref_path.rsplit('/').next().unwrap_or("Unknown");
            Ok(IrSchema::Alias(IrAliasSchema {
                name: normalize_name(name),
                description: None,
                target: IrType::Ref(normalize_name(target).pascal_case),
            }))
        }
        SchemaOrRef::Schema(schema) => schema_to_ir_schema(name, schema),
    }
}

/// Convert a named `Schema` to an `IrSchema`.
pub fn schema_to_ir_schema(name: &str, schema: &Schema) -> Result<IrSchema, TransformError> {
    let normalized = normalize_name(name);

    // Check for enum
    if !schema.enum_values.is_empty() {
        let variants: Vec<String> = schema
            .enum_values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        return Ok(IrSchema::Enum(IrEnumSchema {
            name: normalized,
            description: schema.description.clone(),
            variants,
        }));
    }

    // Check for oneOf / anyOf (union)
    if !schema.one_of.is_empty() || !schema.any_of.is_empty() {
        let variants_src = if !schema.one_of.is_empty() {
            &schema.one_of
        } else {
            &schema.any_of
        };
        let variants: Vec<IrType> = variants_src.iter().map(schema_or_ref_to_ir_type).collect();
        let discriminator = schema.discriminator.as_ref().map(|d| IrDiscriminator {
            property_name: d.property_name.clone(),
            mapping: d
                .mapping
                .iter()
                .map(|(k, v)| {
                    let name = v.rsplit('/').next().unwrap_or(v);
                    (k.clone(), normalize_name(name).pascal_case)
                })
                .collect(),
        });
        return Ok(IrSchema::Union(IrUnionSchema {
            name: normalized,
            description: schema.description.clone(),
            variants,
            discriminator,
        }));
    }

    // Check for allOf
    if !schema.all_of.is_empty() {
        let has_refs = schema
            .all_of
            .iter()
            .any(|s| matches!(s, SchemaOrRef::Ref { .. }));
        if has_refs {
            // Build intersection: refs stay as Ref, inline schemas become Objects
            let mut parts: Vec<IrType> = schema
                .all_of
                .iter()
                .map(|sub| match sub {
                    SchemaOrRef::Ref { .. } => schema_or_ref_to_ir_type(sub),
                    SchemaOrRef::Schema(s) => {
                        let fields = build_fields(&s.properties, &s.required);
                        if fields.is_empty() {
                            schema_to_ir_type(s)
                        } else {
                            let inline_fields: Vec<(String, IrType, bool)> = fields
                                .into_iter()
                                .map(|f| (f.original_name, f.field_type, f.required))
                                .collect();
                            IrType::Object(inline_fields)
                        }
                    }
                })
                .collect();
            // Add extra properties from the parent schema if any
            if !schema.properties.is_empty() {
                let extra_fields = build_fields(&schema.properties, &schema.required);
                let inline_fields: Vec<(String, IrType, bool)> = extra_fields
                    .into_iter()
                    .map(|f| (f.original_name, f.field_type, f.required))
                    .collect();
                parts.push(IrType::Object(inline_fields));
            }
            return Ok(IrSchema::Alias(IrAliasSchema {
                name: normalized,
                description: schema.description.clone(),
                target: IrType::Intersection(parts),
            }));
        }
        // No refs — safe to flatten merge as before
        let merged = merge_all_of(&schema.all_of, &schema.properties, &schema.required);
        return Ok(IrSchema::Object(IrObjectSchema {
            name: normalized,
            description: schema.description.clone(),
            fields: merged,
            additional_properties: None,
        }));
    }

    // Check if it's a simple type alias
    match &schema.schema_type {
        Some(TypeSet::Single(SchemaType::Object)) | None if !schema.properties.is_empty() => {
            // Object with properties
            let fields = build_fields(&schema.properties, &schema.required);
            let additional = schema
                .additional_properties
                .as_ref()
                .and_then(|ap| match ap {
                    AdditionalProperties::Schema(s) => Some(schema_or_ref_to_ir_type(s)),
                    AdditionalProperties::Bool(true) => Some(IrType::Any),
                    _ => None,
                });
            Ok(IrSchema::Object(IrObjectSchema {
                name: normalized,
                description: schema.description.clone(),
                fields,
                additional_properties: additional,
            }))
        }
        _ => {
            // Simple alias (string, number, array, etc.)
            let target = schema_to_ir_type(schema);
            Ok(IrSchema::Alias(IrAliasSchema {
                name: normalized,
                description: schema.description.clone(),
                target,
            }))
        }
    }
}

fn build_fields(properties: &IndexMap<String, SchemaOrRef>, required: &[String]) -> Vec<IrField> {
    properties
        .iter()
        .map(|(name, prop)| {
            let (description, read_only, write_only) = match prop {
                SchemaOrRef::Schema(s) => (
                    s.description.clone(),
                    s.read_only.unwrap_or(false),
                    s.write_only.unwrap_or(false),
                ),
                _ => (None, false, false),
            };
            IrField {
                name: normalize_name(name),
                original_name: name.clone(),
                field_type: schema_or_ref_to_ir_type(prop),
                required: required.contains(name),
                description,
                read_only,
                write_only,
            }
        })
        .collect()
}

fn merge_all_of(
    all_of: &[SchemaOrRef],
    extra_properties: &IndexMap<String, SchemaOrRef>,
    extra_required: &[String],
) -> Vec<IrField> {
    let mut fields = Vec::new();

    for item in all_of {
        if let SchemaOrRef::Schema(schema) = item {
            fields.extend(build_fields(&schema.properties, &schema.required));
            // Recursively merge nested allOf
            if !schema.all_of.is_empty() {
                fields.extend(merge_all_of(&schema.all_of, &IndexMap::new(), &[]));
            }
        }
    }

    // Add extra properties from the parent schema
    fields.extend(build_fields(extra_properties, extra_required));

    fields
}
