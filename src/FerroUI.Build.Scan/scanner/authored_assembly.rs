//! Re-locate the selected assembly's metadata in the current Rust buffer.
use super::super::source::{field_of, last_segment_of_type, pair_expressions};
use super::{module_items, rust_body};
use crate::authored_symbols::{AuthoredAssembly, AuthoredMetadata, AuthoredText};
use std::ops::Range;
use syn::{spanned::Spanned, Expr, Item, Lit, LitStr};

#[path = "metadata_constants.rs"]
mod constants;
use constants::MetadataConstants;

impl AuthoredAssembly {
    /// Exact tuple operands in the assembly's metadata array. A text constant
    /// in this file can be edited at its use without changing its definition.
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
        let constants = MetadataConstants::new(&file, &self.modules)?;
        Some(
            pairs
                .into_iter()
                .map(|(key, value)| AuthoredMetadata {
                    key: authored_text(&constants, key, offset),
                    value: authored_text(&constants, value, offset),
                })
                .collect(),
        )
    }
}

fn authored_text(constants: &MetadataConstants<'_>, expr: &Expr, offset: usize) -> Option<AuthoredText> {
    let operand = ungroup(expr);
    let literal = match operand {
        Expr::Path(path) => constants.literal(path)?,
        other => string_literal(other)?,
    };
    let shifted = |range: Range<usize>| (range.start + offset)..(range.end + offset);
    Some(AuthoredText {
        value: literal.value(),
        range: shifted(operand.span().byte_range()),
        literal: shifted(literal.span().byte_range()),
    })
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
