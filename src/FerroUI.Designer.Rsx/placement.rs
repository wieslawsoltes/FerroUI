use crate::source_edits::remove_field;
use crate::{RsxProjection, xaml::XamlTree};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};

impl RsxProjection<'_> {
    pub fn insert_child(
        &self,
        parent: usize,
        before: Option<usize>,
        fragment: &str,
    ) -> Result<TextPatch, String> {
        let document = Document::new(fragment).map_err(|error| error.to_string())?;
        let tree = XamlTree::parse(&document, self.catalog)?;
        self.insert_element(parent, before, &tree.tree.rsx_body(), true)
    }

    /// Move the original subtree tokens. Layout authoring may update literals on
    /// the moved root; other expressions and comments travel with the element.
    pub fn move_child(
        &self,
        index: usize,
        parent: usize,
        before: Option<usize>,
        mut values: Vec<TextPatch>,
    ) -> Result<Vec<TextPatch>, String> {
        let span = self.nodes.get(index).ok_or("Unknown RSX element")?.span;
        let mut element = self
            .source
            .get(span.range())
            .ok_or("Unknown RSX source span")?
            .to_owned();
        values.sort_by_key(|patch| std::cmp::Reverse(patch.span.start));
        for patch in values {
            if patch.span.start < span.start || patch.span.end > span.end {
                return Err("A layout change escaped the moved Rust element".into());
            }
            element.replace_range(
                patch.span.start - span.start..patch.span.end - span.start,
                &patch.replacement,
            );
        }
        let insertion = self.insert_element(parent, before, &element, false)?;
        let removal = remove_field(self.source, span);
        if removal.span.start <= insertion.span.start && insertion.span.start < removal.span.end {
            return Err("The move destination overlaps its Rust element".into());
        }
        Ok(vec![removal, insertion])
    }

    fn insert_element(
        &self,
        parent: usize,
        before: Option<usize>,
        element: &str,
        generated: bool,
    ) -> Result<TextPatch, String> {
        let parent = self
            .nodes
            .get(parent)
            .filter(|node| node.editable_children())
            .ok_or("Choose a Rust container with fully mapped children")?;
        let offset = match before {
            Some(before) => {
                self.nodes
                    .get(before)
                    .ok_or("Unknown Rust insertion anchor")?
                    .span
                    .start
            }
            None => parent.close,
        };
        if !(parent.open..=parent.close).contains(&offset) {
            return Err("The Rust insertion anchor is outside its container".into());
        }
        let line = self.source[..offset].rsplit('\n').next().unwrap_or("");
        let replacement = if line.trim().is_empty() {
            let eol = if self.source.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let indent = if before.is_some() {
                line.to_owned()
            } else {
                format!("{line}    ")
            };
            let prefix = if before.is_none() { "    " } else { "" };
            let element = if generated {
                element.replace('\n', &format!("{eol}{indent}"))
            } else {
                // Preserve literals and comments inside the moved block byte for byte.
                element.to_owned()
            };
            format!("{prefix}{element}{eol}{line}")
        } else {
            format!(" {element}, ")
        };
        Ok(TextPatch {
            span: SourceSpan::new(offset, offset),
            replacement,
        })
    }
}
