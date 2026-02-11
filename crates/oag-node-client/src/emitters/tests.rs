use minijinja::{Environment, context};
use oag_core::ir::{IrOperation, IrParameterLocation, IrReturnType, IrSpec, IrType};

use crate::type_mapper::ir_type_to_ts;

/// Emit `client.test.ts` — vitest tests for the API client.
pub fn emit_client_tests(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_template(
        "client.test.ts.j2",
        include_str!("../../templates/client.test.ts.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("client.test.ts.j2").unwrap();

    // Collect type names referenced in mock values for imports
    let type_imports: Vec<String> = collect_type_imports(ir);

    let operations: Vec<minijinja::Value> = ir
        .operations
        .iter()
        .flat_map(build_test_operation_contexts)
        .collect();

    tmpl.render(context! {
        operations => operations,
        type_imports => type_imports,
    })
    .expect("render should succeed")
}

/// Collect unique type names used in mock values across all operations.
fn collect_type_imports(ir: &IrSpec) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();

    for op in &ir.operations {
        // Request body refs
        if let Some(ref body) = op.request_body {
            collect_ref_names(&body.body_type, &mut names);
        }
        // Return type refs (used in mock_response via guess_mock_type)
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_ref_names(&resp.response_type, &mut names);
            }
            IrReturnType::Sse(sse) => {
                collect_ref_names(&sse.event_type, &mut names);
                if let Some(ref json_resp) = sse.json_response {
                    collect_ref_names(&json_resp.response_type, &mut names);
                }
            }
            IrReturnType::Void => {}
        }
    }

    names.into_iter().collect()
}

fn collect_ref_names(ir_type: &IrType, names: &mut std::collections::BTreeSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            names.insert(name.clone());
        }
        IrType::Array(inner) => collect_ref_names(inner, names),
        IrType::Union(variants) => {
            for v in variants {
                collect_ref_names(v, names);
            }
        }
        _ => {}
    }
}

fn build_test_operation_contexts(op: &IrOperation) -> Vec<minijinja::Value> {
    let mut results = Vec::new();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = ir_type_to_ts(&resp.response_type);
            results.push(build_test_context(op, "standard", &op.name.camel_case, &return_type));
        }
        IrReturnType::Void => {
            results.push(build_test_context(op, "void", &op.name.camel_case, "void"));
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
                ir_type_to_ts(&sse.event_type)
            };
            results.push(build_test_context(op, "sse", &sse_name, &return_type));

            if let Some(ref json_resp) = sse.json_response {
                let rt = ir_type_to_ts(&json_resp.response_type);
                results.push(build_test_context(op, "standard", &op.name.camel_case, &rt));
            }
        }
    }

    results
}

fn build_test_context(
    op: &IrOperation,
    kind: &str,
    method_name: &str,
    return_type: &str,
) -> minijinja::Value {
    let has_body = op.request_body.is_some();
    let test_call_args = build_test_call_args(op);
    let expected_url_pattern = build_expected_url_pattern(op);
    let mock_response = mock_value_ts(&if return_type == "void" {
        IrType::Void
    } else {
        // Use a simple mock for the response
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

/// Build test call arguments for an operation.
fn build_test_call_args(op: &IrOperation) -> String {
    let mut args = Vec::new();

    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            args.push(mock_value_ts(&param.param_type));
        }
    }

    for param in &op.parameters {
        if param.location == IrParameterLocation::Query && param.required {
            args.push(mock_value_ts(&param.param_type));
        }
    }

    if let Some(ref body) = op.request_body {
        args.push(mock_value_ts(&body.body_type));
    }

    args.join(", ")
}

/// Build the expected URL pattern for assertions.
fn build_expected_url_pattern(op: &IrOperation) -> String {
    let mut path = op.path.clone();
    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            let placeholder = format!("{{{}}}", param.original_name);
            path = path.replace(&placeholder, &mock_path_value_ts(&param.param_type));
        }
    }
    path
}

/// Generate a mock TypeScript value for a given IrType.
fn mock_value_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime => "\"test\"".to_string(),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::Boolean => "true".to_string(),
        IrType::Null | IrType::Void => "undefined".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Ref(name) => format!("{{}} as {}", name),
        IrType::Binary => "new Blob()".to_string(),
        IrType::Union(variants) => {
            if let Some(first) = variants.first() {
                mock_value_ts(first)
            } else {
                "{}".to_string()
            }
        }
    }
}

/// Mock path parameter value as a string for URL patterns.
fn mock_path_value_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::Integer | IrType::Number => "1".to_string(),
        _ => "test".to_string(),
    }
}

/// Guess a mock IrType from a return type string for simple response mocking.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_value_ts() {
        assert_eq!(mock_value_ts(&IrType::String), "\"test\"");
        assert_eq!(mock_value_ts(&IrType::Integer), "1");
        assert_eq!(mock_value_ts(&IrType::Boolean), "true");
        assert_eq!(mock_value_ts(&IrType::Void), "undefined");
        assert_eq!(mock_value_ts(&IrType::Ref("Pet".to_string())), "{} as Pet");
    }

    #[test]
    fn test_mock_path_value() {
        assert_eq!(mock_path_value_ts(&IrType::Integer), "1");
        assert_eq!(mock_path_value_ts(&IrType::String), "test");
    }
}
