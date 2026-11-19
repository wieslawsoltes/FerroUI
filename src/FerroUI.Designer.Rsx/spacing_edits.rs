use crate::blocks::keep_comments;
use crate::literal::Literal;
use crate::RsxProjection;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::spanned::Spanned;
use syn::ExprTuple;

impl RsxProjection<'_> {
    /// Check whether a native spacing edit can preserve the mapped Rust literal.
    pub fn spacing_property_support(&self, index: usize, name: &str) -> Result<(), &'static str> {
        let node = self
            .nodes
            .get(index)
            .filter(|node| node.supports_literal(name))
            .ok_or("This property has no mapped RSX literal.")?;
        if node.expressions.contains_key(name) {
            return Err("This property is a Rust expression. Edit it in source.");
        }
        if let Some(span) = node.properties.get(name) {
            let original = &self.source[span.range()];
            let has_comment = |text| keep_comments(text).is_ok_and(|comments| !comments.is_empty());
            if has_comment(original)
                && !four_sides(name, original).is_some_and(|sides| {
                    sides
                        .elems
                        .iter()
                        .all(|side| !has_comment(&original[side.span().byte_range()]))
                })
            {
                return Err("Edit this value in Rust to preserve the comments within it.");
            }
        }
        Ok(())
    }

    /// Use the registered property codec, preserving all surrounding Rust and unchanged sides.
    pub fn edit_spacing_property(
        &self,
        index: usize,
        name: &str,
        value: &str,
    ) -> Result<Vec<TextPatch>, String> {
        self.spacing_property_support(index, name)?;
        let patch = self.edit_property(index, name, Some(value))?;
        let original = &self.source[patch.span.range()];
        let (Some(old), Some(new)) = (
            four_sides(name, original),
            four_sides(name, &patch.replacement),
        ) else {
            return Ok(vec![patch]);
        };
        Ok(old
            .elems
            .iter()
            .zip(&new.elems)
            .filter_map(|(old, new)| {
                if Literal::Number.read_rust(old) == Literal::Number.read_rust(new) {
                    return None;
                }
                let range = old.span().byte_range();
                Some(TextPatch {
                    span: SourceSpan::new(
                        patch.span.start + range.start,
                        patch.span.start + range.end,
                    ),
                    replacement: patch.replacement[new.span().byte_range()].into(),
                })
            })
            .collect())
    }
}

fn four_sides(name: &str, source: &str) -> Option<ExprTuple> {
    if !matches!(name, "Margin" | "Padding") {
        return None;
    }
    syn::parse_str::<ExprTuple>(source)
        .ok()
        .filter(|tuple| tuple.elems.len() == 4)
}
