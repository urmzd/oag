use std::collections::HashSet;

use minijinja::{Value, context};

use crate::ir::{
    HttpMethod, IrEnumVariant, IrObjectSchema, IrOperation, IrParameterLocation, IrReturnType,
    IrSchema, IrSpec, IrType,
};

use super::pack::TypeMapConfig;
use super::type_map::{map_field_type, map_type};

struct ParamsResult {
    path_params: Vec<Value>,
    query_params: Vec<Value>,
    py_params: Vec<Value>,
    header_params_obj: String,
    has_body: bool,
    body_content_type: String,
    has_path_params: bool,
    has_query_params: bool,
    has_header_params: bool,
}

/// Build the universal template context for an IrSpec.
///
/// Returns a minijinja Value containing all data needed by any template pack.
pub fn build_context(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> Value {
    let schemas = build_schema_contexts(ir, tm, field_casing);
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
    let sse_event_types = collect_sse_event_types(ir, &schema_names, tm);
    let guards = build_guard_contexts(ir);

    // Build operations with deduplication
    let (operations, used_op_indices) =
        build_operation_contexts(ir, tm, field_casing, operation_casing);

    // Build hooks (React-specific, but included universally — templates ignore if unused)
    let (hooks, hook_op_indices) = build_hook_contexts(ir, tm, operation_casing);

    // Collect imported types from surviving operations
    let imported_types = collect_imported_types(
        ir.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| used_op_indices.contains(i))
            .map(|(_, op)| op),
        tm,
    );

    let hook_imported_types = collect_imported_types(
        ir.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| hook_op_indices.contains(i))
            .map(|(_, op)| op),
        tm,
    );

    // Model imports for Python-style generators (Ref names from operations)
    let model_imports = collect_model_imports(ir);

    let has_sse = operations.iter().any(|op| {
        op.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });
    let has_queries = hooks.iter().any(|h| {
        h.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("query"))
    });
    let has_mutations = hooks.iter().any(|h| {
        h.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("mutation"))
    });
    let has_hook_sse = hooks.iter().any(|h| {
        h.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });

    // Test contexts
    let (test_operations, test_op_indices) =
        build_test_operation_contexts(ir, tm, operation_casing);
    let test_type_imports = collect_test_type_imports(
        ir.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| test_op_indices.contains(i))
            .map(|(_, op)| op),
    );
    let (py_test_operations, py_model_imports) = build_python_test_contexts(ir);
    let hook_names = build_hook_names(ir);

    // Extract guard sub-fields for flat template access
    let guard_imports = guards
        .get_attr("imports")
        .unwrap_or(Value::from(Vec::<String>::new()));
    let guard_items = guards
        .get_attr("guards")
        .unwrap_or(Value::from(Vec::<Value>::new()));

    context! {
        title => ir.info.title.clone(),
        schemas => schemas,
        sse_event_types => sse_event_types,
        // guards.ts.j2 expects top-level `imports` and `guards`
        imports => guard_imports,
        guards => guard_items,
        operations => operations,
        hooks => hooks,
        imported_types => imported_types,
        hook_imported_types => hook_imported_types,
        model_imports => model_imports,
        has_sse => has_sse,
        has_queries => has_queries,
        has_mutations => has_mutations,
        has_hook_sse => has_hook_sse,
        // Test contexts
        test_operations => test_operations,
        test_type_imports => test_type_imports,
        py_test_operations => py_test_operations,
        py_model_imports => py_model_imports,
        hook_names => hook_names,
    }
}

// ─── Schema contexts ─────────────────────────────────────────────

fn build_schema_contexts(ir: &IrSpec, tm: &TypeMapConfig, field_casing: &str) -> Vec<Value> {
    ir.schemas
        .iter()
        .map(|s| schema_to_ctx(s, tm, field_casing))
        .collect()
}

fn schema_to_ctx(schema: &IrSchema, tm: &TypeMapConfig, field_casing: &str) -> Value {
    match schema {
        IrSchema::Object(obj) => object_to_ctx(obj, tm, field_casing),
        IrSchema::Enum(e) => {
            let is_integer = !e.variants.is_empty()
                && e.variants
                    .iter()
                    .all(|v| matches!(v, IrEnumVariant::Integer(_)));

            // TS-style: literal string values
            let ts_variants: Vec<String> = e
                .variants
                .iter()
                .map(|v| match v {
                    IrEnumVariant::String(s) => format!("\"{s}\""),
                    IrEnumVariant::Integer(i) => i.to_string(),
                })
                .collect();

            // Python-style: name/value pairs
            let py_variants: Vec<Value> = e
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
                variants => ts_variants,
                py_variants => py_variants,
                is_integer => is_integer,
            }
        }
        IrSchema::Alias(a) => {
            context! {
                kind => "alias",
                name => a.name.pascal_case.clone(),
                description => a.description.clone(),
                target => map_type(&a.target, tm),
            }
        }
        IrSchema::Union(u) => {
            let variants: Vec<String> = u.variants.iter().map(|v| map_type(v, tm)).collect();
            context! {
                kind => "union",
                name => u.name.pascal_case.clone(),
                description => u.description.clone(),
                variants => variants,
            }
        }
    }
}

fn object_to_ctx(obj: &IrObjectSchema, tm: &TypeMapConfig, field_casing: &str) -> Value {
    let fields: Vec<Value> = obj
        .fields
        .iter()
        .map(|f| {
            let field_name = match field_casing {
                "snake" => f.name.snake_case.clone(),
                "pascal" => f.name.pascal_case.clone(),
                _ => f.name.camel_case.clone(),
            };
            let type_str = map_type(&f.field_type, tm);
            let field_type_str = map_field_type(&f.field_type, f.required, tm);
            context! {
                name => field_name,
                original_name => f.original_name.clone(),
                type_str => type_str,
                field_type_str => field_type_str,
                required => f.required,
                description => f.description.clone(),
                needs_alias => f.name.snake_case != f.original_name,
            }
        })
        .collect();

    let additional = obj.additional_properties.as_ref().map(|t| map_type(t, tm));
    let has_additional_properties = obj.additional_properties.is_some();

    context! {
        kind => "object",
        name => obj.name.pascal_case.clone(),
        description => obj.description.clone(),
        fields => fields,
        additional_properties => additional,
        has_additional_properties => has_additional_properties,
    }
}

fn collect_sse_event_types(
    ir: &IrSpec,
    schema_names: &HashSet<String>,
    tm: &TypeMapConfig,
) -> Vec<Value> {
    let mut event_types = Vec::new();
    let mut seen = HashSet::new();
    for op in &ir.operations {
        if let IrReturnType::Sse(sse) = &op.return_type
            && let Some(ref event_name) = sse.event_type_name
        {
            if seen.contains(event_name) || schema_names.contains(event_name) {
                continue;
            }
            let variants: Vec<String> = sse.variants.iter().map(|v| map_type(v, tm)).collect();
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

// ─── Guard contexts ──────────────────────────────────────────────

fn build_guard_contexts(ir: &IrSpec) -> Value {
    let mut guards = Vec::new();
    let mut import_names: Vec<String> = Vec::new();

    for schema in &ir.schemas {
        if let IrSchema::Union(u) = schema
            && let Some(disc) = &u.discriminator
        {
            let union_name = u.name.pascal_case.clone();
            if !import_names.contains(&union_name) {
                import_names.push(union_name.clone());
            }

            for (disc_value, schema_name) in &disc.mapping {
                if u.variants
                    .iter()
                    .any(|v| matches!(v, IrType::Ref(n) if n == schema_name))
                {
                    if !import_names.contains(schema_name) {
                        import_names.push(schema_name.clone());
                    }
                    let is_integer_discriminator = disc_value.parse::<i64>().is_ok();
                    guards.push(context! {
                        union_name => union_name.clone(),
                        variant_name => schema_name.clone(),
                        property_name => disc.property_name.clone(),
                        discriminator_value => disc_value.clone(),
                        is_integer_discriminator => is_integer_discriminator,
                    });
                }
            }
        }
    }

    context! {
        imports => import_names,
        guards => guards,
    }
}

// ─── Operation contexts ──────────────────────────────────────────

fn op_name(op: &IrOperation, casing: &str) -> String {
    match casing {
        "snake" => op.name.snake_case.clone(),
        "pascal" => op.name.pascal_case.clone(),
        _ => op.name.camel_case.clone(),
    }
}

fn build_operation_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_methods = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_operation_contexts(op, tm, field_casing, operation_casing)
                .into_iter()
                .map(move |ctx| (idx, ctx))
        })
        .filter(|(idx, ctx)| {
            let name = ctx
                .get_attr("method_name")
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            match name {
                Some(n) => {
                    if seen_methods.insert(n) {
                        used_op_indices.insert(*idx);
                        true
                    } else {
                        false
                    }
                }
                None => true,
            }
        })
        .map(|(_, ctx)| ctx)
        .collect();

    (operations, used_op_indices)
}

fn classify_param_type(ir_type: &IrType) -> &'static str {
    match ir_type {
        IrType::Array(_) => "array",
        IrType::Map(_) => "map",
        IrType::Object(_) | IrType::Ref(_) => "object",
        _ => "primitive",
    }
}

/// Build structured parameter data for an operation.
fn build_params(op: &IrOperation, tm: &TypeMapConfig, field_casing: &str) -> ParamsResult {
    let mut required_parts = Vec::new();
    let mut optional_parts = Vec::new();
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_parts = Vec::new();

    let param_name = |op_param: &crate::ir::IrParameter| -> String {
        match field_casing {
            "snake" => op_param.name.snake_case.clone(),
            "pascal" => op_param.name.pascal_case.clone(),
            _ => op_param.name.camel_case.clone(),
        }
    };

    for param in &op.parameters {
        let type_str = map_type(&param.param_type, tm);
        let name = param_name(param);
        match param.location {
            IrParameterLocation::Path => {
                required_parts.push(format!("{name}: {type_str}"));
                path_params.push(context! {
                    name => name,
                    original_name => param.original_name.clone(),
                });
            }
            IrParameterLocation::Query => {
                if param.required {
                    required_parts.push(format!("{name}: {type_str}"));
                } else {
                    optional_parts.push(format!("{name}?: {type_str}"));
                }
                let type_kind = classify_param_type(&param.param_type);
                let style = param.style.clone().unwrap_or_else(|| "form".to_string());
                let explode = param.explode.unwrap_or(style == "form");
                query_params.push(context! {
                    name => name,
                    original_name => param.original_name.clone(),
                    style => style,
                    explode => explode,
                    type_kind => type_kind,
                });
            }
            IrParameterLocation::Header => {
                if param.required {
                    required_parts.push(format!("{name}: {type_str}"));
                } else {
                    optional_parts.push(format!("{name}?: {type_str}"));
                }
                header_parts.push(format!("\"{}\": {}", param.original_name, name));
            }
            _ => {}
        }
    }

    let has_body = op.request_body.is_some();
    let body_content_type = op
        .request_body
        .as_ref()
        .map(|b| b.content_type.clone())
        .unwrap_or_else(|| "application/json".to_string());

    if let Some(ref body) = op.request_body {
        let type_str = map_type(&body.body_type, tm);
        if body.required {
            required_parts.push(format!("body: {type_str}"));
        } else {
            optional_parts.push(format!("body?: {type_str}"));
        }
    }

    optional_parts.push("options?: RequestOptions".to_string());

    let mut parts = required_parts;
    parts.extend(optional_parts);

    let has_path_params = !path_params.is_empty();
    let has_query_params = !query_params.is_empty();
    let has_header_params = !header_parts.is_empty();
    let header_params_obj = header_parts.join(", ");
    let _params_signature = parts.join(", ");

    // Build Python-style params (structured, not pre-formatted)
    let py_params: Vec<Value> = op
        .parameters
        .iter()
        .map(|param| {
            let type_str = map_type(&param.param_type, tm);
            let location = match param.location {
                IrParameterLocation::Path => "path",
                IrParameterLocation::Query => "query",
                IrParameterLocation::Header => "header",
                IrParameterLocation::Cookie => "cookie",
            };
            let name = param_name(param);
            context! {
                name => name,
                original_name => param.original_name.clone(),
                type_str => type_str,
                location => location,
                required => param.required,
                needs_alias => param.name.snake_case != param.original_name,
            }
        })
        .collect();

    ParamsResult {
        path_params,
        query_params,
        py_params,
        header_params_obj,
        has_body,
        body_content_type,
        has_path_params,
        has_query_params,
        has_header_params,
    }
}

fn is_multipart_op(op: &IrOperation) -> bool {
    op.request_body
        .as_ref()
        .is_some_and(|b| b.content_type == "multipart/form-data")
}

fn build_single_operation_contexts(
    op: &IrOperation,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> Vec<Value> {
    let mut results = Vec::new();
    let method_name = op_name(op, operation_casing);
    let http_method = op.method.as_str();
    let path = op.path.clone();

    // Build TS-style params
    let ParamsResult {
        path_params,
        query_params,
        py_params,
        header_params_obj,
        has_body,
        body_content_type,
        has_path_params,
        has_query_params,
        has_header_params,
    } = build_params(op, tm, field_casing);

    // Build params_signature (TS-style pre-formatted)
    let params_signature = build_ts_params_signature(op, tm, field_casing, false);

    let body_type = op
        .request_body
        .as_ref()
        .map(|b| map_type(&b.body_type, tm))
        .unwrap_or_default();
    let body_param_name = "body".to_string();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = map_type(&resp.response_type, tm);
            results.push(context! {
                kind => "standard",
                method_name => method_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => params_signature,
                return_type => return_type,
                path_params => path_params,
                query_params => query_params,
                params => py_params,
                header_params_obj => header_params_obj,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                body_content_type => body_content_type,
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });
        }
        IrReturnType::Void => {
            results.push(context! {
                kind => "void",
                method_name => method_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => params_signature,
                return_type => map_type(&IrType::Void, tm),
                path_params => path_params,
                query_params => query_params,
                params => py_params,
                header_params_obj => header_params_obj,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                body_content_type => body_content_type,
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });
        }
        IrReturnType::Sse(sse) => {
            let return_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                map_type(&sse.event_type, tm)
            };
            let sse_name = if sse.also_has_json {
                format!("{}Stream", op.name.camel_case)
            } else {
                op.name.camel_case.clone()
            };

            // SSE params use SSEOptions instead of RequestOptions
            let sse_params_sig = build_ts_params_signature(op, tm, field_casing, true);

            results.push(context! {
                kind => "sse",
                method_name => sse_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => sse_params_sig,
                return_type => return_type,
                event_type => return_type,
                path_params => path_params,
                query_params => query_params.clone(),
                params => py_params.clone(),
                header_params_obj => header_params_obj.clone(),
                has_body => has_body,
                body_type => body_type.clone(),
                body_param_name => body_param_name.clone(),
                body_content_type => body_content_type.clone(),
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });

            if let Some(ref json_resp) = sse.json_response {
                let json_return_type = map_type(&json_resp.response_type, tm);
                let json_desc = format!(
                    "{} (JSON response)",
                    op.description.as_deref().unwrap_or("")
                );
                results.push(context! {
                    kind => "standard",
                    method_name => method_name,
                    name => op.name.snake_case.clone(),
                    http_method => http_method,
                    path => path,
                    params_signature => params_signature,
                    return_type => json_return_type,
                    path_params => path_params.clone(),
                    query_params => query_params,
                    params => py_params,
                    header_params_obj => header_params_obj,
                    has_body => has_body,
                    body_type => body_type,
                    body_param_name => body_param_name,
                    body_content_type => body_content_type,
                    is_multipart => is_multipart_op(op),
                    has_path_params => has_path_params,
                    has_query_params => has_query_params,
                    has_header_params => has_header_params,
                    summary => op.summary.clone(),
                    description => json_desc,
                    deprecated => op.deprecated,
                });
            }
        }
    }

    results
}

/// Build a TS-style params signature string.
fn build_ts_params_signature(
    op: &IrOperation,
    tm: &TypeMapConfig,
    field_casing: &str,
    is_sse: bool,
) -> String {
    let param_name = |p: &crate::ir::IrParameter| -> String {
        match field_casing {
            "snake" => p.name.snake_case.clone(),
            "pascal" => p.name.pascal_case.clone(),
            _ => p.name.camel_case.clone(),
        }
    };

    let mut required_parts = Vec::new();
    let mut optional_parts = Vec::new();

    for param in &op.parameters {
        let type_str = map_type(&param.param_type, tm);
        let name = param_name(param);
        match param.location {
            IrParameterLocation::Path => {
                required_parts.push(format!("{name}: {type_str}"));
            }
            IrParameterLocation::Query => {
                if param.required {
                    required_parts.push(format!("{name}: {type_str}"));
                } else {
                    optional_parts.push(format!("{name}?: {type_str}"));
                }
            }
            IrParameterLocation::Header => {
                if param.required {
                    required_parts.push(format!("{name}: {type_str}"));
                } else {
                    optional_parts.push(format!("{name}?: {type_str}"));
                }
            }
            _ => {}
        }
    }

    if let Some(ref body) = op.request_body {
        let type_str = map_type(&body.body_type, tm);
        if body.required {
            required_parts.push(format!("body: {type_str}"));
        } else {
            optional_parts.push(format!("body?: {type_str}"));
        }
    }

    let options_type = if is_sse {
        "options?: SSEOptions"
    } else {
        "options?: RequestOptions"
    };
    optional_parts.push(options_type.to_string());

    let mut parts = required_parts;
    parts.extend(optional_parts);
    parts.join(", ")
}

// ─── Hook contexts (React) ───────────────────────────────────────

fn build_hook_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    _operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_hooks = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let hooks: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_hook_contexts(op, tm)
                .into_iter()
                .map(move |ctx| (idx, ctx))
        })
        .filter(|(idx, h)| {
            let name = h
                .get_attr("hook_name")
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            match name {
                Some(n) => {
                    if seen_hooks.insert(n) {
                        used_op_indices.insert(*idx);
                        true
                    } else {
                        false
                    }
                }
                None => true,
            }
        })
        .map(|(_, ctx)| ctx)
        .collect();

    (hooks, used_op_indices)
}

fn build_single_hook_contexts(op: &IrOperation, tm: &TypeMapConfig) -> Vec<Value> {
    let mut results = Vec::new();

    match (&op.method, &op.return_type) {
        // GET → useSWR query hook
        (HttpMethod::Get, IrReturnType::Standard(resp)) => {
            let return_type = map_type(&resp.response_type, tm);
            let (params_sig, swr_key, call_args) = build_hook_query_params(op, tm);
            results.push(context! {
                kind => "query",
                hook_name => format!("use{}", op.name.pascal_case),
                method_name => op.name.camel_case.clone(),
                params_signature => params_sig,
                return_type => return_type,
                swr_key => swr_key,
                call_args => call_args,
                description => op.summary.clone().or(op.description.clone()),
            });
        }
        // POST/PUT/DELETE non-streaming → useSWRMutation
        (_, IrReturnType::Standard(_)) | (_, IrReturnType::Void) => {
            let return_type = match &op.return_type {
                IrReturnType::Standard(r) => map_type(&r.response_type, tm),
                _ => map_type(&IrType::Void, tm),
            };
            let has_body = op.request_body.is_some();
            let body_type = op
                .request_body
                .as_ref()
                .map(|b| map_type(&b.body_type, tm))
                .unwrap_or_else(|| map_type(&IrType::Void, tm));

            let (path_params_sig, swr_key, call_args, swr_key_type) =
                build_hook_mutation_params(op, tm);
            results.push(context! {
                kind => "mutation",
                hook_name => format!("use{}", op.name.pascal_case),
                method_name => op.name.camel_case.clone(),
                path_params_signature => path_params_sig,
                return_type => return_type,
                has_body => has_body,
                body_type => body_type,
                swr_key => swr_key,
                swr_key_type => swr_key_type,
                call_args => call_args,
                description => op.summary.clone().or(op.description.clone()),
            });
        }
        // SSE → custom streaming hook
        (_, IrReturnType::Sse(sse)) => {
            let event_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                map_type(&sse.event_type, tm)
            };
            let event_type_array = if event_type.contains('|') {
                format!("({event_type})[]")
            } else {
                format!("{event_type}[]")
            };
            let method_name = if sse.also_has_json {
                format!("{}Stream", op.name.camel_case)
            } else {
                op.name.camel_case.clone()
            };
            let hook_name = if sse.also_has_json {
                format!("use{}Stream", op.name.pascal_case)
            } else {
                format!("use{}", op.name.pascal_case)
            };
            let (path_params_sig, trigger_params, stream_call_args, deps) =
                build_hook_sse_params(op, tm);

            results.push(context! {
                kind => "sse",
                hook_name => hook_name,
                method_name => method_name,
                path_params_signature => path_params_sig,
                event_type => event_type,
                event_type_array => event_type_array,
                trigger_params => trigger_params,
                stream_call_args => stream_call_args,
                deps => deps,
                description => op.summary.clone().or(op.description.clone()),
            });

            // Dual endpoint: also generate JSON hook
            if let Some(ref json_resp) = sse.json_response {
                let return_type = map_type(&json_resp.response_type, tm);
                match op.method {
                    HttpMethod::Get => {
                        let (params_sig, swr_key, call_args) = build_hook_query_params(op, tm);
                        results.push(context! {
                            kind => "query",
                            hook_name => format!("use{}", op.name.pascal_case),
                            method_name => op.name.camel_case.clone(),
                            params_signature => params_sig,
                            return_type => return_type,
                            swr_key => swr_key,
                            call_args => call_args,
                            description => op.summary.clone().or(op.description.clone()),
                        });
                    }
                    _ => {
                        let has_body = op.request_body.is_some();
                        let body_type = op
                            .request_body
                            .as_ref()
                            .map(|b| map_type(&b.body_type, tm))
                            .unwrap_or_else(|| map_type(&IrType::Void, tm));
                        let (path_params_sig, swr_key, call_args, swr_key_type) =
                            build_hook_mutation_params(op, tm);
                        results.push(context! {
                            kind => "mutation",
                            hook_name => format!("use{}", op.name.pascal_case),
                            method_name => op.name.camel_case.clone(),
                            path_params_signature => path_params_sig,
                            return_type => return_type,
                            has_body => has_body,
                            body_type => body_type,
                            swr_key => swr_key,
                            swr_key_type => swr_key_type,
                            call_args => call_args,
                            description => op.summary.clone().or(op.description.clone()),
                        });
                    }
                }
            }
        }
    }

    results
}

fn build_hook_query_params(op: &IrOperation, tm: &TypeMapConfig) -> (String, String, String) {
    let mut required_sig = Vec::new();
    let mut optional_sig = Vec::new();
    let mut required_call = Vec::new();
    let mut optional_call = Vec::new();
    let mut key_parts = Vec::new();

    for param in &op.parameters {
        match param.location {
            IrParameterLocation::Path
            | IrParameterLocation::Query
            | IrParameterLocation::Header => {
                let ts = map_type(&param.param_type, tm);
                let is_required = param.required || param.location == IrParameterLocation::Path;
                if is_required {
                    required_sig.push(format!("{}: {}", param.name.camel_case, ts));
                    required_call.push(param.name.camel_case.clone());
                } else {
                    optional_sig.push(format!("{}?: {}", param.name.camel_case, ts));
                    optional_call.push(param.name.camel_case.clone());
                }
                key_parts.push(param.name.camel_case.clone());
            }
            _ => {}
        }
    }

    let mut sig_parts = required_sig;
    sig_parts.extend(optional_sig);
    let mut call_parts = required_call;
    call_parts.extend(optional_call);

    let swr_key = if key_parts.is_empty() {
        format!("\"{}\"", op.path)
    } else {
        format!("[\"{}\", {}] as const", op.path, key_parts.join(", "))
    };

    (sig_parts.join(", "), swr_key, call_parts.join(", "))
}

fn build_hook_mutation_params(
    op: &IrOperation,
    tm: &TypeMapConfig,
) -> (String, String, String, String) {
    let mut required_sig = Vec::new();
    let mut optional_sig = Vec::new();
    let mut required_call = Vec::new();
    let mut optional_call = Vec::new();
    let mut key_parts = Vec::new();
    let mut key_type_parts = Vec::new();

    for param in &op.parameters {
        match param.location {
            IrParameterLocation::Path
            | IrParameterLocation::Query
            | IrParameterLocation::Header => {
                let ts = map_type(&param.param_type, tm);
                let is_required = param.required || param.location == IrParameterLocation::Path;
                if is_required {
                    required_sig.push(format!("{}: {}", param.name.camel_case, ts));
                    required_call.push(param.name.camel_case.clone());
                } else {
                    optional_sig.push(format!("{}?: {}", param.name.camel_case, ts));
                    optional_call.push(param.name.camel_case.clone());
                }
                key_parts.push(param.name.camel_case.clone());
                key_type_parts.push(ts);
            }
            _ => {}
        }
    }

    let mut sig_parts = required_sig;
    sig_parts.extend(optional_sig);
    let mut call_parts = required_call;
    call_parts.extend(optional_call);

    if op.request_body.is_some() {
        call_parts.push("arg".to_string());
    }

    let swr_key = if key_parts.is_empty() {
        format!("\"{}\"", op.path)
    } else {
        format!("[\"{}\", {}] as const", op.path, key_parts.join(", "))
    };
    let swr_key_type = if key_type_parts.is_empty() {
        "string".to_string()
    } else {
        format!("readonly [string, {}]", key_type_parts.join(", "))
    };

    (
        sig_parts.join(", "),
        swr_key,
        call_parts.join(", "),
        swr_key_type,
    )
}

fn build_hook_sse_params(op: &IrOperation, tm: &TypeMapConfig) -> (String, String, String, String) {
    let mut required_sig = Vec::new();
    let mut optional_sig = Vec::new();
    let mut required_call = Vec::new();
    let mut optional_call = Vec::new();
    let mut deps_parts = Vec::new();

    for param in &op.parameters {
        match param.location {
            IrParameterLocation::Path
            | IrParameterLocation::Query
            | IrParameterLocation::Header => {
                let ts = map_type(&param.param_type, tm);
                let is_required = param.required || param.location == IrParameterLocation::Path;
                if is_required {
                    required_sig.push(format!("{}: {}", param.name.camel_case, ts));
                    required_call.push(param.name.camel_case.clone());
                } else {
                    optional_sig.push(format!("{}?: {}", param.name.camel_case, ts));
                    optional_call.push(param.name.camel_case.clone());
                }
                deps_parts.push(format!(", {}", param.name.camel_case));
            }
            _ => {}
        }
    }

    let mut sig_parts = required_sig;
    sig_parts.extend(optional_sig);
    let mut stream_call_parts = required_call;
    stream_call_parts.extend(optional_call);

    let trigger_params = if let Some(ref body) = op.request_body {
        let ts = map_type(&body.body_type, tm);
        stream_call_parts.push("body".to_string());
        if body.required {
            format!("body: {}", ts)
        } else {
            format!("body?: {}", ts)
        }
    } else {
        String::new()
    };

    (
        sig_parts.join(", "),
        trigger_params,
        stream_call_parts.join(", "),
        deps_parts.join(""),
    )
}

fn build_hook_names(ir: &IrSpec) -> Vec<String> {
    let mut seen = HashSet::new();
    ir.operations
        .iter()
        .flat_map(|op| {
            let mut names = Vec::new();
            match &op.return_type {
                IrReturnType::Sse(sse) => {
                    if sse.also_has_json {
                        names.push(format!("use{}Stream", op.name.pascal_case));
                        names.push(format!("use{}", op.name.pascal_case));
                    } else {
                        names.push(format!("use{}", op.name.pascal_case));
                    }
                }
                _ => {
                    names.push(format!("use{}", op.name.pascal_case));
                }
            }
            names
        })
        .filter(|n| seen.insert(n.clone()))
        .collect()
}

// ─── Import collection ───────────────────────────────────────────

fn collect_imported_types<'a>(
    ops: impl Iterator<Item = &'a IrOperation>,
    _tm: &TypeMapConfig,
) -> Vec<String> {
    let mut types = HashSet::new();

    for op in ops {
        collect_types_from_return(&op.return_type, &mut types);
        if let Some(ref body) = op.request_body {
            collect_refs_from_ir_type(&body.body_type, &mut types);
        }
        for param in &op.parameters {
            collect_refs_from_ir_type(&param.param_type, &mut types);
        }
    }

    let mut sorted: Vec<String> = types.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_types_from_return(ret: &IrReturnType, types: &mut HashSet<String>) {
    match ret {
        IrReturnType::Standard(resp) => {
            collect_refs_from_ir_type(&resp.response_type, types);
        }
        IrReturnType::Sse(sse) => {
            if let Some(ref name) = sse.event_type_name {
                types.insert(name.clone());
            } else {
                collect_refs_from_ir_type(&sse.event_type, types);
            }
            if let Some(ref json) = sse.json_response {
                collect_refs_from_ir_type(&json.response_type, types);
            }
        }
        IrReturnType::Void => {}
    }
}

fn collect_refs_from_ir_type(ir_type: &IrType, types: &mut HashSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            types.insert(name.clone());
        }
        IrType::Array(inner) | IrType::Map(inner) => collect_refs_from_ir_type(inner, types),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_refs_from_ir_type(v, types);
            }
        }
        IrType::Object(fields) => {
            for (_, ty, _) in fields {
                collect_refs_from_ir_type(ty, types);
            }
        }
        _ => {}
    }
}

fn collect_model_imports(ir: &IrSpec) -> Vec<String> {
    let mut imports = HashSet::new();

    for op in &ir.operations {
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_refs_from_ir_type(&resp.response_type, &mut imports);
            }
            IrReturnType::Sse(sse) => {
                if let Some(ref name) = sse.event_type_name {
                    imports.insert(name.clone());
                } else {
                    collect_refs_from_ir_type(&sse.event_type, &mut imports);
                }
                if let Some(ref json) = sse.json_response {
                    collect_refs_from_ir_type(&json.response_type, &mut imports);
                }
            }
            IrReturnType::Void => {}
        }
        if let Some(ref body) = op.request_body {
            collect_refs_from_ir_type(&body.body_type, &mut imports);
        }
        for param in &op.parameters {
            collect_refs_from_ir_type(&param.param_type, &mut imports);
        }
    }

    let mut sorted: Vec<String> = imports.into_iter().collect();
    sorted.sort();
    sorted
}

// ─── Test contexts ───────────────────────────────────────────────

fn build_test_operation_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    _operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_methods = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_test_contexts(op, tm)
                .into_iter()
                .map(move |ctx| (idx, ctx))
        })
        .filter(|(idx, op)| {
            let name = op
                .get_attr("method_name")
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            match name {
                Some(n) => {
                    if seen_methods.insert(n) {
                        used_op_indices.insert(*idx);
                        true
                    } else {
                        false
                    }
                }
                None => true,
            }
        })
        .map(|(_, ctx)| ctx)
        .collect();

    (operations, used_op_indices)
}

fn build_single_test_contexts(op: &IrOperation, tm: &TypeMapConfig) -> Vec<Value> {
    let mut results = Vec::new();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = map_type(&resp.response_type, tm);
            results.push(build_ts_test_context(
                op,
                "standard",
                &op.name.camel_case,
                &return_type,
            ));
        }
        IrReturnType::Void => {
            results.push(build_ts_test_context(
                op,
                "void",
                &op.name.camel_case,
                "void",
            ));
        }
        IrReturnType::Sse(sse) => {
            let sse_name = if sse.also_has_json {
                format!("{}Stream", op.name.camel_case)
            } else {
                op.name.camel_case.clone()
            };
            let return_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                map_type(&sse.event_type, tm)
            };
            results.push(build_ts_test_context(op, "sse", &sse_name, &return_type));

            if let Some(ref json_resp) = sse.json_response {
                let rt = map_type(&json_resp.response_type, tm);
                results.push(build_ts_test_context(
                    op,
                    "standard",
                    &op.name.camel_case,
                    &rt,
                ));
            }
        }
    }

    results
}

fn build_ts_test_context(
    op: &IrOperation,
    kind: &str,
    method_name: &str,
    return_type: &str,
) -> Value {
    let has_body = op.request_body.is_some();
    let test_call_args = build_ts_test_call_args(op);
    let expected_url_pattern = build_ts_expected_url_pattern(op);
    let mock_response = mock_value_ts(&if return_type == "void" {
        IrType::Void
    } else {
        guess_mock_type(return_type)
    });

    context! {
        kind => kind,
        method_name => method_name,
        http_method => op.method.as_str(),
        return_type => return_type,
        has_body => has_body,
        test_call_args => test_call_args,
        expected_url_pattern => expected_url_pattern,
        mock_response => mock_response,
    }
}

fn build_ts_test_call_args(op: &IrOperation) -> String {
    let mut args = Vec::new();
    for param in &op.parameters {
        match param.location {
            IrParameterLocation::Path => args.push(mock_value_ts(&param.param_type)),
            IrParameterLocation::Query | IrParameterLocation::Header => {
                if param.required {
                    args.push(mock_value_ts(&param.param_type));
                }
            }
            _ => {}
        }
    }
    if let Some(ref body) = op.request_body {
        args.push(mock_value_ts(&body.body_type));
    }
    args.join(", ")
}

fn build_ts_expected_url_pattern(op: &IrOperation) -> String {
    let mut path = op.path.clone();
    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            let placeholder = format!("{{{}}}", param.original_name);
            path = path.replace(&placeholder, &mock_path_value_ts(&param.param_type));
        }
    }
    path
}

fn mock_value_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime => "\"test\"".to_string(),
        IrType::StringLiteral(s) => format!("\"{s}\""),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::IntegerLiteral(i) => i.to_string(),
        IrType::Boolean => "true".to_string(),
        IrType::Null | IrType::Void => "undefined".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Ref(name) => format!("{{}} as {}", name),
        IrType::Binary => "new Blob()".to_string(),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            if let Some(first) = variants.first() {
                mock_value_ts(first)
            } else {
                "{}".to_string()
            }
        }
    }
}

fn mock_path_value_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::Integer | IrType::Number => "1".to_string(),
        _ => "test".to_string(),
    }
}

fn guess_mock_type(return_type: &str) -> IrType {
    match return_type {
        "string" => IrType::String,
        "number" => IrType::Number,
        "boolean" => IrType::Boolean,
        "void" => IrType::Void,
        t if t.ends_with("[]") => IrType::Array(Box::new(IrType::Any)),
        _ => IrType::Ref(return_type.to_string()),
    }
}

fn collect_test_type_imports<'a>(ops: impl Iterator<Item = &'a IrOperation>) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();

    for op in ops {
        if let Some(ref body) = op.request_body {
            collect_test_ref_names(&body.body_type, &mut names);
        }
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_test_ref_names(&resp.response_type, &mut names);
            }
            IrReturnType::Sse(sse) => {
                // SSE tests only check for async iterable — event types are never used as
                // type annotations, so skip them to avoid unused-import lint errors.
                if let Some(ref json_resp) = sse.json_response {
                    collect_test_ref_names(&json_resp.response_type, &mut names);
                }
            }
            IrReturnType::Void => {}
        }
    }

    names.into_iter().collect()
}

fn collect_test_ref_names(ir_type: &IrType, names: &mut std::collections::BTreeSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            names.insert(name.clone());
        }
        IrType::Array(inner) => collect_test_ref_names(inner, names),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_test_ref_names(v, names);
            }
        }
        _ => {}
    }
}

// ─── Python test contexts ────────────────────────────────────────

fn build_python_test_contexts(ir: &IrSpec) -> (Vec<Value>, Vec<String>) {
    let model_imports: Vec<String> = ir
        .operations
        .iter()
        .filter_map(|op| {
            op.request_body.as_ref().and_then(|b| match &b.body_type {
                IrType::Ref(name) => Some(name.clone()),
                _ => None,
            })
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .flat_map(build_single_python_test_context)
        .collect();

    (operations, model_imports)
}

fn build_single_python_test_context(op: &IrOperation) -> Vec<Value> {
    let mut results = Vec::new();

    let http_method = match op.method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
        _ => "get",
    };

    let test_path = build_python_test_path(&op.path, op);
    let has_body = op.request_body.is_some();
    let mock_body = op
        .request_body
        .as_ref()
        .map(|b| mock_value_python(&b.body_type))
        .unwrap_or_else(|| "{}".to_string());

    match &op.return_type {
        IrReturnType::Standard(_) => {
            results.push(context! {
                kind => "standard",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => op.path.clone(),
                test_path => test_path,
                has_body => has_body,
                mock_body => mock_body,
            });
        }
        IrReturnType::Void => {
            results.push(context! {
                kind => "void",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => op.path.clone(),
                test_path => test_path,
                has_body => has_body,
                mock_body => mock_body,
            });
        }
        IrReturnType::Sse(sse) => {
            results.push(context! {
                kind => "sse",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => op.path.clone(),
                test_path => test_path,
                has_body => has_body,
                mock_body => mock_body,
            });
            if sse.json_response.is_some() {
                results.push(context! {
                    kind => "standard",
                    name => op.name.snake_case.clone(),
                    http_method => http_method,
                    path => op.path.clone(),
                    test_path => test_path,
                    has_body => has_body,
                    mock_body => mock_body,
                });
            }
        }
    }

    results
}

fn build_python_test_path(path: &str, op: &IrOperation) -> String {
    let mut result = path.to_string();
    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            let placeholder = format!("{{{}}}", param.original_name);
            let test_value = match &param.param_type {
                IrType::Integer | IrType::Number => "1".to_string(),
                _ => "test".to_string(),
            };
            result = result.replace(&placeholder, &test_value);
        }
    }
    result
}

fn mock_value_python(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime => "\"test\"".to_string(),
        IrType::StringLiteral(s) => format!("\"{s}\""),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::IntegerLiteral(i) => i.to_string(),
        IrType::Boolean => "True".to_string(),
        IrType::Null | IrType::Void => "None".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Ref(name) => format!("{}.model_construct()", name),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Binary => "b\"test\"".to_string(),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            if let Some(first) = variants.first() {
                mock_value_python(first)
            } else {
                "{}".to_string()
            }
        }
    }
}
