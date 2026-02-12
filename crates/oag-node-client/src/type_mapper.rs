use oag_core::ir::IrType;

/// Map an `IrType` to its TypeScript type string representation.
pub fn ir_type_to_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String => "string".to_string(),
        IrType::StringLiteral(s) => format!("\"{s}\""),
        IrType::Number => "number".to_string(),
        IrType::Integer => "number".to_string(),
        IrType::Boolean => "boolean".to_string(),
        IrType::Null => "null".to_string(),
        IrType::DateTime => "string".to_string(),
        IrType::Binary => "Blob".to_string(),
        IrType::Any => "unknown".to_string(),
        IrType::Void => "void".to_string(),
        IrType::Ref(name) => name.clone(),
        IrType::Array(inner) => {
            let inner_ts = ir_type_to_ts(inner);
            if inner_ts.contains('|') {
                format!("({inner_ts})[]")
            } else {
                format!("{inner_ts}[]")
            }
        }
        IrType::Map(value_type) => {
            let value_ts = ir_type_to_ts(value_type);
            format!("Record<string, {value_ts}>")
        }
        IrType::Object(fields) => {
            if fields.is_empty() {
                return "Record<string, unknown>".to_string();
            }
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(name, ty, required)| {
                    let ts_type = ir_type_to_ts(ty);
                    if *required {
                        format!("{name}: {ts_type}")
                    } else {
                        format!("{name}?: {ts_type}")
                    }
                })
                .collect();
            format!("{{ {} }}", field_strs.join("; "))
        }
        IrType::Union(variants) => {
            let variant_strs: Vec<String> = variants.iter().map(ir_type_to_ts).collect();
            variant_strs.join(" | ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives() {
        assert_eq!(ir_type_to_ts(&IrType::String), "string");
        assert_eq!(ir_type_to_ts(&IrType::Number), "number");
        assert_eq!(ir_type_to_ts(&IrType::Integer), "number");
        assert_eq!(ir_type_to_ts(&IrType::Boolean), "boolean");
        assert_eq!(ir_type_to_ts(&IrType::Null), "null");
        assert_eq!(ir_type_to_ts(&IrType::Any), "unknown");
        assert_eq!(ir_type_to_ts(&IrType::Void), "void");
    }

    #[test]
    fn test_array() {
        assert_eq!(
            ir_type_to_ts(&IrType::Array(Box::new(IrType::String))),
            "string[]"
        );
        assert_eq!(
            ir_type_to_ts(&IrType::Array(Box::new(IrType::Union(vec![
                IrType::String,
                IrType::Number,
            ])))),
            "(string | number)[]"
        );
    }

    #[test]
    fn test_map() {
        assert_eq!(
            ir_type_to_ts(&IrType::Map(Box::new(IrType::String))),
            "Record<string, string>"
        );
    }

    #[test]
    fn test_ref() {
        assert_eq!(ir_type_to_ts(&IrType::Ref("Pet".to_string())), "Pet");
    }

    #[test]
    fn test_union() {
        assert_eq!(
            ir_type_to_ts(&IrType::Union(vec![IrType::String, IrType::Number])),
            "string | number"
        );
    }
}
