use super::syntax;
use crate::blocks::keep_comments;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, spanned::Spanned};

/// An enum variant may be qualified by its type, imported by name, or renamed.
#[derive(Clone, Debug)]
pub(super) struct VariantSource {
    span: SourceSpan,
    expanded: Option<String>,
}

impl VariantSource {
    pub(super) fn read(
        expression: &Expr,
        owner: &str,
        variant: &str,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<Self> {
        let Expr::Path(site) = expression else {
            return None;
        };
        let last = site.path.segments.last()?;
        if site.path.segments.len() > 1 && last.ident == variant {
            let mut prefix = site.clone();
            prefix.path.segments.pop();
            prefix.path.segments.pop_punct();
            if resolve(&Expr::Path(prefix)).as_deref() == Some(owner) {
                return Some(Self {
                    span: syntax::span(&last.ident),
                    expanded: None,
                });
            }
        }
        Some(Self {
            span: syntax::span(expression),
            expanded: Some(keep_comments(&expression.span().source_text()?).ok()?),
        })
    }

    pub(super) fn plan(&self, owner: &str, variant: &str, patches: &mut Vec<TextPatch>) {
        let replacement = self.expanded.as_ref().map_or_else(
            || variant.into(),
            |comments| format!("{comments}::{owner}::{variant}"),
        );
        patches.push(TextPatch {
            span: self.span,
            replacement,
        });
    }
}
