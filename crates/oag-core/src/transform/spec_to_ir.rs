use std::collections::HashMap;

use indexmap::IndexMap;

use crate::config::NamingStrategy;
use crate::error::TransformError;
use crate::ir::*;
use crate::parse::operation::{Operation, PathItem};
use crate::parse::parameter::{ParameterLocation, ParameterOrRef};
use crate::parse::ref_resolve::RefResolver;
use crate::parse::request_body::RequestBodyOrRef;
use crate::parse::spec::OpenApiSpec;

use super::name_normalizer::{normalize_name, route_to_name};
use super::promote_inline::promote_inline_objects;
use super::schema_resolver::{schema_or_ref_to_ir_schema, schema_or_ref_to_ir_type};
use super::sse_detector::detect_return_type;

/// Options controlling how the transform phase resolves operation names.
#[derive(Debug, Clone, Default)]
pub struct TransformOptions {
    pub naming_strategy: NamingStrategy,
    pub aliases: IndexMap<String, String>,
}

/// Transform a parsed OpenAPI spec into the fully resolved IR.
pub fn transform(spec: &OpenApiSpec) -> Result<IrSpec, TransformError> {
    transform_with_options(spec, &TransformOptions::default())
}

/// Transform with explicit naming options.
pub fn transform_with_options(
    spec: &OpenApiSpec,
    options: &TransformOptions,
) -> Result<IrSpec, TransformError> {
    // Phase 1: Resolve all $ref pointers
    let mut resolver = RefResolver::new(spec);
    let resolved = resolver.resolve_spec(spec)?;

    // Phase 2: Convert component schemas to IR schemas
    let schemas = resolve_schemas(&resolved)?;

    // Phase 3: Convert operations
    let operations = resolve_operations(&resolved, options)?;

    // Phase 4: Group operations into modules by tag
    let modules = group_into_modules(&operations);

    // Phase 5: Build IR info and servers
    let info = IrInfo {
        title: resolved.info.title.clone(),
        description: resolved.info.description.clone(),
        version: resolved.info.version.clone(),
    };

    let servers = resolved
        .servers
        .iter()
        .map(|s| IrServer {
            url: s.url.clone(),
            description: s.description.clone(),
        })
        .collect();

    let mut ir = IrSpec {
        info,
        servers,
        schemas,
        operations,
        modules,
    };

    // Phase 6: Promote inline objects to named schemas
    promote_inline_objects(&mut ir);

    Ok(ir)
}

fn resolve_schemas(spec: &OpenApiSpec) -> Result<Vec<IrSchema>, TransformError> {
    let mut schemas = Vec::new();
    if let Some(ref components) = spec.components {
        for (name, schema_or_ref) in &components.schemas {
            let ir_schema = schema_or_ref_to_ir_schema(name, schema_or_ref)?;
            schemas.push(ir_schema);
        }
    }
    Ok(schemas)
}

fn resolve_operations(
    spec: &OpenApiSpec,
    options: &TransformOptions,
) -> Result<Vec<IrOperation>, TransformError> {
    let mut operations = Vec::new();

    for (path, path_item) in &spec.paths {
        let path_params = resolve_parameters(&path_item.parameters);
        collect_operations(path, path_item, &path_params, options, &mut operations)?;
    }

    Ok(operations)
}

fn collect_operations(
    path: &str,
    item: &PathItem,
    path_params: &[IrParameter],
    options: &TransformOptions,
    out: &mut Vec<IrOperation>,
) -> Result<(), TransformError> {
    macro_rules! add_op {
        ($method:expr, $op:expr) => {
            if let Some(ref op) = $op {
                let ir_op = build_operation($method, path, op, path_params, options)?;
                out.push(ir_op);
            }
        };
    }

    add_op!(HttpMethod::Get, item.get);
    add_op!(HttpMethod::Post, item.post);
    add_op!(HttpMethod::Put, item.put);
    add_op!(HttpMethod::Delete, item.delete);
    add_op!(HttpMethod::Patch, item.patch);
    add_op!(HttpMethod::Options, item.options);
    add_op!(HttpMethod::Head, item.head);
    add_op!(HttpMethod::Trace, item.trace);

    Ok(())
}

fn build_operation(
    method: HttpMethod,
    path: &str,
    op: &Operation,
    path_params: &[IrParameter],
    options: &TransformOptions,
) -> Result<IrOperation, TransformError> {
    // Derive the raw operation name based on naming strategy
    let raw_name = match options.naming_strategy {
        NamingStrategy::UseOperationId => {
            op.operation_id.clone().unwrap_or_else(|| {
                // Fallback: route-based even in operationId mode when no operationId
                route_to_name(method.as_str(), path)
            })
        }
        NamingStrategy::UseRouteBased => route_to_name(method.as_str(), path),
    };

    // Apply aliases: if the raw name matches an alias key, use the alias value
    let name = if let Some(alias) = options.aliases.get(&raw_name) {
        alias.clone()
    } else {
        raw_name.clone()
    };

    let mut parameters = path_params.to_vec();
    parameters.extend(resolve_parameters(&op.parameters));

    let request_body = op.request_body.as_ref().and_then(resolve_request_body);

    let return_type = detect_return_type(&name, &op.responses);

    Ok(IrOperation {
        name: normalize_name(&name),
        method,
        path: path.to_string(),
        summary: op.summary.clone(),
        description: op.description.clone(),
        tags: op.tags.clone(),
        parameters,
        request_body,
        return_type,
        deprecated: op.deprecated.unwrap_or(false),
    })
}

fn resolve_parameters(params: &[ParameterOrRef]) -> Vec<IrParameter> {
    params
        .iter()
        .filter_map(|p| match p {
            ParameterOrRef::Parameter(param) => {
                let location = match param.location {
                    ParameterLocation::Path => IrParameterLocation::Path,
                    ParameterLocation::Query => IrParameterLocation::Query,
                    ParameterLocation::Header => IrParameterLocation::Header,
                    ParameterLocation::Cookie => IrParameterLocation::Cookie,
                };
                let param_type = param
                    .schema
                    .as_ref()
                    .map(schema_or_ref_to_ir_type)
                    .unwrap_or(IrType::Any);
                Some(IrParameter {
                    name: normalize_name(&param.name),
                    original_name: param.name.clone(),
                    location,
                    param_type,
                    required: param.required,
                    description: param.description.clone(),
                })
            }
            ParameterOrRef::Ref { .. } => None, // Should already be resolved
        })
        .collect()
}

fn resolve_request_body(body: &RequestBodyOrRef) -> Option<IrRequestBody> {
    match body {
        RequestBodyOrRef::RequestBody(rb) => {
            // Prefer application/json, fall back to first content type
            let (content_type, mt) = rb
                .content
                .get_key_value("application/json")
                .or_else(|| rb.content.first())?;

            let body_type = mt
                .schema
                .as_ref()
                .map(schema_or_ref_to_ir_type)
                .unwrap_or(IrType::Any);

            let encoding = if mt.encoding.is_empty() {
                None
            } else {
                Some(
                    mt.encoding
                        .iter()
                        .map(|(field_name, enc)| IrFieldEncoding {
                            field_name: field_name.clone(),
                            content_type: enc.content_type.clone(),
                        })
                        .collect(),
                )
            };

            Some(IrRequestBody {
                body_type,
                required: rb.required,
                content_type: content_type.clone(),
                description: rb.description.clone(),
                encoding,
            })
        }
        RequestBodyOrRef::Ref { .. } => None, // Should already be resolved
    }
}

fn group_into_modules(operations: &[IrOperation]) -> Vec<IrModule> {
    let mut tag_groups: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, op) in operations.iter().enumerate() {
        if op.tags.is_empty() {
            tag_groups.entry("default".to_string()).or_default().push(i);
        } else {
            for tag in &op.tags {
                tag_groups.entry(tag.clone()).or_default().push(i);
            }
        }
    }

    let mut modules: Vec<IrModule> = tag_groups
        .into_iter()
        .map(|(name, ops)| IrModule {
            name: normalize_name(&name),
            operations: ops,
        })
        .collect();

    modules.sort_by(|a, b| a.name.original.cmp(&b.name.original));
    modules
}
