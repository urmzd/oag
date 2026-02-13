use minijinja::{Environment, context};
use oag_core::ir::{HttpMethod, IrOperation, IrParameterLocation, IrReturnType, IrSpec, IrType};

use crate::type_mapper::ir_type_to_python;

/// Escape triple-quote sequences that would prematurely close Python docstrings.
fn escape_docstring(value: String) -> String {
    value.replace("\"\"\"", "\\\"\\\"\\\"")
}

/// Emit `routes.py` — FastAPI router with stub endpoints.
pub fn emit_routes(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_filter("escape_docstring", escape_docstring);
    env.add_template("routes.py.j2", include_str!("../../templates/routes.py.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("routes.py.j2").unwrap();

    let operations: Vec<minijinja::Value> = ir
        .operations
        .iter()
        .flat_map(build_operation_contexts)
        .collect();

    let model_imports = collect_model_imports(ir);

    tmpl.render(context! {
        operations => operations,
        model_imports => model_imports,
    })
    .expect("render should succeed")
}

fn build_operation_contexts(op: &IrOperation) -> Vec<minijinja::Value> {
    let mut results = Vec::new();

    let http_method = match op.method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
        _ => "get",
    };

    // Convert OpenAPI path params {param} to FastAPI path params {param}
    // (they use the same syntax so no conversion needed)
    let path = op.path.clone();

    let (params, has_body, body_type, body_param_name) = build_params(op);

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = ir_type_to_python(&resp.response_type);
            results.push(context! {
                kind => "standard",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params => params,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                return_type => return_type,
                summary => op.summary.clone(),
                description => op.description.clone(),
            });
        }
        IrReturnType::Void => {
            results.push(context! {
                kind => "void",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params => params,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                return_type => "None",
                summary => op.summary.clone(),
                description => op.description.clone(),
            });
        }
        IrReturnType::Sse(sse) => {
            let event_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                ir_type_to_python(&sse.event_type)
            };
            results.push(context! {
                kind => "sse",
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params => params,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                event_type => event_type,
                summary => op.summary.clone(),
                description => op.description.clone(),
            });

            // Also generate JSON endpoint if dual
            if let Some(ref json_resp) = sse.json_response {
                let return_type = ir_type_to_python(&json_resp.response_type);
                results.push(context! {
                    kind => "standard",
                    name => op.name.snake_case.clone(),
                    http_method => http_method,
                    path => path,
                    params => params,
                    has_body => has_body,
                    body_type => body_type,
                    body_param_name => body_param_name,
                    return_type => return_type,
                    summary => op.summary.clone(),
                    description => format!("{} (JSON response)", op.description.as_deref().unwrap_or("")),
                });
            }
        }
    }

    results
}

fn build_params(op: &IrOperation) -> (Vec<minijinja::Value>, bool, String, String) {
    let mut params = Vec::new();

    for param in &op.parameters {
        let py_type = ir_type_to_python(&param.param_type);
        let location = match param.location {
            IrParameterLocation::Path => "path",
            IrParameterLocation::Query => "query",
            IrParameterLocation::Header => "header",
            IrParameterLocation::Cookie => "cookie",
        };
        params.push(context! {
            name => param.name.snake_case.clone(),
            original_name => param.original_name.clone(),
            type_str => py_type,
            location => location,
            required => param.required,
            needs_alias => param.name.snake_case != param.original_name,
        });
    }

    let has_body = op.request_body.is_some();
    let body_type = op
        .request_body
        .as_ref()
        .map(|b| ir_type_to_python(&b.body_type))
        .unwrap_or_default();
    let body_param_name = "body".to_string();

    (params, has_body, body_type, body_param_name)
}

fn collect_model_imports(ir: &IrSpec) -> Vec<String> {
    let mut imports = std::collections::HashSet::new();

    for op in &ir.operations {
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_refs(&resp.response_type, &mut imports);
            }
            IrReturnType::Sse(sse) => {
                if let Some(ref name) = sse.event_type_name {
                    imports.insert(name.clone());
                } else {
                    collect_refs(&sse.event_type, &mut imports);
                }
                if let Some(ref json) = sse.json_response {
                    collect_refs(&json.response_type, &mut imports);
                }
            }
            IrReturnType::Void => {}
        }
        if let Some(ref body) = op.request_body {
            collect_refs(&body.body_type, &mut imports);
        }
        for param in &op.parameters {
            collect_refs(&param.param_type, &mut imports);
        }
    }

    let mut sorted: Vec<String> = imports.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_refs(ir_type: &IrType, imports: &mut std::collections::HashSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            imports.insert(name.clone());
        }
        IrType::Array(inner) | IrType::Map(inner) => collect_refs(inner, imports),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_refs(v, imports);
            }
        }
        _ => {}
    }
}
