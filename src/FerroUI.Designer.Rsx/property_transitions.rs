//! Static property-transition collections declared on authored Rust controls.
mod edit;
mod syntax;
#[cfg(test)]
mod tests;

pub use edit::{RsxPropertyTransitionChoice, RsxPropertyTransitionEdit};

use crate::animation::syntax::span;
use crate::local_rust::LocalRust;
use crate::registered::Class;
use crate::source_array::ArraySlot;
use crate::RsxCatalog;
use ferroui_designer_animation::Easing;
use ferroui_designer_core::SourceSpan;
use syn::Expr;

#[derive(Debug)]
pub struct RsxPropertyTransitions {
    pub source: SourceSpan,
    pub entries: Vec<RsxPropertyTransition>,
    array: ArraySlot,
}

#[derive(Debug)]
pub struct RsxPropertyTransition {
    pub source: SourceSpan,
    pub property: String,
    pub type_name: &'static str,
    pub duration: f64,
    pub easing: Option<Easing>,
    getter: String,
    comma: Option<SourceSpan>,
    operands: syntax::Operands,
    markup: String,
}

impl RsxPropertyTransitions {
    pub(crate) fn read(
        value: &Expr,
        class: &Class,
        catalog: &RsxCatalog,
        bindings: &LocalRust,
    ) -> Option<Self> {
        let Expr::Array(array) = syntax::collection(value, bindings)? else {
            return None;
        };
        if !array.attrs.is_empty() {
            return None;
        }
        let entries = array
            .elems
            .pairs()
            .map(|entry| {
                syntax::entry(
                    entry.value(),
                    entry.punct().map(span),
                    class,
                    catalog,
                    bindings,
                )
            })
            .collect::<Option<_>>()?;
        Some(Self {
            source: span(value),
            entries,
            array: ArraySlot::at(array),
        })
    }

    pub(crate) fn render(&self, source: &mut String, tag: &str, indent: &str) {
        use std::fmt::Write;
        let _ = write!(source, "\n{indent}    <{tag}.Transitions><Transitions>");
        for entry in &self.entries {
            let _ = write!(source, "\n{indent}        {}", entry.markup);
        }
        let _ = write!(source, "\n{indent}    </Transitions></{tag}.Transitions>");
    }
}
