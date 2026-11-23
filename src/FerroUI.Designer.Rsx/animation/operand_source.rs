//! Replace a literal or constant path at its use site.

use super::syntax::span;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

#[derive(Clone, Debug)]
pub(super) struct OperandSource {
    value: SourceSpan,
    qualifiers: Option<Box<[SourceSpan]>>,
}

impl OperandSource {
    pub(super) fn literal(value: SourceSpan) -> Self {
        Self {
            value,
            qualifiers: None,
        }
    }

    pub(super) fn reference(expression: &Expr) -> Option<Self> {
        let Expr::Path(path) = expression else {
            return None;
        };
        if !path.attrs.is_empty()
            || path.qself.is_some()
            || path
                .path
                .segments
                .iter()
                .any(|segment| !segment.arguments.is_empty())
        {
            return None;
        }

        // Join adjacent path tokens and leave comments between them in place.
        let mut spans: Vec<SourceSpan> = Vec::new();
        let mut push = |span: SourceSpan| {
            if let Some(last) = spans.last_mut().filter(|last| last.end == span.start) {
                last.end = span.end;
            } else {
                spans.push(span);
            }
        };
        if let Some(leading) = &path.path.leading_colon {
            push(span(leading));
        }
        for segment in path.path.segments.pairs() {
            push(span(&segment.value().ident));
            if let Some(separator) = segment.punct() {
                push(span(separator));
            }
        }
        let value = spans.pop()?;
        let qualifiers = (!spans.is_empty()).then(|| spans.into_boxed_slice());
        Some(Self { value, qualifiers })
    }

    pub(super) fn anchor(&self) -> usize {
        self.value.start
    }

    pub(super) fn plan(&self, replacement: String, patches: &mut Vec<TextPatch>) {
        patches.push(TextPatch {
            span: self.value,
            replacement,
        });
        if let Some(qualifiers) = &self.qualifiers {
            patches.extend(qualifiers.iter().map(|&span| TextPatch {
                span,
                replacement: String::new(),
            }));
        }
    }
}
