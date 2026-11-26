use crate::{
    blocks::keep_comments,
    child_collection::ChildEnvelope,
    source_edits::{remove_field, reset_field},
};
use crate::{xaml::XamlTree, RsxProjection};
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
        let source_parent = self
            .nodes
            .iter()
            .position(|node| node.editable_children() && node.child_indices.contains(&index))
            .ok_or("The Rust control needs an authored parent")?;
        let moving = BTreeSet::from([index]);
        let element = self.moved_child(index, parent, values)?;
        let insertion = self.insert_elements(
            parent,
            before.map_or(ChildBoundary::End, ChildBoundary::Before),
            &[(element, false)],
            &moving,
        )?;
        let mut patches = self.detach_children(source_parent, &[index], source_parent != parent)?;
        if patches.iter().any(|removal| {
            removal.span.start <= insertion.span.start && insertion.span.start < removal.span.end
        }) {
            return Err("The move destination overlaps its Rust source".into());
        }
        patches.push(insertion);
        Ok(patches)
    }

    pub(crate) fn child_fragment(&self, fragment: &str) -> Result<String, String> {
        let document = Document::new(fragment).map_err(|error| error.to_string())?;
        Ok(XamlTree::parse(&document, self.catalog)?.tree.rsx_body())
    }

    pub(crate) fn moved_child(
        &self,
        index: usize,
        parent: usize,
        mut edits: Vec<TextPatch>,
    ) -> Result<String, String> {
        let node = self.nodes.get(index).ok_or("Unknown RSX element")?;
        let destination = self.nodes.get(parent).ok_or("Unknown RSX container")?;
        let span = node.child_span();
        for edit in &mut edits {
            if edit.span.start < span.start || edit.span.end > span.end {
                return Err("A layout change escaped the moved Rust element".into());
            }
            edit.span.start -= span.start;
            edit.span.end -= span.start;
        }
        let source = TextPatch::apply_all(&self.source[span.range()], edits)
            .map_err(|error| error.to_string())?;
        Ok(
            match (node.envelope, destination.child_collection.is_some()) {
                (None, true) => wrap_invocation(&source),
                (Some(ChildEnvelope::Collection(_)), false) => format!("{{ {source} }}"),
                _ => source,
            },
        )
    }

    pub(crate) fn detach_children(
        &self,
        parent: usize,
        children: &[usize],
        can_empty: bool,
    ) -> Result<Vec<TextPatch>, String> {
        let node = &self.nodes[parent];
        let mut spans: Vec<_> = children
            .iter()
            .map(|&index| self.nodes[index].child_span())
            .collect();
        if let Some(collection) = node.child_collection {
            if can_empty && collection.len == spans.len() {
                // An empty Rust array passed to IntoChildren has no inferred item type.
                // Remove the assignment and keep every comment outside the moved entries.
                let mut patch = reset_field(self.source, collection.assignment, node.close)?;
                spans.sort_by_key(|span| std::cmp::Reverse(span.start));
                let mut retained = self.source[patch.span.range()].to_owned();
                for span in spans {
                    retained.replace_range(
                        span.start - patch.span.start..span.end - patch.span.start,
                        "",
                    );
                }
                patch.replacement = keep_comments(&retained)?;
                return Ok(vec![patch]);
            }
            return spans
                .into_iter()
                .map(|span| {
                    let mut patch = reset_field(self.source, span, collection.close)?;
                    patch.replacement = keep_comments(&self.source[span.end..patch.span.end])?;
                    Ok(patch)
                })
                .collect();
        }
        Ok(spans
            .into_iter()
            .map(|span| remove_field(self.source, span))
            .collect())
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
        let close = parent
            .child_collection
            .map_or(parent.close, |collection| collection.close);
        let offset = match boundary {
            ChildBoundary::Before(index) => child(index)?.child_span().start,
            ChildBoundary::After(index) => {
                let end = child(index)?.child_span().end;
                parent
                    .child_indices
                    .iter()
                    .filter(|index| !moving.contains(index))
                    .map(|index| self.nodes[*index].child_span().start)
                    .chain(parent.opaque_children.iter().copied())
                    .filter(|start| *start >= end)
                    .min()
                    .unwrap_or(close)
            }
            ChildBoundary::End => close,
        };
        let line = self.source[..offset].rsplit('\n').next().unwrap_or("");
        let eol = if self.source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let in_collection = parent.child_collection.is_some();
        let render = |(element, generated): &(String, bool), newline: &str| {
            if !generated {
                return element.clone();
            }
            let element = element.replace('\n', newline);
            if in_collection {
                wrap_invocation(&element)
            } else {
                element
            }
        };
        let mut replacement = if line.trim().is_empty() {
            let extra = if offset == close { "    " } else { "" };
            let next_line = format!("{eol}{line}{extra}");
            let items = elements
                .iter()
                .map(|element| {
                    let mut item = render(element, &next_line);
                    if in_collection {
                        item.push(',');
                    }
                    item
                })
                .collect::<Vec<_>>()
                .join(&next_line);
            format!("{extra}{items}{eol}{line}")
        } else {
            let elements = elements
                .iter()
                .map(|element| render(element, eol))
                .collect::<Vec<_>>()
                .join(", ");
            format!(" {elements}, ")
        };
        let mut start = offset;
        if let Some(last) = parent
            .child_collection
            .and_then(|collection| collection.unterminated)
        {
            if offset == close
                && !moving
                    .iter()
                    .any(|&index| self.nodes[index].child_span() == last)
            {
                start = last.end;
                replacement = format!(",{}{replacement}", &self.source[start..offset]);
            }
        }
        Ok(TextPatch {
            span: SourceSpan::new(start, offset),
            replacement,
        })
    }
}

fn wrap_invocation(body: &str) -> String {
    format!("::ferroui_view::rsx! {{ {body} }}")
}
