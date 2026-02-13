use std::collections::HashSet;

use minijinja::{Environment, context};
use oag_core::ir::{IrOperation, IrParameterLocation, IrReturnType, IrSpec, IrType};

use crate::type_mapper::ir_type_to_ts;

/// Escape `*/` sequences that would prematurely close JSDoc comment blocks.
fn escape_jsdoc(value: String) -> String {
    value.replace("*/", "*\\/")
}

/// Emit `client.ts` — the API client class with REST and SSE methods.
pub fn emit_client(ir: &IrSpec, _no_jsdoc: bool) -> String {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.add_filter("escape_jsdoc", escape_jsdoc);
    env.add_template("client.ts.j2", include_str!("../../templates/client.ts.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("client.ts.j2").unwrap();

    // Build and deduplicate operations, tracking which source ops survived.
    let mut seen_methods = HashSet::new();
    let mut used_op_indices = HashSet::new();
    let operations: Vec<minijinja::Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_operation_contexts(op)
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

    // Only collect types from operations that contributed surviving methods.
    let imported_types = collect_imported_types(
        ir.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| used_op_indices.contains(i))
            .map(|(_, op)| op),
    );

    let has_sse = operations.iter().any(|op| {
        op.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });

    tmpl.render(context! {
        title => ir.info.title.clone(),
        imported_types => imported_types,
        operations => operations,
        has_sse => has_sse,
        no_jsdoc => _no_jsdoc,
    })
    .expect("render should succeed")
}

fn build_operation_contexts(op: &IrOperation) -> Vec<minijinja::Value> {
    let mut results = Vec::new();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            results.push(build_standard_op(op, &ir_type_to_ts(&resp.response_type)));
        }
        IrReturnType::Void => {
            results.push(build_void_op(op));
        }
        IrReturnType::Sse(sse) => {
            let return_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                ir_type_to_ts(&sse.event_type)
            };
            let sse_name = if sse.also_has_json {
                format!("{}Stream", op.name.camel_case)
            } else {
                op.name.camel_case.clone()
            };
            results.push(build_sse_op(op, &return_type, &sse_name));

            if let Some(ref json_resp) = sse.json_response {
                results.push(build_standard_op(
                    op,
                    &ir_type_to_ts(&json_resp.response_type),
                ));
            }
        }
    }

    results
}

fn build_standard_op(op: &IrOperation, return_type: &str) -> minijinja::Value {
    let result = build_params(op);

    context! {
        kind => "standard",
        method_name => op.name.camel_case.clone(),
        http_method => op.method.as_str(),
        path => op.path.clone(),
        params_signature => result.parts.join(", "),
        return_type => return_type,
        path_params => result.path_params,
        query_params_obj => result.query_params_obj,
        header_params_obj => result.header_params_obj,
        has_body => result.has_body,
        body_content_type => result.body_content_type.clone(),
        has_path_params => result.has_path_params,
        has_query_params => result.has_query_params,
        has_header_params => result.has_header_params,
        summary => op.summary.clone(),
        description => op.description.clone(),
        deprecated => op.deprecated,
    }
}

fn build_void_op(op: &IrOperation) -> minijinja::Value {
    let result = build_params(op);

    context! {
        kind => "void",
        method_name => op.name.camel_case.clone(),
        http_method => op.method.as_str(),
        path => op.path.clone(),
        params_signature => result.parts.join(", "),
        return_type => "void",
        path_params => result.path_params,
        query_params_obj => result.query_params_obj,
        header_params_obj => result.header_params_obj,
        has_body => result.has_body,
        body_content_type => result.body_content_type.clone(),
        has_path_params => result.has_path_params,
        has_query_params => result.has_query_params,
        has_header_params => result.has_header_params,
        summary => op.summary.clone(),
        description => op.description.clone(),
        deprecated => op.deprecated,
    }
}

fn build_sse_op(op: &IrOperation, return_type: &str, method_name: &str) -> minijinja::Value {
    let mut result = build_params_raw(op);

    // For SSE, use SSEOptions instead of RequestOptions
    if let Some(last) = result.parts.last_mut()
        && last.starts_with("options?")
    {
        *last = "options?: SSEOptions".to_string();
    }
    let params_sig = result.parts.join(", ");

    context! {
        kind => "sse",
        method_name => method_name,
        http_method => op.method.as_str(),
        path => op.path.clone(),
        params_signature => params_sig,
        return_type => return_type,
        path_params => result.path_params,
        query_params_obj => result.query_params_obj,
        header_params_obj => result.header_params_obj,
        has_body => result.has_body,
        body_content_type => result.body_content_type.clone(),
        has_path_params => result.has_path_params,
        has_query_params => result.has_query_params,
        has_header_params => result.has_header_params,
        summary => op.summary.clone(),
        description => op.description.clone(),
        deprecated => op.deprecated,
    }
}

struct ParamsResult {
    parts: Vec<String>,
    path_params: Vec<minijinja::Value>,
    query_params_obj: String,
    header_params_obj: String,
    has_body: bool,
    body_content_type: String,
    has_path_params: bool,
    has_query_params: bool,
    has_header_params: bool,
}

fn build_params(op: &IrOperation) -> ParamsResult {
    build_params_raw(op)
}

fn build_params_raw(op: &IrOperation) -> ParamsResult {
    let mut required_parts = Vec::new();
    let mut optional_parts = Vec::new();
    let mut path_params = Vec::new();
    let mut query_parts = Vec::new();
    let mut header_parts = Vec::new();

    for param in &op.parameters {
        let ts_type = ir_type_to_ts(&param.param_type);
        match param.location {
            IrParameterLocation::Path => {
                required_parts.push(format!("{}: {}", param.name.camel_case, ts_type));
                path_params.push(context! {
                    name => param.name.camel_case.clone(),
                    original_name => param.original_name.clone(),
                });
            }
            IrParameterLocation::Query => {
                if param.required {
                    required_parts.push(format!("{}: {}", param.name.camel_case, ts_type));
                } else {
                    optional_parts.push(format!("{}?: {}", param.name.camel_case, ts_type));
                }
                query_parts.push(format!(
                    "\"{}\": {}",
                    param.original_name, param.name.camel_case
                ));
            }
            IrParameterLocation::Header => {
                if param.required {
                    required_parts.push(format!("{}: {}", param.name.camel_case, ts_type));
                } else {
                    optional_parts.push(format!("{}?: {}", param.name.camel_case, ts_type));
                }
                header_parts.push(format!(
                    "\"{}\": {}",
                    param.original_name, param.name.camel_case
                ));
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
        let ts_type = ir_type_to_ts(&body.body_type);
        if body.required {
            required_parts.push(format!("body: {ts_type}"));
        } else {
            optional_parts.push(format!("body?: {ts_type}"));
        }
    }

    optional_parts.push("options?: RequestOptions".to_string());

    let mut parts = required_parts;
    parts.extend(optional_parts);

    let has_path_params = !path_params.is_empty();
    let has_query_params = !query_parts.is_empty();
    let has_header_params = !header_parts.is_empty();
    let query_params_obj = query_parts.join(", ");
    let header_params_obj = header_parts.join(", ");

    ParamsResult {
        parts,
        path_params,
        query_params_obj,
        header_params_obj,
        has_body,
        body_content_type,
        has_path_params,
        has_query_params,
        has_header_params,
    }
}

fn collect_imported_types<'a>(ops: impl Iterator<Item = &'a IrOperation>) -> Vec<String> {
    let mut types = HashSet::new();

    for op in ops {
        collect_types_from_return(&op.return_type, &mut types);

        if let Some(ref body) = op.request_body {
            collect_types_from_ir_type(&body.body_type, &mut types);
        }

        for param in &op.parameters {
            collect_types_from_ir_type(&param.param_type, &mut types);
        }
    }

    let mut sorted: Vec<String> = types.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_types_from_return(ret: &IrReturnType, types: &mut HashSet<String>) {
    match ret {
        IrReturnType::Standard(resp) => {
            collect_types_from_ir_type(&resp.response_type, types);
        }
        IrReturnType::Sse(sse) => {
            if let Some(ref name) = sse.event_type_name {
                types.insert(name.clone());
            } else {
                collect_types_from_ir_type(&sse.event_type, types);
            }
            // SSE variant types are only used in type definitions (types.ts),
            // not in client method signatures — skip them here.
            if let Some(ref json) = sse.json_response {
                collect_types_from_ir_type(&json.response_type, types);
            }
        }
        IrReturnType::Void => {}
    }
}

fn collect_types_from_ir_type(ir_type: &IrType, types: &mut HashSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            types.insert(name.clone());
        }
        IrType::Array(inner) | IrType::Map(inner) => collect_types_from_ir_type(inner, types),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_types_from_ir_type(v, types);
            }
        }
        IrType::Object(fields) => {
            for (_, ty, _) in fields {
                collect_types_from_ir_type(ty, types);
            }
        }
        _ => {}
    }
}
