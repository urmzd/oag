use std::collections::HashSet;

use minijinja::{Environment, context};
use oag_core::ir::{HttpMethod, IrOperation, IrParameterLocation, IrReturnType, IrSpec, IrType};
use oag_node_client::type_mapper::ir_type_to_ts;

/// Escape `*/` sequences that would prematurely close JSDoc comment blocks.
fn escape_jsdoc(value: String) -> String {
    value.replace("*/", "*\\/")
}

/// Emit `hooks.ts` — React hooks wrapping the API client.
pub fn emit_hooks(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.add_filter("escape_jsdoc", escape_jsdoc);
    env.add_template("hooks.ts.j2", include_str!("../../templates/hooks.ts.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("hooks.ts.j2").unwrap();

    let mut seen_hooks = HashSet::new();
    let mut used_op_indices = HashSet::new();
    let hooks: Vec<minijinja::Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_hook_contexts(op)
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
    let imported_types = collect_imported_types(
        ir.operations
            .iter()
            .enumerate()
            .filter(|(i, _)| used_op_indices.contains(i))
            .map(|(_, op)| op),
    );
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
    let has_sse = hooks.iter().any(|h| {
        h.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });

    tmpl.render(context! {
        imported_types => imported_types,
        hooks => hooks,
        has_queries => has_queries,
        has_mutations => has_mutations,
        has_sse => has_sse,
    })
    .expect("render should succeed")
}

fn build_hook_contexts(op: &IrOperation) -> Vec<minijinja::Value> {
    let mut results = Vec::new();

    match (&op.method, &op.return_type) {
        // GET → useSWR query hook
        (HttpMethod::Get, IrReturnType::Standard(resp)) => {
            let return_type = ir_type_to_ts(&resp.response_type);
            let (params_sig, swr_key, call_args) = build_query_params(op);
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
        // POST/PUT/DELETE non-streaming → useSWRMutation hook
        (_, IrReturnType::Standard(_)) | (_, IrReturnType::Void) => {
            let return_type = match &op.return_type {
                IrReturnType::Standard(r) => ir_type_to_ts(&r.response_type),
                _ => "void".to_string(),
            };
            let has_body = op.request_body.is_some();
            let body_type = op
                .request_body
                .as_ref()
                .map(|b| ir_type_to_ts(&b.body_type))
                .unwrap_or_else(|| "void".to_string());

            let (path_params_sig, swr_key, call_args, swr_key_type) = build_mutation_params(op);
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
                ir_type_to_ts(&sse.event_type)
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
                build_sse_hook_params(op);

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

            // If dual endpoint, also generate the JSON query/mutation hook
            if let Some(ref json_resp) = sse.json_response {
                let return_type = ir_type_to_ts(&json_resp.response_type);
                match op.method {
                    HttpMethod::Get => {
                        let (params_sig, swr_key, call_args) = build_query_params(op);
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
                            .map(|b| ir_type_to_ts(&b.body_type))
                            .unwrap_or_else(|| "void".to_string());
                        let (path_params_sig, swr_key, call_args, swr_key_type) =
                            build_mutation_params(op);
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

fn build_query_params(op: &IrOperation) -> (String, String, String) {
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
                let ts = ir_type_to_ts(&param.param_type);
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

    let params_sig = sig_parts.join(", ");
    let call_args = call_parts.join(", ");

    (params_sig, swr_key, call_args)
}

fn build_mutation_params(op: &IrOperation) -> (String, String, String, String) {
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
                let ts = ir_type_to_ts(&param.param_type);
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

    // For mutation, the body comes from arg
    if op.request_body.is_some() {
        call_parts.push("arg".to_string());
    }

    let path_params_sig = sig_parts.join(", ");
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
    let call_args = call_parts.join(", ");

    (path_params_sig, swr_key, call_args, swr_key_type)
}

fn build_sse_hook_params(op: &IrOperation) -> (String, String, String, String) {
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
                let ts = ir_type_to_ts(&param.param_type);
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
        let ts = ir_type_to_ts(&body.body_type);
        stream_call_parts.push("body".to_string());
        if body.required {
            format!("body: {}", ts)
        } else {
            format!("body?: {}", ts)
        }
    } else {
        String::new()
    };

    let path_params_sig = sig_parts.join(", ");
    let stream_call_args = stream_call_parts.join(", ");
    let deps = deps_parts.join("");

    (path_params_sig, trigger_params, stream_call_args, deps)
}

fn collect_imported_types<'a>(ops: impl Iterator<Item = &'a IrOperation>) -> Vec<String> {
    let mut types = HashSet::new();

    for op in ops {
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_refs(&resp.response_type, &mut types);
            }
            IrReturnType::Sse(sse) => {
                if let Some(ref name) = sse.event_type_name {
                    types.insert(name.clone());
                } else {
                    collect_refs(&sse.event_type, &mut types);
                }
                if let Some(ref json) = sse.json_response {
                    collect_refs(&json.response_type, &mut types);
                }
            }
            IrReturnType::Void => {}
        }
        if let Some(ref body) = op.request_body {
            collect_refs(&body.body_type, &mut types);
        }
    }

    let mut sorted: Vec<String> = types.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_refs(ir_type: &IrType, types: &mut HashSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            types.insert(name.clone());
        }
        IrType::Array(inner) | IrType::Map(inner) => collect_refs(inner, types),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_refs(v, types);
            }
        }
        _ => {}
    }
}
