//! Test-only full reference closure using the shared production dependency importer.
use super::*;
pub(super) fn compilation(c: Compilation) -> Compilation {
    let roots: Vec<_> = ["Object", "String", "Int32"]
        .iter()
        .map(|name| {
            c.types
                .iter()
                .position(|t| t.namespace == "System" && t.metadata_name == *name)
                .unwrap()
        })
        .collect();
    native_adapter::import_semantic_dependencies(&c, &roots, &[], |_, _| Ok(())).unwrap()
}
