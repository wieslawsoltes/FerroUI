mod imports;

use self::imports::StandardImports;
use crate::visit_rust_imports;
use super::{string_literal, RustPathAnchor};
use std::collections::{BTreeMap, BTreeSet};
use syn::{
    punctuated::Punctuated,
    visit::{self, Visit},
    Expr, ExprLit, Lit, LitStr, Macro, Token, UseTree,
};

#[derive(Default)]
pub(super) struct MacroSyntax {
    local_names: BTreeMap<String, usize>,
    file_input_names: BTreeSet<String>,
    standard_imports: StandardImports,
    bound_values: BTreeSet<String>,
    pub(super) changes_directory: bool,
}

impl MacroSyntax {
    pub(super) fn new(file: &syn::File) -> Self {
        let mut syntax = Self {
            standard_imports: StandardImports::new(file),
            ..Self::default()
        };
        syntax.visit_file(file);
        syntax
    }

    pub(super) fn shadows(&self, name: &str) -> bool {
        self.local_names.contains_key(name)
    }

    pub(super) fn standard_path(&self, path: &syn::Path, expected: &[&str]) -> bool {
        let mut parts = path.segments.iter();
        let Some(first) = parts.next().map(|part| part.ident.to_string()) else {
            return false;
        };
        if first == "std" && (path.leading_colon.is_some() || !self.shadows("std")) {
            return parts
                .map(|part| part.ident.to_string())
                .eq(expected.iter().copied());
        }
        if path.leading_colon.is_some()
            || self.local_names.get(&first) != Some(&1)
            || (parts.len() == 0 && self.bound_values.contains(&first))
        {
            return false;
        }
        self.standard_imports
            .find(&first, self.shadows("std"))
            .is_some_and(|source| {
                source
                    .iter()
                    .cloned()
                    .chain(parts.map(|part| part.ident.to_string()))
                    .eq(expected.iter().copied())
            })
    }

    fn note_local_name(&mut self, identifier: &syn::Ident) {
        *self.local_names.entry(identifier.to_string()).or_default() += 1;
    }

    pub(super) fn file_input(&self, expression: &Macro) -> Option<LitStr> {
        let mut segments = expression.path.segments.iter();
        let first = &segments.next()?.ident;
        let absolute = expression.path.leading_colon.is_some();
        let known = match segments.next() {
            None => !absolute && self.file_input_names.contains(&first.to_string()),
            Some(last) => {
                first == "ferroui_build"
                    && last.ident == "xaml_document_file"
                    && segments.next().is_none()
                    && (absolute || !self.shadows(&first.to_string()))
            }
        };
        if !known {
            return None;
        }
        let args = expression
            .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
            .ok()?;
        if args.len() != 2 {
            return None;
        }
        take_literal(args.into_iter().next_back()?)
    }

    pub(super) fn manifest_path(
        &self,
        expression: &Macro,
    ) -> Option<(LitStr, RustPathAnchor, &'static str)> {
        let args = self.arguments(expression, "concat")?;
        if args.len() != 2 {
            return None;
        }
        let mut args = args.into_iter();
        let Expr::Macro(env) = args.next()? else {
            return None;
        };
        let env = self.arguments(&env.mac, "env")?;
        if env.len() != 1 || env.first().and_then(string_literal)?.value() != "CARGO_MANIFEST_DIR" {
            return None;
        }
        Some((take_literal(args.next()?)?, RustPathAnchor::Package, "/"))
    }

    pub(super) fn arguments(
        &self,
        expression: &Macro,
        name: &str,
    ) -> Option<Punctuated<Expr, Token![,]>> {
        let mut segments = expression.path.segments.iter();
        let first = &segments.next()?.ident;
        let absolute = expression.path.leading_colon.is_some();
        let known = match segments.next() {
            None => first == name && !absolute && !self.shadows(name),
            Some(last) => {
                (first == "std" || first == "core")
                    && last.ident == name
                    && segments.next().is_none()
                    && (absolute || !self.shadows(&first.to_string()))
            }
        };
        if !known {
            return None;
        }
        expression
            .parse_body_with(Punctuated::parse_terminated)
            .ok()
    }
}

impl<'source> Visit<'source> for MacroSyntax {
    fn visit_expr_path(&mut self, path: &'source syn::ExprPath) {
        if path
            .path
            .segments
            .last()
            .is_some_and(|part| part.ident == "set_current_dir")
        {
            self.changes_directory = true;
        }
        visit::visit_expr_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'source syn::ItemUse) {
        if let UseTree::Path(path) = &item.tree {
            if path.ident == "ferroui_build" {
                file_input_imports(&path.tree, &mut self.file_input_names);
            }
        }
        visit_rust_imports(&item.tree, |name, path| {
            self.changes_directory |= path.last().is_some_and(|part| *part == "set_current_dir");
            self.note_local_name(name);
        });
    }

    fn visit_item_macro(&mut self, item: &'source syn::ItemMacro) {
        if let Some(name) = &item.ident {
            self.note_local_name(name);
        }
    }

    fn visit_item_mod(&mut self, item: &'source syn::ItemMod) {
        self.note_local_name(&item.ident);
        visit::visit_item_mod(self, item);
    }

    fn visit_item_fn(&mut self, item: &'source syn::ItemFn) {
        self.bound_values.insert(item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
    }

    fn visit_pat_ident(&mut self, pattern: &'source syn::PatIdent) {
        self.bound_values.insert(pattern.ident.to_string());
        visit::visit_pat_ident(self, pattern);
    }
}

fn file_input_imports(tree: &UseTree, names: &mut BTreeSet<String>) {
    match tree {
        UseTree::Name(name) if name.ident == "xaml_document_file" => {
            names.insert(name.ident.to_string());
        }
        UseTree::Rename(name) if name.ident == "xaml_document_file" => {
            names.insert(name.rename.to_string());
        }
        UseTree::Group(group) => {
            for item in &group.items {
                file_input_imports(item, names);
            }
        }
        _ => {}
    }
}

pub(super) fn take_literal(expr: Expr) -> Option<LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(literal),
            ..
        }) => Some(literal),
        Expr::Paren(paren) => take_literal(*paren.expr),
        Expr::Group(group) => take_literal(*group.expr),
        _ => None,
    }
}
