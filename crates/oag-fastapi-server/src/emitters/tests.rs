use minijinja::{Environment, context};
use oag_core::ir::{HttpMethod, IrOperation, IrParameterLocation, IrReturnType, IrSpec, IrType};
use oag_core::GeneratedFile;

/// Emit `conftest.py` + `test_routes.py` for pytest.
pub fn emit_tests(ir: &IrSpec) -> Vec<GeneratedFile> {
    vec![
        GeneratedFile {
            path: "conftest.py".to_string(),
            content: include_str!("../../templates/conftest.py.j2").to_string(),
        },
        GeneratedFile {
            path: "test_routes.py".to_string(),
            content: emit_test_routes(ir),
        },
    ]
}

fn emit_test_routes(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_template(
        "test_routes.py.j2",
        include_str!("../../templates/test_routes.py.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("test_routes.py.j2").unwrap();

    // Collect model names referenced in request bodies for imports
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

    let operations: Vec<minijinja::Value> = ir
        .operations
        .iter()
        .flat_map(build_test_operation_contexts)
        .collect();

    tmpl.render(context! {
        operations => operations,
        model_imports => model_imports,
    })
    .expect("render should succeed")
}

fn build_test_operation_contexts(op: &IrOperation) -> Vec<minijinja::Value> {
    let mut results = Vec::new();

    let http_method = match op.method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
        _ => "get",
    };

    // Replace path params with placeholder values for test URLs
    let test_path = build_test_path(&op.path, op);
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

            // Also test the JSON endpoint if dual
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

/// Replace `{param}` placeholders in the path with test values.
fn build_test_path(path: &str, op: &IrOperation) -> String {
    let mut result = path.to_string();
    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            let placeholder = format!("{{{}}}", param.original_name);
            let test_value = mock_path_value(&param.param_type);
            result = result.replace(&placeholder, &test_value);
        }
    }
    result
}

/// Generate a mock path parameter value.
fn mock_path_value(ir_type: &IrType) -> String {
    match ir_type {
        IrType::Integer => "1".to_string(),
        IrType::Number => "1".to_string(),
        IrType::String | IrType::DateTime => "test".to_string(),
        _ => "test".to_string(),
    }
}

/// Generate a mock Python value for a given IrType (for request bodies).
fn mock_value_python(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime => "\"test\"".to_string(),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::Boolean => "True".to_string(),
        IrType::Null | IrType::Void => "None".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Ref(name) => format!("{}.model_construct()", name),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Binary => "b\"test\"".to_string(),
        IrType::Union(variants) => {
            if let Some(first) = variants.first() {
                mock_value_python(first)
            } else {
                "{}".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_path_value() {
        assert_eq!(mock_path_value(&IrType::Integer), "1");
        assert_eq!(mock_path_value(&IrType::String), "test");
    }

    #[test]
    fn test_mock_value_python() {
        assert_eq!(mock_value_python(&IrType::String), "\"test\"");
        assert_eq!(mock_value_python(&IrType::Integer), "1");
        assert_eq!(mock_value_python(&IrType::Boolean), "True");
        assert_eq!(mock_value_python(&IrType::Array(Box::new(IrType::String))), "[]");
        assert_eq!(
            mock_value_python(&IrType::Ref("Pet".to_string())),
            "Pet.model_construct()"
        );
    }
}
