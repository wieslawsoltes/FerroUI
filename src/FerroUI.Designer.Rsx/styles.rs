//! Authored Rust styles used by both the native projection and source editors.
mod edit;
pub(crate) mod fields;
mod syntax;
#[cfg(test)]
mod tests;

pub use edit::RsxStyleEdit;
pub use fields::RsxStyleField;
pub(crate) use syntax::projected_styles;

use crate::animation::RsxAnimation;
use crate::RsxProjection;
use ferroui_designer_core::SourceSpan;

#[derive(Debug)]
pub struct RsxStyle {
    pub declaration: SourceSpan,
    pub selector: String,
    pub selector_span: SourceSpan,
    pub setters: Vec<RsxStyleSetter>,
    pub children: Vec<RsxStyle>,
    pub(crate) clips: Vec<RsxAnimation>,
    pub(crate) preview_selector: Option<syntax::PreviewSelector>,
    deletion: Deletion,
    removable: bool,
}

#[derive(Debug)]
pub struct RsxStyleSetter {
    pub declaration: SourceSpan,
    pub source: SourceSpan,
    pub field: Option<RsxStyleField>,
    value: Option<fields::SetterValue>,
}

#[derive(Clone, Debug)]
pub struct RsxStyleIssue {
    pub source: SourceSpan,
    pub reason: &'static str,
}

#[derive(Debug)]
struct Deletion {
    span: SourceSpan,
    comma: Option<SourceSpan>,
}

impl RsxStyle {
    pub fn can_remove(&self) -> bool {
        self.removable
    }

    pub fn is_nested(&self) -> bool {
        self.deletion.span != self.declaration
    }

    pub(crate) fn visit(&self, visitor: &mut impl FnMut(&Self)) {
        visitor(self);
        for child in &self.children {
            child.visit(visitor);
        }
    }
}

impl RsxStyleSetter {
    pub fn value(&self) -> Option<String> {
        self.value.as_ref().map(fields::SetterValue::text)
    }
}

impl<'a> RsxProjection<'a> {
    pub fn style_definitions(&self) -> Vec<(usize, &RsxStyle)> {
        let mut styles = Vec::new();
        for (&owner, rules) in &self.styles {
            for style in rules {
                collect_styles(owner, style, &mut styles);
            }
        }
        styles
    }

    pub fn can_create_style(&self, owner: usize) -> bool {
        self.nodes.get(owner).is_some_and(|node| {
            !matches!(node.style_slot, crate::source_array::ArraySlot::Unavailable)
        })
    }

    pub fn style_fields(&self) -> Vec<&RsxStyleField> {
        self.catalog.style_fields()
    }
}

fn collect_styles<'a>(owner: usize, style: &'a RsxStyle, styles: &mut Vec<(usize, &'a RsxStyle)>) {
    styles.push((owner, style));
    for child in &style.children {
        collect_styles(owner, child, styles);
    }
}

fn string_literal(original: &str, value: &str) -> String {
    if let Some((prefix, _)) = original.split_once('"') {
        if let Some(hashes) = prefix.strip_prefix('r') {
            if hashes.chars().all(|character| character == '#')
                && !value.contains(&format!("\"{hashes}"))
                && !value.contains('\r')
            {
                return format!("{prefix}\"{value}\"{hashes}");
            }
        }
    }
    format!("{value:?}")
}
