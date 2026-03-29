use crate::ir::IrType;

use super::pack::TypeMapConfig;

/// Map an IrType to a language type string using the declarative type map.
pub fn map_type(ir_type: &IrType, tm: &TypeMapConfig) -> String {
    match ir_type {
        IrType::String => tm.string.clone(),
        IrType::StringLiteral(s) => tm.string_literal.replace("{value}", s),
        IrType::IntegerLiteral(i) => tm.integer_literal.replace("{value}", &i.to_string()),
        IrType::Number => tm.number.clone(),
        IrType::Integer => tm.integer.clone(),
        IrType::Boolean => tm.boolean.clone(),
        IrType::Null => tm.null.clone(),
        IrType::DateTime => tm.datetime.clone(),
        IrType::Binary => tm.binary.clone(),
        IrType::Any => tm.any.clone(),
        IrType::Void => tm.void.clone(),
        IrType::Ref(name) => tm.ref_type.replace("{name}", name),
        IrType::Array(inner) => {
            let inner_str = map_type(inner, tm);
            // Use array_union template if inner is a union and the key exists
            if matches!(inner.as_ref(), IrType::Union(_))
                && let Some(ref array_union) = tm.array_union
            {
                return array_union.replace("{inner}", &inner_str);
            }
            tm.array.replace("{inner}", &inner_str)
        }
        IrType::Map(value_type) => {
            let inner_str = map_type(value_type, tm);
            tm.map.replace("{inner}", &inner_str)
        }
        IrType::Object(fields) => {
            if fields.is_empty() {
                return tm.object_empty.clone();
            }
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(name, ty, required)| {
                    let type_str = map_type(ty, tm);
                    if *required {
                        tm.object_field_required
                            .replace("{name}", name)
                            .replace("{type}", &type_str)
                    } else {
                        tm.object_field_optional
                            .replace("{name}", name)
                            .replace("{type}", &type_str)
                    }
                })
                .collect();
            tm.object
                .replace("{fields}", &field_strs.join(&tm.object_field_separator))
        }
        IrType::Union(variants) => {
            let variant_strs: Vec<String> = variants.iter().map(|v| map_type(v, tm)).collect();
            variant_strs.join(&tm.union_separator)
        }
        IrType::Intersection(parts) => {
            let part_strs: Vec<String> = parts.iter().map(|p| map_type(p, tm)).collect();
            part_strs.join(&tm.intersection_separator)
        }
    }
}

/// Map an IrType to a field type string, applying optional suffix for non-required fields.
pub fn map_field_type(ir_type: &IrType, required: bool, tm: &TypeMapConfig) -> String {
    let base = map_type(ir_type, tm);
    if required || tm.optional_suffix.is_empty() {
        base
    } else {
        format!("{base}{}", tm.optional_suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts_type_map() -> TypeMapConfig {
        TypeMapConfig {
            string: "string".into(),
            number: "number".into(),
            integer: "number".into(),
            boolean: "boolean".into(),
            null: "null".into(),
            datetime: "string".into(),
            binary: "Blob".into(),
            any: "unknown".into(),
            void: "void".into(),
            string_literal: "\"{value}\"".into(),
            integer_literal: "{value}".into(),
            ref_type: "{name}".into(),
            array: "{inner}[]".into(),
            array_union: Some("({inner})[]".into()),
            map: "Record<string, {inner}>".into(),
            object: "{ {fields} }".into(),
            object_empty: "Record<string, unknown>".into(),
            object_field_required: "{name}: {type}".into(),
            object_field_optional: "{name}?: {type}".into(),
            object_field_separator: "; ".into(),
            union_separator: " | ".into(),
            intersection_separator: " & ".into(),
            optional_suffix: String::new(),
        }
    }

    fn py_type_map() -> TypeMapConfig {
        TypeMapConfig {
            string: "str".into(),
            number: "float".into(),
            integer: "int".into(),
            boolean: "bool".into(),
            null: "None".into(),
            datetime: "str".into(),
            binary: "bytes".into(),
            any: "Any".into(),
            void: "None".into(),
            string_literal: "Literal[\"{value}\"]".into(),
            integer_literal: "Literal[{value}]".into(),
            ref_type: "{name}".into(),
            array: "list[{inner}]".into(),
            array_union: None,
            map: "dict[str, {inner}]".into(),
            object: "dict[str, Any]".into(),
            object_empty: "dict[str, Any]".into(),
            object_field_required: "{name}: {type}".into(),
            object_field_optional: "{name}: {type}".into(),
            object_field_separator: ", ".into(),
            union_separator: " | ".into(),
            intersection_separator: ", ".into(),
            optional_suffix: " | None = None".into(),
        }
    }

    #[test]
    fn ts_primitives() {
        let tm = ts_type_map();
        assert_eq!(map_type(&IrType::String, &tm), "string");
        assert_eq!(map_type(&IrType::Number, &tm), "number");
        assert_eq!(map_type(&IrType::Integer, &tm), "number");
        assert_eq!(map_type(&IrType::Boolean, &tm), "boolean");
        assert_eq!(map_type(&IrType::Null, &tm), "null");
        assert_eq!(map_type(&IrType::Any, &tm), "unknown");
        assert_eq!(map_type(&IrType::Void, &tm), "void");
    }

    #[test]
    fn ts_array() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(&IrType::Array(Box::new(IrType::String)), &tm),
            "string[]"
        );
        assert_eq!(
            map_type(
                &IrType::Array(Box::new(IrType::Union(vec![
                    IrType::String,
                    IrType::Number,
                ]))),
                &tm
            ),
            "(string | number)[]"
        );
    }

    #[test]
    fn ts_map() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(&IrType::Map(Box::new(IrType::String)), &tm),
            "Record<string, string>"
        );
    }

    #[test]
    fn ts_ref() {
        let tm = ts_type_map();
        assert_eq!(map_type(&IrType::Ref("Pet".to_string()), &tm), "Pet");
    }

    #[test]
    fn ts_union() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(&IrType::Union(vec![IrType::String, IrType::Number]), &tm),
            "string | number"
        );
    }

    #[test]
    fn ts_object() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(
                &IrType::Object(vec![
                    ("name".into(), IrType::String, true),
                    ("age".into(), IrType::Number, false),
                ]),
                &tm
            ),
            "{ name: string; age?: number }"
        );
        assert_eq!(
            map_type(&IrType::Object(vec![]), &tm),
            "Record<string, unknown>"
        );
    }

    #[test]
    fn py_primitives() {
        let tm = py_type_map();
        assert_eq!(map_type(&IrType::String, &tm), "str");
        assert_eq!(map_type(&IrType::Number, &tm), "float");
        assert_eq!(map_type(&IrType::Integer, &tm), "int");
        assert_eq!(map_type(&IrType::Boolean, &tm), "bool");
        assert_eq!(map_type(&IrType::Null, &tm), "None");
        assert_eq!(map_type(&IrType::Any, &tm), "Any");
        assert_eq!(map_type(&IrType::Void, &tm), "None");
    }

    #[test]
    fn py_array() {
        let tm = py_type_map();
        assert_eq!(
            map_type(&IrType::Array(Box::new(IrType::String)), &tm),
            "list[str]"
        );
    }

    #[test]
    fn py_map() {
        let tm = py_type_map();
        assert_eq!(
            map_type(&IrType::Map(Box::new(IrType::String)), &tm),
            "dict[str, str]"
        );
    }

    #[test]
    fn py_ref() {
        let tm = py_type_map();
        assert_eq!(map_type(&IrType::Ref("Pet".to_string()), &tm), "Pet");
    }

    #[test]
    fn py_optional_field() {
        let tm = py_type_map();
        assert_eq!(map_field_type(&IrType::String, true, &tm), "str");
        assert_eq!(
            map_field_type(&IrType::String, false, &tm),
            "str | None = None"
        );
    }

    #[test]
    fn ts_string_literal() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(&IrType::StringLiteral("hello".into()), &tm),
            "\"hello\""
        );
    }

    #[test]
    fn py_string_literal() {
        let tm = py_type_map();
        assert_eq!(
            map_type(&IrType::StringLiteral("hello".into()), &tm),
            "Literal[\"hello\"]"
        );
    }

    #[test]
    fn ts_integer_literal() {
        let tm = ts_type_map();
        assert_eq!(map_type(&IrType::IntegerLiteral(42), &tm), "42");
    }

    #[test]
    fn py_integer_literal() {
        let tm = py_type_map();
        assert_eq!(map_type(&IrType::IntegerLiteral(42), &tm), "Literal[42]");
    }

    #[test]
    fn ts_intersection() {
        let tm = ts_type_map();
        assert_eq!(
            map_type(
                &IrType::Intersection(vec![
                    IrType::Ref("Base".into()),
                    IrType::Ref("Extra".into()),
                ]),
                &tm
            ),
            "Base & Extra"
        );
    }
}
