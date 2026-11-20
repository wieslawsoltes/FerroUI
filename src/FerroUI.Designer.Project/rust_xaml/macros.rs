use super::{string_literal, RustPathAnchor};
use std::collections::BTreeSet;
use syn::{
    punctuated::Punctuated,
    visit::{self, Visit},
    Expr, ExprLit, Lit, LitStr, Macro, Token, UseTree,
};

#[derive(Default)]
pub(super) struct MacroSyntax {
    local_names: BTreeSet<String>,
}

impl MacroSyntax {
    pub(super) fn new(file: &syn::File) -> Self {
        let mut syntax = Self::default();
        syntax.visit_file(file);
        syntax
    }

    pub(super) fn include_path(
        &self,
        expression: &Macro,
    ) -> Option<(LitStr, RustPathAnchor, &'static str)> {
        let args = self
            .arguments(expression, "include_str")
            .or_else(|| self.arguments(expression, "include_bytes"))?;
        if args.len() != 1 {
            return None;
        }
        match args.into_iter().next()? {
            Expr::Macro(nested) => self.manifest_path(&nested.mac),
            value => Some((take_literal(value)?, RustPathAnchor::SourceFile, "")),
        }
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
            None => first == name && !absolute && !self.local_names.contains(name),
            Some(last) => {
                (first == "std" || first == "core")
                    && last.ident == name
                    && segments.next().is_none()
                    && (absolute || !self.local_names.contains(&first.to_string()))
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
    fn visit_item_macro(&mut self, item: &'source syn::ItemMacro) {
        if let Some(name) = &item.ident {
            self.local_names.insert(name.to_string());
        }
    }

    fn visit_item_mod(&mut self, item: &'source syn::ItemMod) {
        self.local_names.insert(item.ident.to_string());
        visit::visit_item_mod(self, item);
    }

    fn visit_use_tree(&mut self, tree: &'source UseTree) {
        match tree {
            UseTree::Name(name) => {
                self.local_names.insert(name.ident.to_string());
            }
            UseTree::Rename(name) => {
                self.local_names.insert(name.rename.to_string());
            }
            _ => visit::visit_use_tree(self, tree),
        }
    }
}

fn take_literal(expr: Expr) -> Option<LitStr> {
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
