//! Resolve metadata operands from one physical Rust source file.
use super::{string_literal, ungroup};
use crate::scanner::modules::{Import, ItemKind, Modules, Target};
use crate::scanner::source::local_imports;
use std::collections::{BTreeMap, BTreeSet};
use syn::{Attribute, ExprPath, File, Item, LitStr};

pub(super) struct MetadataConstants<'a> {
    tree: Modules,
    scope: usize,
    literals: BTreeMap<(usize, String), Option<&'a LitStr>>,
}

impl<'a> MetadataConstants<'a> {
    pub(super) fn new(file: &'a File, active: &[String]) -> Option<Self> {
        let mut tree = Modules::new("", &[]);
        // Physical files can be nested in the crate. A synthetic parent
        // leaves crate-relative paths and paths outside this file unresolved.
        let scope = tree.add_module(0, "", false);
        let mut constants = Self {
            tree,
            scope,
            literals: BTreeMap::new(),
        };
        constants.index(&file.items, scope);
        for name in active {
            let ItemKind::Module(child) = constants.tree.modules[constants.scope]
                .items
                .get(name)?
                .kind
            else {
                return None;
            };
            constants.scope = child;
        }
        constants.tree.finish();
        Some(constants)
    }

    pub(super) fn literal(&self, path: &ExprPath) -> Option<&'a LitStr> {
        if path.qself.is_some()
            || path.path.leading_colon.is_some()
            || !path
                .path
                .segments
                .iter()
                .all(|segment| segment.arguments.is_empty())
        {
            return None;
        }
        let segments = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let Target::Item { module, name, rest } = self.tree.resolve(self.scope, &segments)? else {
            return None;
        };
        if !rest.is_empty() {
            return None;
        }
        *self.literals.get(&(module, name))?
    }

    fn index(&mut self, items: &'a [Item], scope: usize) {
        let mut children = BTreeMap::new();
        let mut imports = Vec::new();
        let mut uncertain = BTreeSet::new();
        for item in items {
            match item {
                Item::Const(item) => {
                    let name = item.ident.to_string();
                    let literal = (!conditional(&item.attrs))
                        .then(|| string_literal(ungroup(&item.expr)))
                        .flatten();
                    self.tree.add_item(scope, &name, ItemKind::Value, false);
                    self.literals
                        .entry((scope, name))
                        .and_modify(|previous| *previous = None)
                        .or_insert(literal);
                }
                Item::Mod(item) => {
                    let name = item.ident.to_string();
                    let body = item
                        .content
                        .as_ref()
                        .filter(|_| !conditional(&item.attrs))
                        .map(|(_, items)| items);
                    children
                        .entry(name)
                        .and_modify(|previous| *previous = None)
                        .or_insert(body);
                }
                Item::Use(item) => {
                    let mut bindings = Vec::new();
                    let mut prefix = item
                        .leading_colon
                        .is_some()
                        .then(String::new)
                        .into_iter()
                        .collect();
                    local_imports(&item.tree, &mut prefix, &mut bindings);
                    for (name, path) in bindings {
                        if conditional(&item.attrs)
                            || imports.iter().any(|import: &Import| import.name == name)
                        {
                            uncertain.insert(name.clone());
                        }
                        imports.push(Import {
                            name,
                            path,
                            public: false,
                            explicit_alias: false,
                        });
                    }
                }
                _ => {}
            }
        }
        for import in &imports {
            if let Some(literal) = self.literals.get_mut(&(scope, import.name.clone())) {
                *literal = None;
            }
        }
        self.tree.modules[scope].imports = imports
            .into_iter()
            .filter(|import| !uncertain.contains(&import.name))
            .collect();
        for (name, body) in children {
            let child = self.tree.add_module(scope, &name, false);
            if let Some(body) = body {
                self.index(body, child);
            }
        }
    }
}

fn conditional(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}
