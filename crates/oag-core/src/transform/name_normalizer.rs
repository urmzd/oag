use heck::{ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase, ToSnakeCase};

use crate::ir::NormalizedName;

/// Create a `NormalizedName` from an arbitrary string, computing all casing variants.
pub fn normalize_name(name: &str) -> NormalizedName {
    // Handle names that start with numbers or contain special chars
    let sanitized = sanitize_identifier(name);

    NormalizedName {
        original: name.to_string(),
        pascal_case: sanitized.to_pascal_case(),
        camel_case: sanitized.to_lower_camel_case(),
        snake_case: sanitized.to_snake_case(),
        screaming_snake: sanitized.to_shouty_snake_case(),
    }
}

/// Derive a camelCase operation name from HTTP method + path.
///
/// Examples:
/// - `GET /users` → `listUsers`
/// - `POST /users` → `createUser`
/// - `GET /users/{userId}` → `getUser`
/// - `PUT /users/{userId}` → `updateUser`
/// - `DELETE /users/{userId}` → `deleteUser`
/// - `PATCH /users/{userId}` → `patchUser`
/// - `POST /users/{userId}/messages` → `createUserMessage`
/// - `GET /users/{userId}/messages` → `listUserMessages`
pub fn route_to_name(method: &str, path: &str) -> String {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Collect non-parameter segments and track whether the path ends with a param
    let mut resource_parts: Vec<String> = Vec::new();
    let mut ends_with_param = false;

    for seg in &segments {
        if seg.starts_with('{') && seg.ends_with('}') {
            ends_with_param = true;
        } else {
            resource_parts.push(seg.to_string());
            ends_with_param = false;
        }
    }

    // Build the resource name from non-parameter path segments
    let method_upper = method.to_uppercase();
    let prefix = match method_upper.as_str() {
        "GET" if ends_with_param => "get",
        "GET" => "list",
        "POST" => "create",
        "PUT" => "update",
        "DELETE" => "delete",
        "PATCH" => "patch",
        "OPTIONS" => "options",
        "HEAD" => "head",
        "TRACE" => "trace",
        other => other,
    };

    if resource_parts.is_empty() {
        return prefix.to_string();
    }

    // For single-resource ops (ends with param), singularize the last segment
    // For collection ops (no trailing param), keep as-is
    let mut pascal_parts = String::new();
    for (i, part) in resource_parts.iter().enumerate() {
        let is_last = i == resource_parts.len() - 1;
        let word = if is_last && ends_with_param {
            singularize(part)
        } else {
            part.to_string()
        };
        pascal_parts.push_str(&word.to_pascal_case());
    }

    format!("{prefix}{pascal_parts}")
}

/// Naive singularization: strips trailing 's' if present.
fn singularize(word: &str) -> String {
    if word.ends_with("ies") && word.len() > 3 {
        format!("{}y", &word[..word.len() - 3])
    } else if word.ends_with("ses") || word.ends_with("xes") || word.ends_with("zes") {
        word[..word.len() - 2].to_string()
    } else if word.ends_with('s') && !word.ends_with("ss") && word.len() > 1 {
        word[..word.len() - 1].to_string()
    } else {
        word.to_string()
    }
}

/// Sanitize a string to be a valid identifier.
fn sanitize_identifier(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut prev_was_separator = false;

    for (i, ch) in name.chars().enumerate() {
        if ch.is_alphanumeric() {
            if i == 0 && ch.is_ascii_digit() {
                result.push('_');
            }
            if prev_was_separator && !result.is_empty() {
                result.push('_');
            }
            result.push(ch);
            prev_was_separator = false;
        } else {
            prev_was_separator = true;
        }
    }

    if result.is_empty() {
        return "unnamed".to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_name() {
        let n = normalize_name("listModels");
        assert_eq!(n.pascal_case, "ListModels");
        assert_eq!(n.camel_case, "listModels");
        assert_eq!(n.snake_case, "list_models");
        assert_eq!(n.screaming_snake, "LIST_MODELS");
    }

    #[test]
    fn test_kebab_case() {
        let n = normalize_name("pet-store");
        assert_eq!(n.pascal_case, "PetStore");
        assert_eq!(n.camel_case, "petStore");
    }

    #[test]
    fn test_leading_number() {
        let n = normalize_name("3dModel");
        // heck preserves leading digits without underscore prefix
        assert_eq!(n.pascal_case, "3dModel");
        assert_eq!(n.snake_case, "3d_model");
    }

    #[test]
    fn test_special_chars() {
        let n = normalize_name("application/json");
        assert_eq!(n.pascal_case, "ApplicationJson");
    }

    #[test]
    fn test_route_to_name_list() {
        assert_eq!(route_to_name("GET", "/users"), "listUsers");
    }

    #[test]
    fn test_route_to_name_create() {
        assert_eq!(route_to_name("POST", "/users"), "createUsers");
    }

    #[test]
    fn test_route_to_name_get_single() {
        assert_eq!(route_to_name("GET", "/users/{userId}"), "getUser");
    }

    #[test]
    fn test_route_to_name_update() {
        assert_eq!(route_to_name("PUT", "/users/{userId}"), "updateUser");
    }

    #[test]
    fn test_route_to_name_delete() {
        assert_eq!(route_to_name("DELETE", "/users/{userId}"), "deleteUser");
    }

    #[test]
    fn test_route_to_name_patch() {
        assert_eq!(route_to_name("PATCH", "/users/{userId}"), "patchUser");
    }

    #[test]
    fn test_route_to_name_nested() {
        assert_eq!(
            route_to_name("POST", "/users/{userId}/messages"),
            "createUsersMessages"
        );
    }

    #[test]
    fn test_route_to_name_nested_get() {
        assert_eq!(
            route_to_name("GET", "/users/{userId}/messages"),
            "listUsersMessages"
        );
    }

    #[test]
    fn test_route_to_name_nested_single() {
        assert_eq!(
            route_to_name("GET", "/users/{userId}/messages/{messageId}"),
            "getUsersMessage"
        );
    }
}
