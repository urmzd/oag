use super::types::{IrSpec, NormalizedName};
use crate::config::SplitBy;
use crate::transform::name_normalizer::normalize_name;
use indexmap::IndexMap;

/// A group of operations, used for split layout.
#[derive(Debug, Clone)]
pub struct OperationGroup {
    pub name: NormalizedName,
    pub operation_indices: Vec<usize>,
}

/// Group operations in the IR spec according to the split strategy.
pub fn group_operations(ir: &IrSpec, split_by: SplitBy) -> Vec<OperationGroup> {
    match split_by {
        SplitBy::Tag => group_by_tag(ir),
        SplitBy::Operation => group_by_operation(ir),
        SplitBy::Route => group_by_route(ir),
    }
}

/// Group by tag — reuses `IrModule` groupings.
fn group_by_tag(ir: &IrSpec) -> Vec<OperationGroup> {
    ir.modules
        .iter()
        .map(|m| OperationGroup {
            name: m.name.clone(),
            operation_indices: m.operations.clone(),
        })
        .collect()
}

/// Group by operation — one group per operation.
fn group_by_operation(ir: &IrSpec) -> Vec<OperationGroup> {
    ir.operations
        .iter()
        .enumerate()
        .map(|(i, op)| OperationGroup {
            name: op.name.clone(),
            operation_indices: vec![i],
        })
        .collect()
}

/// Group by route — group operations by their first path segment.
fn group_by_route(ir: &IrSpec) -> Vec<OperationGroup> {
    let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();

    for (i, op) in ir.operations.iter().enumerate() {
        let prefix = extract_path_prefix(&op.path);
        groups.entry(prefix).or_default().push(i);
    }

    groups
        .into_iter()
        .map(|(prefix, indices)| OperationGroup {
            name: normalize_name(&prefix),
            operation_indices: indices,
        })
        .collect()
}

/// Extract the first meaningful path segment as a group name.
/// e.g. "/pets/{petId}" → "pets", "/store/inventory" → "store"
fn extract_path_prefix(path: &str) -> String {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .collect();

    segments.first().unwrap_or(&"default").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_prefix() {
        assert_eq!(extract_path_prefix("/pets"), "pets");
        assert_eq!(extract_path_prefix("/pets/{petId}"), "pets");
        assert_eq!(extract_path_prefix("/store/inventory"), "store");
        assert_eq!(extract_path_prefix("/chat/completions"), "chat");
        assert_eq!(extract_path_prefix("/"), "default");
    }
}
