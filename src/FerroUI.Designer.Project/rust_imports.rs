//! Walk named Rust imports without inferring members from glob imports.
use syn::{Ident, UseTree};

/// Visit each local name and the authored path that it imports.
pub fn visit_rust_imports(tree: &UseTree, mut visitor: impl FnMut(&Ident, &[&Ident])) {
    visit_bindings(tree, &mut Vec::new(), &mut visitor);
}

fn visit_bindings<'source>(
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
