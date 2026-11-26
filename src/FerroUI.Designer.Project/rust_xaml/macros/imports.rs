//! Expand explicit, crate-level standard library imports used by Cargo file reads.
use crate::visit_rust_imports;
use std::collections::BTreeMap;
use syn::{File, Item};

#[derive(Default)]
pub(super) struct StandardImports(BTreeMap<String, (bool, Vec<String>)>);

impl StandardImports {
    pub(super) fn new(file: &File) -> Self {
        let mut imports = BTreeMap::new();
        for item in &file.items {
            let Item::Use(item) = item else { continue };
            visit_rust_imports(&item.tree, |alias, path| {
                if path.first().is_some_and(|part| *part == "std") {
                    imports.insert(
                        alias.to_string(),
                        (
                            item.leading_colon.is_some(),
                            path[1..].iter().map(ToString::to_string).collect(),
                        ),
                    );
                }
            });
        }
        Self(imports)
    }

    pub(super) fn find(&self, name: &str, std_is_shadowed: bool) -> Option<&[String]> {
        let (absolute, path) = self.0.get(name)?;
        (*absolute || !std_is_shadowed).then_some(path)
    }
}
