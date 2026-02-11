use std::collections::HashSet;

use indexmap::IndexMap;

use super::components::Components;
use super::media_type::MediaType;
use super::operation::{Operation, PathItem};
use super::parameter::{Parameter, ParameterOrRef};
use super::request_body::{RequestBody, RequestBodyOrRef};
use super::response::{Response, ResponseOrRef};
use super::schema::{Schema, SchemaOrRef};
use super::spec::OpenApiSpec;
use crate::error::ResolveError;

/// Resolves all `$ref` pointers in an OpenAPI spec, producing a spec
/// with no remaining references. Detects circular references.
pub struct RefResolver<'a> {
    components: Option<&'a Components>,
    visited: HashSet<String>,
}

impl<'a> RefResolver<'a> {
    pub fn new(spec: &'a OpenApiSpec) -> Self {
        Self {
            components: spec.components.as_ref(),
            visited: HashSet::new(),
        }
    }

    /// Resolve the entire spec in place, returning a spec with no `$ref` nodes.
    pub fn resolve_spec(&mut self, spec: &OpenApiSpec) -> Result<OpenApiSpec, ResolveError> {
        let mut resolved = spec.clone();

        // Resolve all paths
        for (_path, item) in &mut resolved.paths {
            self.resolve_path_item(item)?;
        }

        // Resolve component schemas
        if let Some(ref mut components) = resolved.components {
            let schema_names: Vec<String> = components.schemas.keys().cloned().collect();
            for name in schema_names {
                let schema = components.schemas.get(&name).unwrap().clone();
                let resolved_schema = self.resolve_schema_or_ref(&schema)?;
                components.schemas.insert(name, resolved_schema);
            }
        }

        Ok(resolved)
    }

    fn resolve_path_item(&mut self, item: &mut PathItem) -> Result<(), ResolveError> {
        // Resolve path-level parameters
        let mut resolved_params = Vec::new();
        for p in &item.parameters {
            resolved_params.push(self.resolve_parameter_or_ref(p)?);
        }
        item.parameters = resolved_params;

        // Resolve each operation
        macro_rules! resolve_op {
            ($op:expr) => {
                if let Some(ref mut op) = $op {
                    self.resolve_operation(op)?;
                }
            };
        }
        resolve_op!(item.get);
        resolve_op!(item.post);
        resolve_op!(item.put);
        resolve_op!(item.delete);
        resolve_op!(item.patch);
        resolve_op!(item.options);
        resolve_op!(item.head);
        resolve_op!(item.trace);
        Ok(())
    }

    fn resolve_operation(&mut self, op: &mut Operation) -> Result<(), ResolveError> {
        // Resolve parameters
        let mut resolved_params = Vec::new();
        for p in &op.parameters {
            resolved_params.push(self.resolve_parameter_or_ref(p)?);
        }
        op.parameters = resolved_params;

        // Resolve request body
        if let Some(ref body) = op.request_body {
            let resolved = self.resolve_request_body_or_ref(body)?;
            op.request_body = Some(resolved);
        }

        // Resolve responses
        let mut resolved_responses = IndexMap::new();
        for (status, resp) in &op.responses {
            resolved_responses.insert(status.clone(), self.resolve_response_or_ref(resp)?);
        }
        op.responses = resolved_responses;

        Ok(())
    }

    pub fn resolve_schema_or_ref(
        &mut self,
        schema_or_ref: &SchemaOrRef,
    ) -> Result<SchemaOrRef, ResolveError> {
        match schema_or_ref {
            SchemaOrRef::Ref { ref_path } => {
                if self.visited.contains(ref_path) {
                    // Circular reference — return as-is to avoid infinite loop.
                    // The IR transform layer handles these.
                    return Ok(schema_or_ref.clone());
                }
                self.visited.insert(ref_path.clone());
                let resolved = self.lookup_schema(ref_path)?;
                let result =
                    self.resolve_schema_or_ref(&SchemaOrRef::Schema(Box::new(resolved)))?;
                self.visited.remove(ref_path);
                Ok(result)
            }
            SchemaOrRef::Schema(schema) => {
                let resolved = self.resolve_schema(schema)?;
                Ok(SchemaOrRef::Schema(Box::new(resolved)))
            }
        }
    }

    fn resolve_schema(&mut self, schema: &Schema) -> Result<Schema, ResolveError> {
        let mut resolved = schema.clone();

        // Resolve properties
        let mut resolved_props = IndexMap::new();
        for (name, prop) in &schema.properties {
            resolved_props.insert(name.clone(), self.resolve_schema_or_ref(prop)?);
        }
        resolved.properties = resolved_props;

        // Resolve items
        if let Some(ref items) = schema.items {
            resolved.items = Some(Box::new(self.resolve_schema_or_ref(items)?));
        }

        // Resolve allOf, oneOf, anyOf
        resolved.all_of = schema
            .all_of
            .iter()
            .map(|s| self.resolve_schema_or_ref(s))
            .collect::<Result<Vec<_>, _>>()?;
        resolved.one_of = schema
            .one_of
            .iter()
            .map(|s| self.resolve_schema_or_ref(s))
            .collect::<Result<Vec<_>, _>>()?;
        resolved.any_of = schema
            .any_of
            .iter()
            .map(|s| self.resolve_schema_or_ref(s))
            .collect::<Result<Vec<_>, _>>()?;

        // Resolve additionalProperties
        if let Some(super::schema::AdditionalProperties::Schema(ref s)) =
            schema.additional_properties
        {
            resolved.additional_properties = Some(super::schema::AdditionalProperties::Schema(
                Box::new(self.resolve_schema_or_ref(s)?),
            ));
        }

        Ok(resolved)
    }

    fn resolve_parameter_or_ref(
        &mut self,
        param: &ParameterOrRef,
    ) -> Result<ParameterOrRef, ResolveError> {
        match param {
            ParameterOrRef::Ref { ref_path } => {
                let resolved = self.lookup_parameter(ref_path)?;
                Ok(ParameterOrRef::Parameter(resolved))
            }
            ParameterOrRef::Parameter(p) => {
                let mut resolved = p.clone();
                if let Some(ref s) = p.schema {
                    resolved.schema = Some(self.resolve_schema_or_ref(s)?);
                }
                Ok(ParameterOrRef::Parameter(resolved))
            }
        }
    }

    fn resolve_request_body_or_ref(
        &mut self,
        body: &RequestBodyOrRef,
    ) -> Result<RequestBodyOrRef, ResolveError> {
        match body {
            RequestBodyOrRef::Ref { ref_path } => {
                let resolved = self.lookup_request_body(ref_path)?;
                let mut rb = resolved;
                self.resolve_media_types(&mut rb.content)?;
                Ok(RequestBodyOrRef::RequestBody(rb))
            }
            RequestBodyOrRef::RequestBody(rb) => {
                let mut resolved = rb.clone();
                self.resolve_media_types(&mut resolved.content)?;
                Ok(RequestBodyOrRef::RequestBody(resolved))
            }
        }
    }

    fn resolve_response_or_ref(
        &mut self,
        resp: &ResponseOrRef,
    ) -> Result<ResponseOrRef, ResolveError> {
        match resp {
            ResponseOrRef::Ref { ref_path } => {
                let resolved = self.lookup_response(ref_path)?;
                let mut r = resolved;
                self.resolve_media_types(&mut r.content)?;
                Ok(ResponseOrRef::Response(r))
            }
            ResponseOrRef::Response(r) => {
                let mut resolved = r.clone();
                self.resolve_media_types(&mut resolved.content)?;
                Ok(ResponseOrRef::Response(resolved))
            }
        }
    }

    fn resolve_media_types(
        &mut self,
        content: &mut IndexMap<String, MediaType>,
    ) -> Result<(), ResolveError> {
        let keys: Vec<String> = content.keys().cloned().collect();
        for key in keys {
            let mt = content.get(&key).unwrap().clone();
            let mut resolved_mt = mt;
            if let Some(ref s) = resolved_mt.schema {
                resolved_mt.schema = Some(self.resolve_schema_or_ref(s)?);
            }
            if let Some(ref s) = resolved_mt.item_schema {
                resolved_mt.item_schema = Some(self.resolve_schema_or_ref(s)?);
            }
            content.insert(key, resolved_mt);
        }
        Ok(())
    }

    // Lookup helpers

    fn lookup_schema(&self, ref_path: &str) -> Result<Schema, ResolveError> {
        let name = parse_ref_name(ref_path, "schemas")?;
        self.components
            .and_then(|c| c.schemas.get(name))
            .and_then(|s| match s {
                SchemaOrRef::Schema(schema) => Some(schema.as_ref().clone()),
                SchemaOrRef::Ref { ref_path: inner } => {
                    // Transitive ref — just extract the name and look up again
                    let inner_name = parse_ref_name(inner, "schemas").ok()?;
                    self.components
                        .and_then(|c| c.schemas.get(inner_name))
                        .and_then(|s2| match s2 {
                            SchemaOrRef::Schema(schema) => Some(schema.as_ref().clone()),
                            _ => None,
                        })
                }
            })
            .ok_or_else(|| ResolveError::RefTargetNotFound(ref_path.to_string()))
    }

    fn lookup_parameter(&self, ref_path: &str) -> Result<Parameter, ResolveError> {
        let name = parse_ref_name(ref_path, "parameters")?;
        self.components
            .and_then(|c| c.parameters.get(name))
            .and_then(|p| match p {
                ParameterOrRef::Parameter(param) => Some(param.clone()),
                _ => None,
            })
            .ok_or_else(|| ResolveError::RefTargetNotFound(ref_path.to_string()))
    }

    fn lookup_request_body(&self, ref_path: &str) -> Result<RequestBody, ResolveError> {
        let name = parse_ref_name(ref_path, "requestBodies")?;
        self.components
            .and_then(|c| c.request_bodies.get(name))
            .and_then(|rb| match rb {
                RequestBodyOrRef::RequestBody(body) => Some(body.clone()),
                _ => None,
            })
            .ok_or_else(|| ResolveError::RefTargetNotFound(ref_path.to_string()))
    }

    fn lookup_response(&self, ref_path: &str) -> Result<Response, ResolveError> {
        let name = parse_ref_name(ref_path, "responses")?;
        self.components
            .and_then(|c| c.responses.get(name))
            .and_then(|r| match r {
                ResponseOrRef::Response(resp) => Some(resp.clone()),
                _ => None,
            })
            .ok_or_else(|| ResolveError::RefTargetNotFound(ref_path.to_string()))
    }
}

/// Parse a `$ref` path like `#/components/schemas/Foo` and extract the name.
fn parse_ref_name<'a>(ref_path: &'a str, expected_section: &str) -> Result<&'a str, ResolveError> {
    let stripped = ref_path
        .strip_prefix("#/components/")
        .ok_or_else(|| ResolveError::InvalidRefFormat(ref_path.to_string()))?;
    let (section, name) = stripped
        .split_once('/')
        .ok_or_else(|| ResolveError::InvalidRefFormat(ref_path.to_string()))?;
    if section != expected_section {
        return Err(ResolveError::InvalidRefFormat(format!(
            "expected section '{}', got '{}' in {}",
            expected_section, section, ref_path
        )));
    }
    Ok(name)
}
