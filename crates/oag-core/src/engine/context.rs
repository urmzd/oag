use std::collections::HashSet;

use minijinja::{Value, context};

use crate::ir::{
    HttpMethod, IrEnumVariant, IrObjectSchema, IrOperation, IrParameterLocation, IrReturnType,
    IrSchema, IrSpec, IrSseEvent, IrType,
};

use super::pack::TypeMapConfig;
use super::type_map::{map_field_type, map_type};

struct ParamsResult {
    path_params: Vec<Value>,
    query_params: Vec<Value>,
    py_params: Vec<Value>,
    header_params_obj: String,
    has_body: bool,
    body_content_type: String,
    has_path_params: bool,
    has_query_params: bool,
    has_header_params: bool,
}

/// Build the universal template context for an IrSpec.
///
/// Returns a minijinja Value containing all data needed by any template pack.
pub fn build_context(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> Value {
    let schemas = build_schema_contexts(ir, tm, field_casing);
    let schema_names: HashSet<String> = ir
        .schemas
        .iter()
        .map(|s| match s {
            IrSchema::Object(o) => o.name.pascal_case.clone(),
            IrSchema::Enum(e) => e.name.pascal_case.clone(),
            IrSchema::Alias(a) => a.name.pascal_case.clone(),
            IrSchema::Union(u) => u.name.pascal_case.clone(),
        })
        .collect();
    let sse_event_types = collect_sse_event_types(ir, &schema_names, tm);
    let guards = build_guard_contexts(ir);
    let (content_codecs, codec_imports) = build_content_codec_contexts(ir, tm);
    let param_types = build_param_type_contexts(ir, tm, field_casing);

    // Build operations with deduplication
    let (operations, used_op_indices) =
        build_operation_contexts(ir, tm, field_casing, operation_casing);

    // Build hooks (React-specific, but included universally — templates ignore if unused)
    let (hooks, hook_op_indices) = build_hook_contexts(ir, tm, operation_casing);

    // Collect imported types from surviving operations
    let imported_types = with_params_types(
        ir,
        collect_imported_types(
            ir.operations
                .iter()
                .enumerate()
                .filter(|(i, _)| used_op_indices.contains(i))
                .map(|(_, op)| op),
            tm,
        ),
        &used_op_indices,
    );

    let hook_imported_types = with_params_types(
        ir,
        collect_imported_types(
            ir.operations
                .iter()
                .enumerate()
                .filter(|(i, _)| hook_op_indices.contains(i))
                .map(|(_, op)| op),
            tm,
        ),
        &hook_op_indices,
    );

    // Model imports for Python-style generators (Ref names from operations)
    let model_imports = collect_model_imports(ir);

    let has_sse = operations.iter().any(|op| {
        op.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });
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
    let has_hook_sse = hooks.iter().any(|h| {
        h.get_attr("kind")
            .ok()
            .is_some_and(|v| v.as_str() == Some("sse"))
    });
    // Which SSE runtime entry points the client and hooks use: `streamSse`
    // with `SSEEvent<T>` (payload item schemas), or `streamSseEvents` (item
    // schemas describing whole events).
    let sse_attr = |list: &[Value], kind: &str, union: bool| {
        list.iter().any(|v| {
            v.get_attr("kind")
                .ok()
                .is_some_and(|k| k.as_str() == Some(kind))
                && v.get_attr("yield_type")
                    .ok()
                    .and_then(|y| y.as_str().map(|y| !y.starts_with("SSEEvent<")))
                    == Some(union)
        })
    };
    let uses_sse_payload = sse_attr(&operations, "sse", false);
    let uses_sse_event_union = sse_attr(&operations, "sse", true);
    let hooks_use_sse_payload = sse_attr(&hooks, "sse", false);

    // Test contexts
    let (test_operations, _test_op_indices) =
        build_test_operation_contexts(ir, tm, operation_casing);
    let test_type_imports = collect_test_type_imports(ir, &test_operations);
    let (py_test_operations, py_model_imports) = build_python_test_contexts(ir);
    let hook_names = build_hook_names(ir);

    // Extract guard sub-fields for flat template access
    let guard_imports = guards
        .get_attr("imports")
        .unwrap_or(Value::from(Vec::<String>::new()));
    let guard_items = guards
        .get_attr("guards")
        .unwrap_or(Value::from(Vec::<Value>::new()));

    context! {
        title => ir.info.title.clone(),
        schemas => schemas,
        sse_event_types => sse_event_types,
        // guards.ts.j2 expects top-level `imports` and `guards`
        imports => guard_imports,
        guards => guard_items,
        content_codecs => content_codecs,
        param_types => param_types,
        codec_imports => codec_imports,
        operations => operations,
        hooks => hooks,
        imported_types => imported_types,
        hook_imported_types => hook_imported_types,
        model_imports => model_imports,
        has_sse => has_sse,
        has_queries => has_queries,
        has_mutations => has_mutations,
        has_hook_sse => has_hook_sse,
        uses_sse_payload => uses_sse_payload,
        uses_sse_event_union => uses_sse_event_union,
        hooks_use_sse_payload => hooks_use_sse_payload,
        // Test contexts
        test_operations => test_operations,
        test_type_imports => test_type_imports,
        py_test_operations => py_test_operations,
        py_model_imports => py_model_imports,
        hook_names => hook_names,
    }
}

// ─── Schema contexts ─────────────────────────────────────────────

fn build_schema_contexts(ir: &IrSpec, tm: &TypeMapConfig, field_casing: &str) -> Vec<Value> {
    ir.schemas
        .iter()
        .map(|s| schema_to_ctx(s, tm, field_casing))
        .collect()
}

fn schema_to_ctx(schema: &IrSchema, tm: &TypeMapConfig, field_casing: &str) -> Value {
    match schema {
        IrSchema::Object(obj) => object_to_ctx(obj, tm, field_casing),
        IrSchema::Enum(e) => {
            let is_integer = !e.variants.is_empty()
                && e.variants
                    .iter()
                    .all(|v| matches!(v, IrEnumVariant::Integer(_)));

            // TS-style: literal string values
            let ts_variants: Vec<String> = e
                .variants
                .iter()
                .map(|v| match v {
                    IrEnumVariant::String(s) => format!("\"{s}\""),
                    IrEnumVariant::Integer(i) => i.to_string(),
                })
                .collect();

            // Python-style: name/value pairs
            let py_variants: Vec<Value> = e
                .variants
                .iter()
                .map(|v| match v {
                    IrEnumVariant::String(s) => context! {
                        name => heck::AsUpperCamelCase(s).to_string(),
                        value => format!("\"{}\"", s),
                    },
                    IrEnumVariant::Integer(i) => {
                        let name = if *i < 0 {
                            format!("Neg{}", i.unsigned_abs())
                        } else {
                            format!("Value{i}")
                        };
                        context! {
                            name => name,
                            value => i.to_string(),
                        }
                    }
                })
                .collect();

            context! {
                kind => "enum",
                name => e.name.pascal_case.clone(),
                description => e.description.clone(),
                variants => ts_variants,
                py_variants => py_variants,
                is_integer => is_integer,
            }
        }
        IrSchema::Alias(a) => {
            context! {
                kind => "alias",
                name => a.name.pascal_case.clone(),
                description => a.description.clone(),
                target => map_type(&a.target, tm),
            }
        }
        IrSchema::Union(u) => {
            let variants: Vec<String> = u.variants.iter().map(|v| map_type(v, tm)).collect();
            context! {
                kind => "union",
                name => u.name.pascal_case.clone(),
                description => u.description.clone(),
                variants => variants,
            }
        }
    }
}

/// Render a literal `IrType` as a raw value usable as a default initializer in
/// both TypeScript and Python (e.g. `StringLiteral("a")` -> `"a"`,
/// `IntegerLiteral(1)` -> `1`). JSON quoting is valid in both languages.
///
/// `default_repr` is only ever populated with string/integer literals (see
/// `schema_default_literal`), so other variants are not expected; they render
/// `None` to stay safe rather than emit an invalid initializer.
fn literal_value(t: &IrType) -> Option<String> {
    match t {
        IrType::StringLiteral(s) => {
            Some(serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\"")))
        }
        IrType::IntegerLiteral(i) => Some(i.to_string()),
        _ => None,
    }
}

fn object_to_ctx(obj: &IrObjectSchema, tm: &TypeMapConfig, field_casing: &str) -> Value {
    let fields: Vec<Value> = obj
        .fields
        .iter()
        .map(|f| {
            let field_name = match field_casing {
                "snake" => f.name.snake_case.clone(),
                "pascal" => f.name.pascal_case.clone(),
                _ => f.name.camel_case.clone(),
            };
            let type_str = map_type(&f.field_type, tm);
            let field_type_str = map_field_type(&f.field_type, f.required, tm);
            // A literal field (a `const` or single-value `enum`) has exactly one
            // possible value, so it is always present — packs render it as a
            // required field carrying that literal. `default_value` is the raw
            // literal value (e.g. `"message"`, `1`) for use as a default
            // initializer; `is_literal` lets a template detect this case.
            let is_literal = matches!(
                f.field_type,
                IrType::StringLiteral(_) | IrType::IntegerLiteral(_)
            );
            let default_value = f.default_repr.as_ref().and_then(literal_value);
            let content = f.content().map(|(c, is_array)| {
                context! {
                    codec => c.codec(),
                    type_str => map_type(&c.schema, tm),
                    helper => codec_helper_name(obj, f, is_array),
                    is_array => is_array,
                    encoding => c.encoding.clone(),
                    media_type => c.media_type.clone(),
                }
            });
            context! {
                name => field_name,
                original_name => f.original_name.clone(),
                type_str => type_str,
                field_type_str => field_type_str,
                required => f.required,
                is_literal => is_literal,
                description => f.description.clone(),
                needs_alias => f.name.snake_case != f.original_name,
                default_value => default_value,
                content => content,
            }
        })
        .collect();

    let additional = obj.additional_properties.as_ref().map(|t| map_type(t, tm));
    let has_additional_properties = obj.additional_properties.is_some();

    context! {
        kind => "object",
        name => obj.name.pascal_case.clone(),
        description => obj.description.clone(),
        fields => fields,
        additional_properties => additional,
        has_additional_properties => has_additional_properties,
    }
}

fn collect_sse_event_types(
    ir: &IrSpec,
    schema_names: &HashSet<String>,
    tm: &TypeMapConfig,
) -> Vec<Value> {
    let mut event_types = Vec::new();
    let mut seen = HashSet::new();
    for op in &ir.operations {
        if let IrReturnType::Sse(sse) = &op.return_type
            && let Some(ref event_name) = sse.event_type_name
        {
            if seen.contains(event_name) || schema_names.contains(event_name) {
                continue;
            }
            if !sse.events.is_empty() {
                // OAS 3.2 item schema describing the whole event: a union of
                // `{ event, data, id?, retry? }` keyed by the event name.
                seen.insert(event_name.clone());
                event_types.push(context! {
                    name => event_name.clone(),
                    is_event_union => true,
                    variants => sse.events.iter().map(|e| map_type(&sse_event_object(e), tm)).collect::<Vec<_>>(),
                    data_variants => sse.events.iter().map(|e| map_type(&e.data_type, tm)).collect::<Vec<_>>(),
                    events => sse.events.iter().map(|e| context! {
                        names => e.names.clone(),
                        data_type => map_type(&e.data_type, tm),
                        codec => e.data_codec.clone(),
                        description => e.description.clone(),
                    }).collect::<Vec<_>>(),
                });
                continue;
            }
            let variants: Vec<String> = sse.variants.iter().map(|v| map_type(v, tm)).collect();
            if !variants.is_empty() {
                seen.insert(event_name.clone());
                event_types.push(context! {
                    name => event_name.clone(),
                    is_event_union => false,
                    variants => variants,
                });
            }
        }
    }
    event_types
}

/// The yielded shape of one SSE event variant:
/// `{ event: "name"; data: T; id?: string; retry?: number }`.
fn sse_event_object(event: &IrSseEvent) -> IrType {
    let name_type = match event.names.as_slice() {
        [] => IrType::String,
        [one] => IrType::StringLiteral(one.clone()),
        many => IrType::Union(
            many.iter()
                .map(|n| IrType::StringLiteral(n.clone()))
                .collect(),
        ),
    };
    IrType::Object(vec![
        ("event".to_string(), name_type, true),
        ("data".to_string(), event.data_type.clone(), true),
        ("id".to_string(), IrType::String, false),
        ("retry".to_string(), IrType::Integer, false),
    ])
}

/// The decoder table passed to the SSE runtime for a typed event stream:
/// `{ "inbox": "json", "heartbeat": "text" }` (`"*"` matches any name).
fn sse_event_codecs(events: &[IrSseEvent]) -> String {
    let mut entries: Vec<String> = Vec::new();
    for event in events {
        let keys: Vec<String> = if event.names.is_empty() {
            vec!["*".to_string()]
        } else {
            event.names.clone()
        };
        for key in keys {
            let quoted = serde_json::to_string(&key).unwrap_or_else(|_| format!("\"{key}\""));
            if !entries.iter().any(|e| e.starts_with(&format!("{quoted}:"))) {
                entries.push(format!("{quoted}: \"{}\"", event.data_codec));
            }
        }
    }
    format!("{{ {} }}", entries.join(", "))
}

// ─── Params object types ─────────────────────────────────────────

/// One `<Operation>Params` interface per operation with query or header
/// parameters, declared in the types file so clients and hooks share it.
fn build_param_type_contexts(ir: &IrSpec, tm: &TypeMapConfig, field_casing: &str) -> Vec<Value> {
    let mut seen = HashSet::new();
    ir.operations
        .iter()
        .filter_map(|op| {
            build_op_args(ir, op, tm, field_casing)
                .params_type
                .map(|pt| (op, pt))
        })
        .filter(|(_, pt)| seen.insert(pt.name.clone()))
        .map(|(op, pt)| {
            context! {
                name => pt.name,
                operation => op.name.camel_case.clone(),
                fields => pt.fields,
            }
        })
        .collect()
}

// ─── Content codec contexts ──────────────────────────────────────

/// `<Schema><Field>`, or `<Schema><Field>Item` for an array of encoded strings
/// (the helper decodes one item).
fn codec_helper_name(obj: &IrObjectSchema, field: &crate::ir::IrField, is_array: bool) -> String {
    let suffix = if is_array { "Item" } else { "" };
    format!("{}{}{suffix}", obj.name.pascal_case, field.name.pascal_case)
}

/// Typed decode/encode helpers for string fields that carry encoded content
/// (`contentEncoding` / `contentMediaType` / `contentSchema`), e.g. a
/// base64-encoded JSON payload. Returns the helper contexts and the type names
/// they reference.
fn build_content_codec_contexts(ir: &IrSpec, tm: &TypeMapConfig) -> (Vec<Value>, Vec<String>) {
    let mut codecs = Vec::new();
    let mut refs = HashSet::new();
    for schema in &ir.schemas {
        let IrSchema::Object(obj) = schema else {
            continue;
        };
        for field in &obj.fields {
            let Some((content, is_array)) = field.content() else {
                continue;
            };
            collect_refs_from_ir_type(&content.schema, &mut refs);
            let is_model = matches!(
                ir.find_schema(match &content.schema {
                    IrType::Ref(n) => n.as_str(),
                    _ => "",
                }),
                Some(IrSchema::Object(_))
            );
            let helper = codec_helper_name(obj, field, is_array);
            codecs.push(context! {
                is_array => is_array,
                schema_name => obj.name.pascal_case.clone(),
                field_name => field.original_name.clone(),
                helper_pascal => helper.clone(),
                helper_snake => heck::AsSnakeCase(&helper).to_string(),
                type_str => map_type(&content.schema, tm),
                codec => content.codec(),
                encoding => content.encoding.clone(),
                media_type => content.media_type.clone().unwrap_or_else(|| "application/json".to_string()),
                is_model => is_model,
            });
        }
    }
    let mut imports: Vec<String> = refs.into_iter().collect();
    imports.sort();
    (codecs, imports)
}

// ─── Guard contexts ──────────────────────────────────────────────

/// One type guard per variant schema.
///
/// A discriminator `mapping` may send several values to the same schema
/// (`lifecycle.started` and `lifecycle.completed` both to `Lifecycle`), and the
/// same schema may be a variant of more than one discriminated union. Emitting
/// a guard per mapping entry produced duplicate `isLifecycle` functions that do
/// not compile, so guards are grouped by variant: the guard accepts any of the
/// unions it belongs to and checks any of its mapped values.
fn build_guard_contexts(ir: &IrSpec) -> Value {
    struct Guard {
        variant_name: String,
        union_names: Vec<String>,
        /// (property name, rendered literal, is integer)
        checks: Vec<(String, String, bool)>,
    }

    let mut guards: Vec<Guard> = Vec::new();
    let mut import_names: Vec<String> = Vec::new();

    for schema in &ir.schemas {
        if let IrSchema::Union(u) = schema
            && let Some(disc) = &u.discriminator
        {
            let union_name = u.name.pascal_case.clone();

            for (disc_value, schema_name) in &disc.mapping {
                if !u
                    .variants
                    .iter()
                    .any(|v| matches!(v, IrType::Ref(n) if n == schema_name))
                {
                    continue;
                }
                if !import_names.contains(&union_name) {
                    import_names.push(union_name.clone());
                }
                if !import_names.contains(schema_name) {
                    import_names.push(schema_name.clone());
                }
                let is_integer = disc_value.parse::<i64>().is_ok();
                let literal = if is_integer {
                    disc_value.clone()
                } else {
                    serde_json::to_string(disc_value)
                        .unwrap_or_else(|_| format!("\"{disc_value}\""))
                };
                let check = (disc.property_name.clone(), literal, is_integer);

                let guard = match guards.iter_mut().find(|g| &g.variant_name == schema_name) {
                    Some(g) => g,
                    None => {
                        guards.push(Guard {
                            variant_name: schema_name.clone(),
                            union_names: Vec::new(),
                            checks: Vec::new(),
                        });
                        guards.last_mut().unwrap()
                    }
                };
                if !guard.union_names.contains(&union_name) {
                    guard.union_names.push(union_name.clone());
                }
                if !guard.checks.contains(&check) {
                    guard.checks.push(check);
                }
            }
        }
    }

    let guards: Vec<Value> = guards
        .into_iter()
        .map(|g| {
            let (first_prop, first_literal, first_is_int) = g.checks[0].clone();
            let checks: Vec<Value> = g
                .checks
                .iter()
                .map(|(prop, literal, is_int)| {
                    context! {
                        property_name => prop.clone(),
                        literal => literal.clone(),
                        is_integer_discriminator => *is_int,
                    }
                })
                .collect();
            context! {
                union_name => g.union_names.join(" | "),
                variant_name => g.variant_name,
                checks => checks,
                // First mapping entry, kept for packs that predate `checks`.
                property_name => first_prop,
                discriminator_value => first_literal.trim_matches('"').to_string(),
                is_integer_discriminator => first_is_int,
            }
        })
        .collect();

    context! {
        imports => import_names,
        guards => guards,
    }
}

// ─── Operation contexts ──────────────────────────────────────────

fn op_name(op: &IrOperation, casing: &str) -> String {
    match casing {
        "snake" => op.name.snake_case.clone(),
        "pascal" => op.name.pascal_case.clone(),
        _ => op.name.camel_case.clone(),
    }
}

fn build_operation_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_methods = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_operation_contexts(ir, op, tm, field_casing, operation_casing)
                .into_iter()
                .map(move |ctx| (idx, ctx))
        })
        .filter(|(idx, ctx)| {
            let name = ctx
                .get_attr("method_name")
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            match name {
                Some(n) => {
                    if seen_methods.insert(n) {
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

    (operations, used_op_indices)
}

/// Classify how a parameter value is serialized. `$ref`s are resolved first so a
/// reference to a string enum (or an alias of a primitive) serializes exactly
/// like the inline schema instead of being walked as an object.
fn classify_param_type(ir_type: &IrType, ir: &IrSpec) -> &'static str {
    match ir.resolve_type(ir_type).as_ref() {
        IrType::Array(_) => "array",
        IrType::Map(_) => "map",
        IrType::Object(_) | IrType::Ref(_) => "object",
        _ => "primitive",
    }
}

// ─── Argument model ──────────────────────────────────────────────

/// Kind of a positional argument of a generated TypeScript method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    Path,
    Body,
    Params,
}

/// One positional argument of a generated TypeScript method.
#[derive(Debug, Clone)]
struct OpArg {
    name: String,
    /// The argument's type (without `| undefined`).
    type_str: String,
    required: bool,
    kind: ArgKind,
    /// A mock value usable in generated tests.
    mock: String,
}

impl OpArg {
    fn sig(&self) -> String {
        if self.required {
            format!("{}: {}", self.name, self.type_str)
        } else {
            format!("{}?: {}", self.name, self.type_str)
        }
    }
}

/// The positional arguments of an operation, plus its params object type.
///
/// Path parameters stay positional (they are always required and ordered by
/// the path). Query and header parameters are grouped into a single params
/// object (`<Operation>Params`) so a long list of optional parameters is not a
/// list of positional arguments. Required arguments come first (path, body,
/// params), then optional ones (body, params); `options` always comes last.
struct OpArgs {
    args: Vec<OpArg>,
    params_type: Option<ParamsType>,
}

struct ParamsType {
    name: String,
    required: bool,
    fields: Vec<Value>,
}

impl OpArgs {
    fn signature(&self, options_type: &str) -> String {
        let mut parts: Vec<String> = self.args.iter().map(OpArg::sig).collect();
        parts.push(format!("options?: {options_type}"));
        parts.join(", ")
    }

    fn names(&self, body_name: &str) -> Vec<String> {
        self.args
            .iter()
            .map(|a| {
                if a.kind == ArgKind::Body {
                    body_name.to_string()
                } else {
                    a.name.clone()
                }
            })
            .collect()
    }

    fn non_body(&self) -> impl Iterator<Item = &OpArg> {
        self.args.iter().filter(|a| a.kind != ArgKind::Body)
    }

    /// Mock values for the required arguments and the body (a valid call that
    /// sends a body). An optional body is the first optional argument, so the
    /// mocked arguments stay contiguous.
    fn test_call_args(&self) -> String {
        self.args
            .iter()
            .filter(|a| a.required || a.kind == ArgKind::Body)
            .map(|a| a.mock.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Mock values for every positional argument, with `undefined` for the
    /// optional ones, followed by `, ` when non-empty: a prefix for passing
    /// `options` explicitly.
    fn test_args_before_options(&self) -> String {
        let parts: Vec<String> = self
            .args
            .iter()
            .map(|a| {
                if a.required || a.kind == ArgKind::Body {
                    a.mock.clone()
                } else {
                    "undefined".to_string()
                }
            })
            .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!("{}, ", parts.join(", "))
        }
    }
}

fn param_ident(param: &crate::ir::IrParameter, field_casing: &str) -> String {
    match field_casing {
        "snake" => param.name.snake_case.clone(),
        "pascal" => param.name.pascal_case.clone(),
        _ => param.name.camel_case.clone(),
    }
}

/// Name of an operation's params object type. Falls back to
/// `<Operation>RequestParams` when a schema already uses `<Operation>Params`.
fn params_type_name(ir: &IrSpec, op: &IrOperation) -> String {
    let name = format!("{}Params", op.name.pascal_case);
    if ir.find_schema(&name).is_some() {
        format!("{}RequestParams", op.name.pascal_case)
    } else {
        name
    }
}

fn is_params_location(location: IrParameterLocation) -> bool {
    matches!(
        location,
        IrParameterLocation::Query | IrParameterLocation::Header
    )
}

fn build_op_args(ir: &IrSpec, op: &IrOperation, tm: &TypeMapConfig, field_casing: &str) -> OpArgs {
    let mut required = Vec::new();
    let mut optional = Vec::new();

    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            required.push(OpArg {
                name: param_ident(param, field_casing),
                type_str: map_type(&param.param_type, tm),
                required: true,
                kind: ArgKind::Path,
                mock: mock_value_ts(&ir.resolve_type(&param.param_type)),
            });
        }
    }

    let body_arg = op.request_body.as_ref().map(|body| OpArg {
        name: "body".to_string(),
        type_str: map_type(&body.body_type, tm),
        required: body.required,
        kind: ArgKind::Body,
        mock: mock_value_ts(&body.body_type),
    });

    let grouped: Vec<&crate::ir::IrParameter> = op
        .parameters
        .iter()
        .filter(|p| is_params_location(p.location))
        .collect();
    let params_type = if grouped.is_empty() {
        None
    } else {
        let fields: Vec<Value> = grouped
            .iter()
            .map(|p| {
                context! {
                    name => param_ident(p, field_casing),
                    original_name => p.original_name.clone(),
                    type_str => map_type(&p.param_type, tm),
                    required => p.required,
                    location => if p.location == IrParameterLocation::Header { "header" } else { "query" },
                    description => p.description.clone(),
                }
            })
            .collect();
        Some(ParamsType {
            name: params_type_name(ir, op),
            required: grouped.iter().any(|p| p.required),
            fields,
        })
    };
    let params_arg = params_type.as_ref().map(|pt| {
        let members: Vec<String> = grouped
            .iter()
            .filter(|p| p.required)
            .map(|p| {
                format!(
                    "{}: {}",
                    param_ident(p, field_casing),
                    mock_value_ts(&ir.resolve_type(&p.param_type))
                )
            })
            .collect();
        OpArg {
            name: "params".to_string(),
            type_str: pt.name.clone(),
            required: pt.required,
            kind: ArgKind::Params,
            mock: if members.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {} }}", members.join(", "))
            },
        }
    });

    for arg in [body_arg, params_arg].into_iter().flatten() {
        if arg.required {
            required.push(arg);
        } else {
            optional.push(arg);
        }
    }
    required.extend(optional);

    OpArgs {
        args: required,
        params_type,
    }
}

/// Expression reading a grouped parameter out of the params object.
fn params_expr(op_args: &OpArgs, ident: &str) -> String {
    match &op_args.params_type {
        Some(pt) if pt.required => format!("params.{ident}"),
        _ => format!("params?.{ident}"),
    }
}

/// Build structured parameter data for an operation.
fn build_params(
    ir: &IrSpec,
    op: &IrOperation,
    tm: &TypeMapConfig,
    field_casing: &str,
    op_args: &OpArgs,
) -> ParamsResult {
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();
    let mut header_parts = Vec::new();

    for param in &op.parameters {
        let ident = param_ident(param, field_casing);
        match param.location {
            IrParameterLocation::Path => {
                path_params.push(context! {
                    name => ident,
                    original_name => param.original_name.clone(),
                });
            }
            IrParameterLocation::Query => {
                let type_kind = classify_param_type(&param.param_type, ir);
                let style = param.style.clone().unwrap_or_else(|| "form".to_string());
                let explode = param.explode.unwrap_or(style == "form");
                query_params.push(context! {
                    // An expression reading the value (from the params object).
                    name => params_expr(op_args, &ident),
                    ident => ident,
                    original_name => param.original_name.clone(),
                    style => style,
                    explode => explode,
                    type_kind => type_kind,
                });
            }
            IrParameterLocation::Header => {
                header_parts.push(format!(
                    "\"{}\": {}",
                    param.original_name,
                    params_expr(op_args, &ident)
                ));
            }
            _ => {}
        }
    }

    let has_body = op.request_body.is_some();
    let body_content_type = op
        .request_body
        .as_ref()
        .map(|b| b.content_type.clone())
        .unwrap_or_else(|| "application/json".to_string());

    let has_path_params = !path_params.is_empty();
    let has_query_params = !query_params.is_empty();
    let has_header_params = !header_parts.is_empty();
    let header_params_obj = header_parts.join(", ");

    // Build Python-style params (structured, not pre-formatted)
    let py_params: Vec<Value> = op
        .parameters
        .iter()
        .map(|param| {
            let type_str = map_type(&param.param_type, tm);
            let location = match param.location {
                IrParameterLocation::Path => "path",
                IrParameterLocation::Query => "query",
                IrParameterLocation::Header => "header",
                IrParameterLocation::Cookie => "cookie",
            };
            context! {
                name => param_ident(param, field_casing),
                original_name => param.original_name.clone(),
                type_str => type_str,
                location => location,
                required => param.required,
                needs_alias => param.name.snake_case != param.original_name,
            }
        })
        .collect();

    ParamsResult {
        path_params,
        query_params,
        py_params,
        header_params_obj,
        has_body,
        body_content_type,
        has_path_params,
        has_query_params,
        has_header_params,
    }
}

fn is_multipart_op(op: &IrOperation) -> bool {
    op.request_body
        .as_ref()
        .is_some_and(|b| b.content_type == "multipart/form-data")
}

fn build_single_operation_contexts(
    ir: &IrSpec,
    op: &IrOperation,
    tm: &TypeMapConfig,
    field_casing: &str,
    operation_casing: &str,
) -> Vec<Value> {
    let mut results = Vec::new();
    let method_name = op_name(op, operation_casing);
    let http_method = op.method.as_str();
    let path = op.path.clone();
    let op_args = build_op_args(ir, op, tm, field_casing);

    // Build TS-style params
    let ParamsResult {
        path_params,
        query_params,
        py_params,
        header_params_obj,
        has_body,
        body_content_type,
        has_path_params,
        has_query_params,
        has_header_params,
    } = build_params(ir, op, tm, field_casing, &op_args);

    // Build params_signature (TS-style pre-formatted)
    let params_signature = op_args.signature("RequestOptions");
    let params_type = op_args.params_type.as_ref().map(|pt| pt.name.clone());

    let body_type = op
        .request_body
        .as_ref()
        .map(|b| map_type(&b.body_type, tm))
        .unwrap_or_default();
    let body_param_name = "body".to_string();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = map_type(&resp.response_type, tm);
            results.push(context! {
                kind => "standard",
                method_name => method_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => params_signature,
                params_type => params_type.clone(),
                return_type => return_type,
                status_code => resp.status_code,
                response_format => resp.format.as_str(),
                path_params => path_params,
                query_params => query_params,
                params => py_params,
                header_params_obj => header_params_obj,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                body_content_type => body_content_type,
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });
        }
        IrReturnType::Void => {
            results.push(context! {
                kind => "void",
                method_name => method_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => params_signature,
                params_type => params_type.clone(),
                return_type => map_type(&IrType::Void, tm),
                path_params => path_params,
                query_params => query_params,
                params => py_params,
                header_params_obj => header_params_obj,
                has_body => has_body,
                body_type => body_type,
                body_param_name => body_param_name,
                body_content_type => body_content_type,
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });
        }
        IrReturnType::Sse(sse) => {
            let return_type = if let Some(ref name) = sse.event_type_name {
                name.clone()
            } else {
                map_type(&sse.event_type, tm)
            };
            let sse_name = if sse.also_has_json {
                format!("{}Stream", op.name.camel_case)
            } else {
                op.name.camel_case.clone()
            };
            // What the stream yields: the typed event union when the item
            // schema describes whole events, else `SSEEvent<payload>`.
            let is_event_union = !sse.events.is_empty();
            let yield_type = if is_event_union {
                return_type.clone()
            } else {
                format!("SSEEvent<{return_type}>")
            };
            let event_codecs = if is_event_union {
                Some(sse_event_codecs(&sse.events))
            } else {
                None
            };

            // SSE params use SSEOptions instead of RequestOptions
            let sse_params_sig = op_args.signature("SSEOptions");

            results.push(context! {
                kind => "sse",
                method_name => sse_name,
                name => op.name.snake_case.clone(),
                http_method => http_method,
                path => path,
                params_signature => sse_params_sig,
                params_type => params_type.clone(),
                return_type => return_type,
                event_type => return_type,
                yield_type => yield_type,
                is_event_union => is_event_union,
                event_codecs => event_codecs,
                path_params => path_params,
                query_params => query_params.clone(),
                params => py_params.clone(),
                header_params_obj => header_params_obj.clone(),
                has_body => has_body,
                body_type => body_type.clone(),
                body_param_name => body_param_name.clone(),
                body_content_type => body_content_type.clone(),
                is_multipart => is_multipart_op(op),
                has_path_params => has_path_params,
                has_query_params => has_query_params,
                has_header_params => has_header_params,
                summary => op.summary.clone(),
                description => op.description.clone(),
                deprecated => op.deprecated,
            });

            if let Some(ref json_resp) = sse.json_response {
                let json_return_type = map_type(&json_resp.response_type, tm);
                let json_desc = format!(
                    "{} (JSON response)",
                    op.description.as_deref().unwrap_or("")
                );
                results.push(context! {
                    kind => "standard",
                    method_name => method_name,
                    name => op.name.snake_case.clone(),
                    http_method => http_method,
                    path => path,
                    params_signature => params_signature,
                    params_type => params_type.clone(),
                    return_type => json_return_type,
                    status_code => json_resp.status_code,
                    response_format => json_resp.format.as_str(),
                    path_params => path_params.clone(),
                    query_params => query_params,
                    params => py_params,
                    header_params_obj => header_params_obj,
                    has_body => has_body,
                    body_type => body_type,
                    body_param_name => body_param_name,
                    body_content_type => body_content_type,
                    is_multipart => is_multipart_op(op),
                    has_path_params => has_path_params,
                    has_query_params => has_query_params,
                    has_header_params => has_header_params,
                    summary => op.summary.clone(),
                    description => json_desc,
                    deprecated => op.deprecated,
                });
            }
        }
    }

    results
}

// ─── Hook contexts (React) ───────────────────────────────────────

fn build_hook_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    _operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_hooks = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let hooks: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_hook_contexts(ir, op, tm)
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

    (hooks, used_op_indices)
}

fn build_single_hook_contexts(ir: &IrSpec, op: &IrOperation, tm: &TypeMapConfig) -> Vec<Value> {
    let mut results = Vec::new();

    match (&op.method, &op.return_type) {
        // GET → useSWR query hook
        (HttpMethod::Get, IrReturnType::Standard(resp)) => {
            let return_type = map_type(&resp.response_type, tm);
            let (params_sig, swr_key, call_args) = build_hook_query_params(ir, op, tm);
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
        // POST/PUT/DELETE non-streaming → useSWRMutation
        (_, IrReturnType::Standard(_)) | (_, IrReturnType::Void) => {
            let return_type = match &op.return_type {
                IrReturnType::Standard(r) => map_type(&r.response_type, tm),
                _ => map_type(&IrType::Void, tm),
            };
            let has_body = op.request_body.is_some();
            let body_type = op
                .request_body
                .as_ref()
                .map(|b| map_type(&b.body_type, tm))
                .unwrap_or_else(|| map_type(&IrType::Void, tm));

            let (path_params_sig, swr_key, call_args, swr_key_type) =
                build_hook_mutation_params(ir, op, tm);
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
                map_type(&sse.event_type, tm)
            };
            let event_type_array = if event_type.contains('|') {
                format!("({event_type})[]")
            } else {
                format!("{event_type}[]")
            };
            let yield_type = if sse.events.is_empty() {
                format!("SSEEvent<{event_type}>")
            } else {
                event_type.clone()
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
                build_hook_sse_params(ir, op, tm);

            results.push(context! {
                kind => "sse",
                hook_name => hook_name,
                method_name => method_name,
                path_params_signature => path_params_sig,
                event_type => event_type,
                event_type_array => event_type_array,
                yield_type => yield_type,
                trigger_params => trigger_params,
                stream_call_args => stream_call_args,
                deps => deps,
                description => op.summary.clone().or(op.description.clone()),
            });

            // Dual endpoint: also generate JSON hook
            if let Some(ref json_resp) = sse.json_response {
                let return_type = map_type(&json_resp.response_type, tm);
                match op.method {
                    HttpMethod::Get => {
                        let (params_sig, swr_key, call_args) = build_hook_query_params(ir, op, tm);
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
                            .map(|b| map_type(&b.body_type, tm))
                            .unwrap_or_else(|| map_type(&IrType::Void, tm));
                        let (path_params_sig, swr_key, call_args, swr_key_type) =
                            build_hook_mutation_params(ir, op, tm);
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

/// Query hook arguments: the signature (all non-options arguments), the SWR key,
/// and the client call arguments.
fn build_hook_query_params(
    ir: &IrSpec,
    op: &IrOperation,
    tm: &TypeMapConfig,
) -> (String, String, String) {
    let args = build_op_args(ir, op, tm, "camel");
    let sig: Vec<String> = args.non_body().map(OpArg::sig).collect();
    let key_parts: Vec<String> = args.non_body().map(|a| a.name.clone()).collect();
    let swr_key = if key_parts.is_empty() {
        format!("\"{}\"", op.path)
    } else {
        format!("[\"{}\", {}] as const", op.path, key_parts.join(", "))
    };
    (sig.join(", "), swr_key, args.names("body").join(", "))
}

/// Mutation hook arguments: the signature (non-body arguments), the SWR key,
/// the client call arguments (the body comes from the trigger `arg`), and the
/// SWR key type.
fn build_hook_mutation_params(
    ir: &IrSpec,
    op: &IrOperation,
    tm: &TypeMapConfig,
) -> (String, String, String, String) {
    let args = build_op_args(ir, op, tm, "camel");
    let sig: Vec<String> = args.non_body().map(OpArg::sig).collect();
    let key_parts: Vec<String> = args.non_body().map(|a| a.name.clone()).collect();
    let key_type_parts: Vec<String> = args
        .non_body()
        .map(|a| {
            if a.required {
                a.type_str.clone()
            } else {
                format!("{} | undefined", a.type_str)
            }
        })
        .collect();

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

    (
        sig.join(", "),
        swr_key,
        args.names("arg").join(", "),
        swr_key_type,
    )
}

/// SSE hook arguments: the hook signature (non-body arguments), the trigger's
/// parameters (the body), the client call arguments, and the effect deps.
fn build_hook_sse_params(
    ir: &IrSpec,
    op: &IrOperation,
    tm: &TypeMapConfig,
) -> (String, String, String, String) {
    let args = build_op_args(ir, op, tm, "camel");
    let sig: Vec<String> = args.non_body().map(OpArg::sig).collect();
    let deps: String = args.non_body().map(|a| format!(", {}", a.name)).collect();
    let trigger_params = args
        .args
        .iter()
        .find(|a| a.kind == ArgKind::Body)
        .map(OpArg::sig)
        .unwrap_or_default();
    (
        sig.join(", "),
        trigger_params,
        args.names("body").join(", "),
        deps,
    )
}

fn build_hook_names(ir: &IrSpec) -> Vec<String> {
    let mut seen = HashSet::new();
    ir.operations
        .iter()
        .flat_map(|op| {
            let mut names = Vec::new();
            match &op.return_type {
                IrReturnType::Sse(sse) if sse.also_has_json => {
                    names.push(format!("use{}Stream", op.name.pascal_case));
                    names.push(format!("use{}", op.name.pascal_case));
                }
                _ => {
                    names.push(format!("use{}", op.name.pascal_case));
                }
            }
            names
        })
        .filter(|n| seen.insert(n.clone()))
        .collect()
}

// ─── Import collection ───────────────────────────────────────────

/// Add the params object types of the given operations to an import list.
fn with_params_types(
    ir: &IrSpec,
    mut types: Vec<String>,
    op_indices: &HashSet<usize>,
) -> Vec<String> {
    for (i, op) in ir.operations.iter().enumerate() {
        if op_indices.contains(&i) && op.parameters.iter().any(|p| is_params_location(p.location)) {
            let name = params_type_name(ir, op);
            if !types.contains(&name) {
                types.push(name);
            }
        }
    }
    types.sort();
    types
}

fn collect_imported_types<'a>(
    ops: impl Iterator<Item = &'a IrOperation>,
    _tm: &TypeMapConfig,
) -> Vec<String> {
    let mut types = HashSet::new();

    for op in ops {
        collect_types_from_return(&op.return_type, &mut types);
        if let Some(ref body) = op.request_body {
            collect_refs_from_ir_type(&body.body_type, &mut types);
        }
        // Query and header parameters live in the params object type (see
        // `with_params_types`); only positional path parameters are referenced
        // directly by the client and hooks.
        for param in op
            .parameters
            .iter()
            .filter(|p| p.location == IrParameterLocation::Path)
        {
            collect_refs_from_ir_type(&param.param_type, &mut types);
        }
    }

    let mut sorted: Vec<String> = types.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_types_from_return(ret: &IrReturnType, types: &mut HashSet<String>) {
    match ret {
        IrReturnType::Standard(resp) => {
            collect_refs_from_ir_type(&resp.response_type, types);
        }
        IrReturnType::Sse(sse) => {
            if let Some(ref name) = sse.event_type_name {
                types.insert(name.clone());
            } else {
                collect_refs_from_ir_type(&sse.event_type, types);
            }
            if let Some(ref json) = sse.json_response {
                collect_refs_from_ir_type(&json.response_type, types);
            }
        }
        IrReturnType::Void => {}
    }
}

fn collect_refs_from_ir_type(ir_type: &IrType, types: &mut HashSet<String>) {
    match ir_type {
        IrType::Ref(name) => {
            types.insert(name.clone());
        }
        IrType::Array(inner) | IrType::Map(inner) => collect_refs_from_ir_type(inner, types),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            for v in variants {
                collect_refs_from_ir_type(v, types);
            }
        }
        IrType::Object(fields) => {
            for (_, ty, _) in fields {
                collect_refs_from_ir_type(ty, types);
            }
        }
        _ => {}
    }
}

fn collect_model_imports(ir: &IrSpec) -> Vec<String> {
    let mut imports = HashSet::new();

    for op in &ir.operations {
        match &op.return_type {
            IrReturnType::Standard(resp) => {
                collect_refs_from_ir_type(&resp.response_type, &mut imports);
            }
            IrReturnType::Sse(sse) => {
                if let Some(ref name) = sse.event_type_name {
                    imports.insert(name.clone());
                } else {
                    collect_refs_from_ir_type(&sse.event_type, &mut imports);
                }
                if let Some(ref json) = sse.json_response {
                    collect_refs_from_ir_type(&json.response_type, &mut imports);
                }
            }
            IrReturnType::Void => {}
        }
        if let Some(ref body) = op.request_body {
            collect_refs_from_ir_type(&body.body_type, &mut imports);
        }
        for param in &op.parameters {
            collect_refs_from_ir_type(&param.param_type, &mut imports);
        }
    }

    let mut sorted: Vec<String> = imports.into_iter().collect();
    sorted.sort();
    sorted
}

// ─── Test contexts ───────────────────────────────────────────────

fn build_test_operation_contexts(
    ir: &IrSpec,
    tm: &TypeMapConfig,
    _operation_casing: &str,
) -> (Vec<Value>, HashSet<usize>) {
    let mut seen_methods = HashSet::new();
    let mut used_op_indices = HashSet::new();

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .enumerate()
        .flat_map(|(idx, op)| {
            build_single_test_contexts(ir, op, tm)
                .into_iter()
                .map(move |ctx| (idx, ctx))
        })
        .filter(|(idx, op)| {
            let name = op
                .get_attr("method_name")
                .ok()
                .and_then(|v| v.as_str().map(String::from));
            match name {
                Some(n) => {
                    if seen_methods.insert(n) {
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

    (operations, used_op_indices)
}

fn build_single_test_contexts(ir: &IrSpec, op: &IrOperation, tm: &TypeMapConfig) -> Vec<Value> {
    let args = build_op_args(ir, op, tm, "camel");
    let mut results = Vec::new();

    match &op.return_type {
        IrReturnType::Standard(resp) => {
            let return_type = map_type(&resp.response_type, tm);
            results.push(build_ts_test_context(
                &args,
                op,
                "standard",
                &op.name.camel_case,
                &return_type,
                resp.format.as_str(),
            ));
        }
        IrReturnType::Void => {
            results.push(build_ts_test_context(
                &args,
                op,
                "void",
                &op.name.camel_case,
                "void",
                "json",
            ));
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
                map_type(&sse.event_type, tm)
            };
            results.push(build_ts_test_context(
                &args,
                op,
                "sse",
                &sse_name,
                &return_type,
                "json",
            ));

            if let Some(ref json_resp) = sse.json_response {
                let rt = map_type(&json_resp.response_type, tm);
                results.push(build_ts_test_context(
                    &args,
                    op,
                    "standard",
                    &op.name.camel_case,
                    &rt,
                    json_resp.format.as_str(),
                ));
            }
        }
    }

    results
}

fn build_ts_test_context(
    args: &OpArgs,
    op: &IrOperation,
    kind: &str,
    method_name: &str,
    return_type: &str,
    response_format: &str,
) -> Value {
    let has_body = op.request_body.is_some();
    let test_call_args = args.test_call_args();
    let test_args_before_options = args.test_args_before_options();
    let expected_url_pattern = build_ts_expected_url_pattern(args, op);
    let mock_response = mock_value_ts(&if return_type == "void" {
        IrType::Void
    } else {
        guess_mock_type(return_type)
    });

    context! {
        kind => kind,
        method_name => method_name,
        http_method => op.method.as_str(),
        return_type => return_type,
        has_body => has_body,
        test_call_args => test_call_args,
        test_args_before_options => test_args_before_options,
        expected_url_pattern => expected_url_pattern,
        mock_response => mock_response,
        response_format => response_format,
    }
}

fn build_ts_expected_url_pattern(args: &OpArgs, op: &IrOperation) -> String {
    let mut path = op.path.clone();
    let path_args = args.args.iter().filter(|a| a.kind == ArgKind::Path);
    let path_params = op
        .parameters
        .iter()
        .filter(|p| p.location == IrParameterLocation::Path);
    for (param, arg) in path_params.zip(path_args) {
        let placeholder = format!("{{{}}}", param.original_name);
        // The mock is a TS literal (`"test"`, `1`); the URL carries its value.
        let value = arg.mock.trim_matches('"').to_string();
        path = path.replace(&placeholder, &value);
    }
    path
}

fn mock_value_ts(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime | IrType::Encoded(_) => "\"test\"".to_string(),
        IrType::StringLiteral(s) => format!("\"{s}\""),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::IntegerLiteral(i) => i.to_string(),
        IrType::Boolean => "true".to_string(),
        IrType::Null | IrType::Void => "undefined".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Ref(name) => format!("{{}} as {}", name),
        IrType::Binary => "new Blob()".to_string(),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            if let Some(first) = variants.first() {
                mock_value_ts(first)
            } else {
                "{}".to_string()
            }
        }
    }
}

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

/// Type names the generated tests reference: exactly the `{} as Name` casts in
/// their mock values, so no import goes unused.
fn collect_test_type_imports(ir: &IrSpec, test_operations: &[Value]) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for op in test_operations {
        for key in [
            "test_call_args",
            "test_args_before_options",
            "mock_response",
        ] {
            let Some(text) = op
                .get_attr(key)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
            else {
                continue;
            };
            let mut rest = text.as_str();
            while let Some(i) = rest.find(" as ") {
                rest = &rest[i + 4..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                // Only generated types: `{} as Blob` refers to the global.
                if ir.find_schema(&name).is_some() {
                    names.insert(name);
                }
            }
        }
    }
    names.into_iter().collect()
}

// ─── Python test contexts ────────────────────────────────────────

fn build_python_test_contexts(ir: &IrSpec) -> (Vec<Value>, Vec<String>) {
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

    let operations: Vec<Value> = ir
        .operations
        .iter()
        .flat_map(build_single_python_test_context)
        .collect();

    (operations, model_imports)
}

fn build_single_python_test_context(op: &IrOperation) -> Vec<Value> {
    let mut results = Vec::new();

    let http_method = match op.method {
        HttpMethod::Get => "get",
        HttpMethod::Post => "post",
        HttpMethod::Put => "put",
        HttpMethod::Delete => "delete",
        HttpMethod::Patch => "patch",
        _ => "get",
    };

    let test_path = build_python_test_path(&op.path, op);
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

fn build_python_test_path(path: &str, op: &IrOperation) -> String {
    let mut result = path.to_string();
    for param in &op.parameters {
        if param.location == IrParameterLocation::Path {
            let placeholder = format!("{{{}}}", param.original_name);
            let test_value = match &param.param_type {
                IrType::Integer | IrType::Number => "1".to_string(),
                _ => "test".to_string(),
            };
            result = result.replace(&placeholder, &test_value);
        }
    }
    result
}

fn mock_value_python(ir_type: &IrType) -> String {
    match ir_type {
        IrType::String | IrType::DateTime | IrType::Encoded(_) => "\"test\"".to_string(),
        IrType::StringLiteral(s) => format!("\"{s}\""),
        IrType::Number | IrType::Integer => "1".to_string(),
        IrType::IntegerLiteral(i) => i.to_string(),
        IrType::Boolean => "True".to_string(),
        IrType::Null | IrType::Void => "None".to_string(),
        IrType::Array(_) => "[]".to_string(),
        IrType::Ref(name) => format!("{}.model_construct()", name),
        IrType::Object(_) | IrType::Map(_) | IrType::Any => "{}".to_string(),
        IrType::Binary => "b\"test\"".to_string(),
        IrType::Union(variants) | IrType::Intersection(variants) => {
            if let Some(first) = variants.first() {
                mock_value_python(first)
            } else {
                "{}".to_string()
            }
        }
    }
}
