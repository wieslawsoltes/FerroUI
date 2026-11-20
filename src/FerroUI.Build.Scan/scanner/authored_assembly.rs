//! Re-locate the selected assembly's metadata in the current Rust buffer.
use super::super::source::{field_of, last_segment_of_type, pair_expressions};
use super::{module_items, rust_body};
use crate::authored_symbols::{AuthoredAssembly, AuthoredMetadata, AuthoredText};
use std::ops::Range;
use syn::{spanned::Spanned, Expr, Item, Lit, LitStr};

impl AuthoredAssembly {
    /// Exact tuple operands in the assembly's metadata array. A text constant
    /// in the same module can be edited at its use without changing its definition.
    pub fn metadata_in(&self, source: &str) -> Option<Vec<AuthoredMetadata>> {
        let body = rust_body(source);
        let offset = source.len() - body.len();
        let file = syn::parse_str::<syn::File>(body).ok()?;
        let items = module_items(&file, &self.modules)?;
        let mut candidates = items.iter().filter_map(|item| match item {
            Item::Static(item) if item.ident == self.item => Some(item),
            _ => None,
        });
        let assembly = candidates.next()?;
        if candidates.next().is_some()
            || last_segment_of_type(&assembly.ty).as_deref() != Some("MarkupAssembly")
        {
            return None;
        }
        let Expr::Struct(value) = assembly.expr.as_ref() else {
            return None;
        };
        let pairs = pair_expressions(field_of(value, "metadata")?)?;
        Some(
            pairs
                .into_iter()
                .map(|(key, value)| AuthoredMetadata {
                    key: authored_text(items, key, offset),
                    value: authored_text(items, value, offset),
                })
                .collect(),
        )
    }
}

fn authored_text(items: &[Item], expr: &Expr, offset: usize) -> Option<AuthoredText> {
    let operand = ungroup(expr);
    let literal = match operand {
        Expr::Path(path) if path.qself.is_none() && path.path.leading_colon.is_none() => {
            let name = local_constant(&path.path)?;
            let mut matches = items.iter().filter_map(|item| match item {
                Item::Const(item) if item.ident == *name => Some(item),
                _ => None,
            });
            let item = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            string_literal(ungroup(&item.expr))?
        }
        other => string_literal(other)?,
    };
    let shifted = |range: Range<usize>| (range.start + offset)..(range.end + offset);
    Some(AuthoredText {
        value: literal.value(),
        range: shifted(operand.span().byte_range()),
        literal: shifted(literal.span().byte_range()),
    })
}

fn local_constant(path: &syn::Path) -> Option<&syn::Ident> {
    let last = path.segments.last()?;
    if !last.arguments.is_empty() {
        return None;
    }
    (path.segments.len() == 1 || (path.segments.len() == 2 && path.segments[0].ident == "self"))
        .then_some(&last.ident)
}

fn ungroup(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(inner) => ungroup(&inner.expr),
        Expr::Group(inner) => ungroup(&inner.expr),
        other => other,
    }
}

fn string_literal(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(literal) => match &literal.lit {
            Lit::Str(value) => Some(value),
            _ => None,
        },
        _ => None,
    }
}
