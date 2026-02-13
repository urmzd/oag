use oag_core::ir::IrType;

/// Map an `IrType` to its Python type string representation.
pub fn ir_type_to_python(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String => "str".to_string(),
        IrType::StringLiteral(s) => format!("Literal[\"{s}\"]"),
        IrType::Number => "float".to_string(),
        IrType::Integer => "int".to_string(),
        IrType::Boolean => "bool".to_string(),
        IrType::Null => "None".to_string(),
        IrType::DateTime => "str".to_string(),
        IrType::Binary => "bytes".to_string(),
        IrType::Any => "Any".to_string(),
        IrType::Void => "None".to_string(),
        IrType::Ref(name) => name.clone(),
        IrType::Array(inner) => {
            let inner_py = ir_type_to_python(inner);
            format!("list[{inner_py}]")
        }
        IrType::Map(value_type) => {
            let value_py = ir_type_to_python(value_type);
            format!("dict[str, {value_py}]")
        }
        IrType::Object(fields) => {
            if fields.is_empty() {
                return "dict[str, Any]".to_string();
            }
            // Inline objects become dict[str, Any] in Python
            "dict[str, Any]".to_string()
        }
        IrType::Union(variants) => {
            let variant_strs: Vec<String> = variants.iter().map(ir_type_to_python).collect();
            variant_strs.join(" | ")
        }
        IrType::Intersection(parts) => {
            // Python doesn't have a native intersection type; use the first part as a fallback
            if parts.len() == 1 {
                ir_type_to_python(&parts[0])
            } else {
                // Multiple inheritance: tuple of base classes
                let part_strs: Vec<String> = parts.iter().map(ir_type_to_python).collect();
                part_strs.join(", ")
            }
        }
    }
}

/// Map an `IrType` to a Python type that's Optional if not required.
pub fn ir_type_to_python_field(ir_type: &IrType, required: bool) -> String {
    let base = ir_type_to_python(ir_type);
    if required {
        base
    } else {
        format!("{base} | None = None")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives() {
        assert_eq!(ir_type_to_python(&IrType::String), "str");
        assert_eq!(ir_type_to_python(&IrType::Number), "float");
        assert_eq!(ir_type_to_python(&IrType::Integer), "int");
        assert_eq!(ir_type_to_python(&IrType::Boolean), "bool");
        assert_eq!(ir_type_to_python(&IrType::Null), "None");
        assert_eq!(ir_type_to_python(&IrType::Any), "Any");
        assert_eq!(ir_type_to_python(&IrType::Void), "None");
    }

    #[test]
    fn test_array() {
        assert_eq!(
            ir_type_to_python(&IrType::Array(Box::new(IrType::String))),
            "list[str]"
        );
    }

    #[test]
    fn test_map() {
        assert_eq!(
            ir_type_to_python(&IrType::Map(Box::new(IrType::String))),
            "dict[str, str]"
        );
    }

    #[test]
    fn test_ref() {
        assert_eq!(ir_type_to_python(&IrType::Ref("Pet".to_string())), "Pet");
    }

    #[test]
    fn test_union() {
        assert_eq!(
            ir_type_to_python(&IrType::Union(vec![IrType::String, IrType::Integer])),
            "str | int"
        );
    }

    #[test]
    fn test_optional_field() {
        assert_eq!(ir_type_to_python_field(&IrType::String, true), "str");
        assert_eq!(
            ir_type_to_python_field(&IrType::String, false),
            "str | None = None"
        );
    }
}
