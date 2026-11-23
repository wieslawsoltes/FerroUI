//! Static property-transition collections declared on authored Rust controls.
mod syntax;
#[cfg(test)]
mod tests;

use crate::animation::syntax::span;
use crate::local_rust::LocalRust;
use crate::registered::Class;
use crate::RsxCatalog;
use ferroui_designer_animation::Easing;
use ferroui_designer_core::SourceSpan;
use syn::Expr;

#[derive(Debug)]
pub struct RsxPropertyTransitions {
    pub source: SourceSpan,
    pub entries: Vec<RsxPropertyTransition>,
}

#[derive(Debug)]
pub struct RsxPropertyTransition {
    pub source: SourceSpan,
    pub property: String,
    pub type_name: &'static str,
    pub duration: f64,
    pub easing: Option<Easing>,
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
            .iter()
            .map(|entry| syntax::entry(entry, class, catalog, bindings))
            .collect::<Option<_>>()?;
        Some(Self {
            source: span(value),
            entries,
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
