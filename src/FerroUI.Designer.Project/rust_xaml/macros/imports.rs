//! Expand explicit, crate-level standard library imports used by Cargo file reads.
use std::collections::BTreeMap;
use syn::{File, Ident, Item, UseTree};

#[derive(Default)]
pub(super) struct StandardImports(BTreeMap<String, (bool, Vec<String>)>);

impl StandardImports {
    pub(super) fn new(file: &File) -> Self {
        let mut imports = BTreeMap::new();
        for item in &file.items {
            let Item::Use(item) = item else { continue };
            visit_bindings(&item.tree, &mut Vec::new(), &mut |alias, path| {
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

pub(super) fn visit_bindings<'source>(
    tree: &'source UseTree,
    path: &mut Vec<&'source Ident>,
    visitor: &mut impl FnMut(&Ident, &[&Ident]),
) {
    let (name, alias) = match tree {
        UseTree::Path(branch) => {
            path.push(&branch.ident);
            visit_bindings(&branch.tree, path, visitor);
            path.pop();
            return;
        }
        UseTree::Group(group) => {
            for child in &group.items {
                visit_bindings(child, path, visitor);
            }
            return;
        }
        UseTree::Name(leaf) => (&leaf.ident, &leaf.ident),
        UseTree::Rename(leaf) => (&leaf.ident, &leaf.rename),
        UseTree::Glob(_) => return,
    };
    if name == "self" {
        if let Some(last) = path.last() {
            visitor(if alias == "self" { last } else { alias }, path);
        }
    } else {
        path.push(name);
        visitor(alias, path);
        path.pop();
    }
}
