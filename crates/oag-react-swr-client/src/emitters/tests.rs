use minijinja::{Environment, context};
use oag_core::ir::{IrOperation, IrReturnType, IrSpec};

/// Emit `hooks.test.ts` — vitest smoke tests for React hook exports.
pub fn emit_hooks_tests(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.add_template(
        "hooks.test.ts.j2",
        include_str!("../../templates/hooks.test.ts.j2"),
    )
    .expect("template should be valid");
    let tmpl = env.get_template("hooks.test.ts.j2").unwrap();

    let hook_names: Vec<String> = ir
        .operations
        .iter()
        .flat_map(build_hook_names)
        .collect();

    tmpl.render(context! { hook_names => hook_names })
        .expect("render should succeed")
}

fn build_hook_names(op: &IrOperation) -> Vec<String> {
    let mut names = Vec::new();

    match &op.return_type {
        IrReturnType::Sse(sse) => {
            if sse.also_has_json {
                names.push(format!("use{}Stream", op.name.pascal_case));
                // Also has a JSON hook
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::ir::{HttpMethod, IrResponse, IrType, NormalizedName};

    fn make_name(name: &str) -> NormalizedName {
        NormalizedName {
            original: name.to_string(),
            pascal_case: name.to_string(),
            camel_case: {
                let mut c = name.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_lowercase().to_string() + c.as_str(),
                }
            },
            snake_case: name.to_lowercase(),
            screaming_snake: name.to_uppercase(),
        }
    }

    #[test]
    fn test_standard_hook_name() {
        let op = IrOperation {
            name: make_name("ListPets"),
            method: HttpMethod::Get,
            path: "/pets".to_string(),
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body: None,
            return_type: IrReturnType::Standard(IrResponse {
                response_type: IrType::Array(Box::new(IrType::Ref("Pet".to_string()))),
                description: None,
            }),
            deprecated: false,
        };
        let names = build_hook_names(&op);
        assert_eq!(names, vec!["useListPets"]);
    }
}
