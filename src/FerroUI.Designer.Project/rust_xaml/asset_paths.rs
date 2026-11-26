//! Checked string operands in the Cargo builder and authored Rust asset references.

use super::{rust_body, MacroSyntax, RustFilePath, RustPathAnchor};
use crate::{is_packaged_asset_uri, ProjectError, Result};
use proc_macro2::{Span, TokenStream, TokenTree};
use std::ops::Range;
use syn::{
    spanned::Spanned,
    visit::{self, Visit},
    Expr, LitStr, Macro,
};

#[derive(Default)]
pub struct RustAssetReferences {
    pub includes: Vec<RustFilePath>,
    pub addresses: Vec<RustFilePath>,
    pub build_calls: Vec<CargoAssetCall>,
}

pub struct CargoAssetCall {
    pub name: String,
    pub callee: Range<usize>,
    pub operands: Vec<RustFilePath>,
}

impl RustAssetReferences {
    pub fn parse(source: &str, build: bool) -> Result<Self> {
        let body = rust_body(source);
        let syntax =
            syn::parse_str::<syn::File>(body).map_err(|error| ProjectError(error.to_string()))?;
        let mut reader = Reader {
            source_offset: source.len() - body.len(),
            build,
            macros: MacroSyntax::new(&syntax),
            references: Self::default(),
        };
        reader.visit_file(&syntax);
        for paths in [
            &mut reader.references.includes,
            &mut reader.references.addresses,
        ] {
            paths.sort_by_key(|site| (site.range.start, site.range.end));
            paths.dedup();
        }
        Ok(reader.references)
    }
}

struct Reader {
    source_offset: usize,
    build: bool,
    macros: MacroSyntax,
    references: RustAssetReferences,
}

impl Reader {
    fn visit_rsx(&mut self, tokens: TokenStream, depth: usize) {
        if depth > 96 {
            return;
        }
        if let Ok(expression) = syn::parse2::<Expr>(tokens.clone()) {
            self.visit_expr(&expression);
            return;
        }
        for token in tokens {
            match token {
                TokenTree::Group(group) => self.visit_rsx(group.stream(), depth + 1),
                TokenTree::Literal(literal) => {
                    if let Ok(literal) = syn::parse2::<LitStr>(TokenTree::Literal(literal).into()) {
                        self.visit_lit_str(&literal);
                    }
                }
                _ => {}
            }
        }
    }

    fn record_build_call(&mut self, name: &str, span: Span, argument: &Expr) {
        let paths = match name {
            "asset" => self.operand(argument).map(|operand| vec![operand]),
            "embed_assets" => match unwrap_operand(argument) {
                Expr::Array(array) => array
                    .elems
                    .iter()
                    .map(|entry| self.operand(entry))
                    .collect::<Option<Vec<_>>>(),
                _ => None,
            },
            _ => return,
        };
        if let Some(operands) = paths {
            let range = span.byte_range();
            self.references.build_calls.push(CargoAssetCall {
                name: name.into(),
                callee: self.source_offset + range.start..self.source_offset + range.end,
                operands,
            });
        }
    }

    fn operand(&self, expression: &Expr) -> Option<RustFilePath> {
        self.macros
            .static_string_path(unwrap_operand(expression), self.source_offset)
            .filter(|path| path.anchor == RustPathAnchor::SourceFile)
    }
}

impl<'ast> Visit<'ast> for Reader {
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}

    fn visit_lit_str(&mut self, literal: &'ast LitStr) {
        if is_packaged_asset_uri(&literal.value()) {
            if let Some(site) = RustFilePath::from_literal(
                literal,
                RustPathAnchor::ResourceUri,
                "",
                self.source_offset,
            ) {
                self.references.addresses.push(site);
            }
        }
    }

    fn visit_macro(&mut self, expression: &'ast Macro) {
        if let Some(site) = self.macros.include_path(expression, self.source_offset) {
            self.references.includes.push(site);
        } else if expression
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx")
        {
            self.visit_rsx(expression.tokens.clone(), 0);
        }
    }

    fn visit_expr_macro(&mut self, expression: &'ast syn::ExprMacro) {
        if let Some(mut path) = self
            .macros
            .static_concat_path(&expression.mac, self.source_offset)
            .filter(|path| {
                path.anchor == RustPathAnchor::SourceFile && is_packaged_asset_uri(&path.path)
            })
        {
            path.anchor = RustPathAnchor::ResourceUri;
            self.references.addresses.push(path);
        } else {
            self.visit_macro(&expression.mac);
        }
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if self.build && call.args.len() == 1 {
            self.record_build_call(&call.method.to_string(), call.method.span(), &call.args[0]);
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.build && call.args.len() == 2 {
            if let Expr::Path(function) = call.func.as_ref() {
                if let Some(name) = function.path.segments.last() {
                    self.record_build_call(&name.ident.to_string(), function.span(), &call.args[1]);
                }
            }
        }
        visit::visit_expr_call(self, call);
    }
}

fn unwrap_operand(expression: &Expr) -> &Expr {
    match expression {
        Expr::Reference(expr) => unwrap_operand(&expr.expr),
        Expr::Paren(expr) => unwrap_operand(&expr.expr),
        Expr::Group(expr) => unwrap_operand(&expr.expr),
        _ => expression,
    }
}

/// Convert Rust's one-based character coordinates to an offset in the current UTF-8 text.
pub fn rust_source_offset(source: &str, line: u32, column: u32) -> Option<usize> {
    let body = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut offset = source.len() - body.len();
    let mut lines = body.split_inclusive('\n');
    for _ in 0..line.checked_sub(1)? {
        offset += lines.next()?.len();
    }
    let (local, _) = lines
        .next()?
        .char_indices()
        .nth(column.checked_sub(1)? as usize)?;
    Some(offset + local)
}
