use super::{LocalRust, Resolved};
use syn::{ItemUse, UseTree, ext::IdentExt};

pub(super) struct GlobImport {
    path: Vec<String>,
    absolute: bool,
    conditional: bool,
}

pub(super) fn is_view_factory(name: &str) -> bool {
    matches!(name, "style" | "animation" | "ms")
}

impl LocalRust {
    pub(super) fn view_prelude(&self, scope: usize, depth: usize) -> bool {
        self.scopes[scope].globs.iter().all(|glob| {
            if glob.conditional {
                return false;
            }
            let path = glob.path.iter().map(String::as_str).collect::<Vec<_>>();
            matches!(self.follow(scope, glob.absolute, &path, depth),
                Some(Resolved::External(name)) if name == "ferroui_view::prelude")
        })
    }
}

pub(super) fn collect(item: &ItemUse, conditional: bool, output: &mut Vec<GlobImport>) {
    collect_tree(
        &item.tree,
        &mut Vec::new(),
        item.leading_colon.is_some(),
        conditional,
        output,
    );
}

fn collect_tree(
    tree: &UseTree,
    path: &mut Vec<String>,
    absolute: bool,
    conditional: bool,
    output: &mut Vec<GlobImport>,
) {
    match tree {
        UseTree::Path(branch) => {
            path.push(branch.ident.unraw().to_string());
            collect_tree(&branch.tree, path, absolute, conditional, output);
            path.pop();
        }
        UseTree::Group(group) => {
            for child in &group.items {
                collect_tree(child, path, absolute, conditional, output);
            }
        }
        UseTree::Glob(_) => output.push(GlobImport {
            path: path.clone(),
            absolute,
            conditional,
        }),
        _ => {}
    }
}
