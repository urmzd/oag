use minijinja::{Environment, context};
use oag_core::ir::{IrSchema, IrSpec, IrType};

/// Emit `guards.ts` containing runtime type guard functions for discriminated unions.
///
/// Only unions with an explicit `discriminator` are included; non-discriminated
/// unions are skipped because structural checks are fragile and error-prone.
pub fn emit_guards(ir: &IrSpec) -> String {
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.add_template("guards.ts.j2", include_str!("../../templates/guards.ts.j2"))
        .expect("template should be valid");
    let tmpl = env.get_template("guards.ts.j2").unwrap();

    let mut guards = Vec::new();
    let mut import_names: Vec<String> = Vec::new();

    for schema in &ir.schemas {
        if let IrSchema::Union(u) = schema {
            if let Some(disc) = &u.discriminator {
                let union_name = u.name.pascal_case.clone();
                if !import_names.contains(&union_name) {
                    import_names.push(union_name.clone());
                }

                for (disc_value, schema_name) in &disc.mapping {
                    // Only emit guard for Ref variants that exist in the union
                    if u.variants
                        .iter()
                        .any(|v| matches!(v, IrType::Ref(n) if n == schema_name))
                    {
                        if !import_names.contains(schema_name) {
                            import_names.push(schema_name.clone());
                        }
                        guards.push(context! {
                            union_name => union_name.clone(),
                            variant_name => schema_name.clone(),
                            property_name => disc.property_name.clone(),
                            discriminator_value => disc_value.clone(),
                        });
                    }
                }
            }
        }
    }

    tmpl.render(context! {
        imports => import_names,
        guards => guards,
    })
    .expect("render should succeed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::{parse, transform};

    const PETSTORE_POLY: &str =
        include_str!("../../../oag-core/tests/fixtures/petstore-polymorphic.yaml");
    const ANTHROPIC: &str =
        include_str!("../../../oag-core/tests/fixtures/anthropic-messages.yaml");
    const PETSTORE: &str = include_str!("../../../oag-core/tests/fixtures/petstore-3.2.yaml");

    fn ir_from_yaml(yaml: &str) -> IrSpec {
        let spec = parse::from_yaml(yaml).unwrap();
        transform::transform(&spec).unwrap()
    }

    #[test]
    fn petstore_polymorphic_generates_pet_guards() {
        let ir = ir_from_yaml(PETSTORE_POLY);
        let output = emit_guards(&ir);

        assert!(output.contains("import type {"), "should have import statement");
        assert!(output.contains("Pet"), "should import Pet");
        assert!(output.contains("Cat"), "should import Cat");
        assert!(output.contains("Dog"), "should import Dog");

        assert!(
            output.contains("export function isCat(value: Pet): value is Cat"),
            "should have isCat guard"
        );
        assert!(
            output.contains(r#".petType === "cat""#),
            "isCat should check petType === \"cat\""
        );

        assert!(
            output.contains("export function isDog(value: Pet): value is Dog"),
            "should have isDog guard"
        );
        assert!(
            output.contains(r#".petType === "dog""#),
            "isDog should check petType === \"dog\""
        );
    }

    #[test]
    fn anthropic_generates_content_block_and_stream_delta_guards() {
        let ir = ir_from_yaml(ANTHROPIC);
        let output = emit_guards(&ir);

        // ContentBlock guards
        assert!(
            output.contains("export function isTextBlock(value: ContentBlock): value is TextBlock"),
            "should have isTextBlock guard"
        );
        assert!(
            output
                .contains("export function isImageBlock(value: ContentBlock): value is ImageBlock"),
            "should have isImageBlock guard"
        );
        assert!(
            output.contains(r#".type === "text""#),
            "isTextBlock should check type === \"text\""
        );
        assert!(
            output.contains(r#".type === "image""#),
            "isImageBlock should check type === \"image\""
        );

        // StreamDelta guards
        assert!(
            output.contains("export function isTextDelta(value: StreamDelta): value is TextDelta"),
            "should have isTextDelta guard"
        );
        assert!(
            output.contains(
                "export function isInputJsonDelta(value: StreamDelta): value is InputJsonDelta"
            ),
            "should have isInputJsonDelta guard"
        );
    }

    #[test]
    fn petstore_32_generates_no_guards() {
        let ir = ir_from_yaml(PETSTORE);
        let output = emit_guards(&ir);

        assert!(
            !output.contains("export function"),
            "petstore-3.2 has no discriminated unions, should have no guards"
        );
        assert!(
            !output.contains("import type"),
            "should have no imports when no guards"
        );
    }
}
