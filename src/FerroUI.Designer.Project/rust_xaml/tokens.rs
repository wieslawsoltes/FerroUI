use super::rust_body;
use crate::{ProjectError, Result, require};
use ferroui_designer_core::TextPatch;
use proc_macro2::{TokenStream, TokenTree};
use std::collections::BTreeSet;

/// Confirm that every rename is at a Rust identifier token, including tokens
/// passed to macros. Skip strings, comments, and other non-identifier tokens.
pub fn check_rust_rename_tokens(source: &str, edits: &[TextPatch]) -> Result<()> {
    let body = rust_body(source);
    let offset = source.len() - body.len();
    let mut outstanding: BTreeSet<_> = edits
        .iter()
        .map(|edit| (edit.span.start, edit.span.end))
        .collect();
    let tokens: TokenStream = body
        .parse()
        .map_err(|error: proc_macro2::LexError| ProjectError(error.to_string()))?;
    let mut scopes = vec![tokens];
    while let Some(scope) = scopes.pop() {
        for token in scope {
            match token {
                TokenTree::Group(group) => scopes.push(group.stream()),
                TokenTree::Ident(ident) => {
                    let range = ident.span().byte_range();
                    outstanding.remove(&(offset + range.start, offset + range.end));
                }
                _ => {}
            }
        }
    }
    require(
        outstanding.is_empty(),
        "A semantic Rust rename must replace whole identifier tokens",
    )
}
