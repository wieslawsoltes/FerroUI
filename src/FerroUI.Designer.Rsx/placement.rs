use crate::source_edits::remove_field;
use crate::{RsxProjection, xaml::XamlTree};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use std::collections::BTreeSet;

pub(crate) enum ChildBoundary {
    Before(usize),
    After(usize),
    End,
}

impl RsxProjection<'_> {
    pub fn insert_child(
        &self,
        parent: usize,
        before: Option<usize>,
        fragment: &str,
    ) -> Result<TextPatch, String> {
        let element = self.child_fragment(fragment)?;
        self.insert_elements(
            parent,
            before.map_or(ChildBoundary::End, ChildBoundary::Before),
            &[(element, true)],
            &BTreeSet::new(),
        )
    }

    /// Move the original subtree tokens. Layout authoring may update literals on
    /// the moved root; other expressions and comments travel with the element.
    pub fn move_child(
        &self,
        index: usize,
        parent: usize,
        before: Option<usize>,
        values: Vec<TextPatch>,
    ) -> Result<Vec<TextPatch>, String> {
        let span = self.nodes.get(index).ok_or("Unknown RSX element")?.span;
        let element = self.moved_child(span, values)?;
        let insertion = self.insert_elements(
            parent,
            before.map_or(ChildBoundary::End, ChildBoundary::Before),
            &[(element, false)],
            &BTreeSet::new(),
        )?;
        let removal = remove_field(self.source, span);
        if removal.span.start <= insertion.span.start && insertion.span.start < removal.span.end {
            return Err("The move destination overlaps its Rust element".into());
        }
        Ok(vec![removal, insertion])
    }

    pub(crate) fn child_fragment(&self, fragment: &str) -> Result<String, String> {
        let document = Document::new(fragment).map_err(|error| error.to_string())?;
        Ok(XamlTree::parse(&document, self.catalog)?.tree.rsx_body())
    }

    pub(crate) fn moved_child(
        &self,
        span: SourceSpan,
        mut edits: Vec<TextPatch>,
    ) -> Result<String, String> {
        for edit in &mut edits {
            if edit.span.start < span.start || edit.span.end > span.end {
                return Err("A layout change escaped the moved Rust element".into());
            }
            edit.span.start -= span.start;
            edit.span.end -= span.start;
        }
        TextPatch::apply_all(&self.source[span.range()], edits).map_err(|error| error.to_string())
    }

    pub(crate) fn insert_elements(
        &self,
        parent: usize,
        boundary: ChildBoundary,
        elements: &[(String, bool)],
        moving: &BTreeSet<usize>,
    ) -> Result<TextPatch, String> {
        let parent = self
            .nodes
            .get(parent)
            .filter(|node| node.editable_children())
            .ok_or("Choose an authored Rust container with an editable child slot")?;
        let child = |index: usize| {
            parent
                .child_indices
                .contains(&index)
                .then(|| &self.nodes[index])
                .ok_or("The Rust anchor is outside its child list")
        };
        let offset = match boundary {
            ChildBoundary::Before(index) => child(index)?.span.start,
            ChildBoundary::After(index) => {
                let end = child(index)?.span.end;
                parent
                    .child_indices
                    .iter()
                    .filter(|index| !moving.contains(index))
                    .map(|index| self.nodes[*index].span.start)
                    .chain(parent.opaque_children.iter().copied())
                    .filter(|start| *start >= end)
                    .min()
                    .unwrap_or(parent.close)
            }
            ChildBoundary::End => parent.close,
        };
        let line = self.source[..offset].rsplit('\n').next().unwrap_or("");
        let eol = if self.source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let replacement = if line.trim().is_empty() {
            let extra = if offset == parent.close { "    " } else { "" };
            let next_line = format!("{eol}{line}{extra}");
            let items = elements
                .iter()
                .map(|(element, generated)| {
                    if *generated {
                        element.replace('\n', &next_line)
                    } else {
                        element.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(&next_line);
            format!("{extra}{items}{eol}{line}")
        } else {
            let elements = elements
                .iter()
                .map(|(element, generated)| {
                    if *generated {
                        element.replace('\n', eol)
                    } else {
                        element.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(" {elements}, ")
        };
        Ok(TextPatch {
            span: SourceSpan::new(offset, offset),
            replacement,
        })
    }
}
