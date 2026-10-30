//! Source-preserving edit planning and one-pass patch application.

use super::{parse_snapshot, DesignerError, Edit, Snapshot, TextPatch};
use crate::{Ast, Node, NodeId, SourceSpan};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn apply_edits(
    mut candidate: Snapshot,
    edits: Vec<Edit>,
    next_id: &mut NodeId,
) -> Result<Snapshot, DesignerError> {
    let mut edits = edits.into_iter().peekable();
    while let Some(edit) = edits.next() {
        if is_attribute_edit(&edit) {
            let mut attributes = vec![edit];
            while edits.peek().is_some_and(is_attribute_edit) {
                attributes.push(edits.next().unwrap());
            }
            candidate = apply_attribute_edits(candidate, attributes, next_id)?;
        } else {
            candidate = apply_edit(candidate, edit, next_id)?;
        }
    }
    Ok(candidate)
}

fn is_attribute_edit(edit: &Edit) -> bool {
    matches!(
        edit,
        Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. }
    )
}

fn element(ast: &Ast, id: NodeId) -> Result<&Node, DesignerError> {
    let node = ast.node(id).ok_or(DesignerError::UnknownNode(id))?;
    if !node.is_element() {
        return Err(DesignerError::NotElement(id));
    }
    Ok(node)
}

/// Multi-selection property edits are planned against one snapshot and parsed
/// once. Last write to a property wins within the transaction. This also permits
/// an attribute and its namespace declaration to be authored atomically.
fn apply_attribute_edits(
    old: Snapshot,
    edits: Vec<Edit>,
    next_id: &mut NodeId,
) -> Result<Snapshot, DesignerError> {
    let mut values = BTreeMap::<NodeId, Vec<(String, Option<String>)>>::new();
    for edit in edits {
        let (id, name, value) = match edit {
            Edit::SetAttribute { node, name, value } => {
                validate_name(&name)?;
                (node, name, Some(value))
            }
            Edit::RemoveAttribute { node, name } => (node, name, None),
            _ => unreachable!("caller filters attribute operations"),
        };
        element(&old.ast, id)?;
        let attributes = values.entry(id).or_default();
        if let Some(attribute) = attributes
            .iter_mut()
            .find(|(existing, _)| *existing == name)
        {
            attribute.1 = value;
        } else {
            attributes.push((name, value));
        }
    }
    let mut patches = Vec::new();
    for (id, values) in values {
        let node = element(&old.ast, id)?;
        let quote = node.attributes.first().map_or('"', |a| a.quote);
        let mut added = String::new();
        for (name, value) in values {
            match (node.attribute_info(&name), value) {
                (Some(attribute), Some(value)) if attribute.value != value => {
                    patches.push(TextPatch {
                        span: attribute.value_span,
                        replacement: escape_attribute(&value, attribute.quote),
                    })
                }
                (Some(attribute), None) => patches.push(TextPatch {
                    span: attribute.span,
                    replacement: String::new(),
                }),
                (None, Some(value)) => added.push_str(&format!(
                    " {name}={quote}{}{quote}",
                    escape_attribute(&value, quote)
                )),
                _ => {}
            }
        }
        if !added.is_empty() {
            let offset = node.opening_span.end - if node.self_closing { 2 } else { 1 };
            patches.push(TextPatch {
                span: SourceSpan::new(offset, offset),
                replacement: added,
            });
        }
    }
    if patches.is_empty() {
        return Ok(old);
    }
    let (source, pins) = patched_source(&old.source, &old.ast, patches, &[])?;
    parse_snapshot(source, &old, next_id, &pins)
}

fn apply_edit(old: Snapshot, edit: Edit, next_id: &mut NodeId) -> Result<Snapshot, DesignerError> {
    let mut extra_pins = Vec::new();
    let patches = match edit {
        Edit::ReplaceSource { source } => return parse_snapshot(source, &old, next_id, &[]),
        Edit::Text { span, replacement } => vec![TextPatch { span, replacement }],
        Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. } => {
            unreachable!("attribute runs are planned together by apply_edits")
        }
        Edit::InsertChild {
            parent,
            index,
            source,
        } => {
            let parent = element(&old.ast, parent)?;
            let children = old.ast.element_children(parent.id);
            if index > children.len() {
                return Err(DesignerError::InvalidEdit(format!(
                    "Child index {index} exceeds {}",
                    children.len()
                )));
            }
            if source.is_empty() {
                return Ok(old);
            }
            validate_fragment(&old.ast, parent.id, &source)?;
            insertion_patches(parent, &old.ast, &children, index, &source)?.0
        }
        Edit::RemoveNode { node } => {
            if node == old.ast.root() {
                return Err(DesignerError::InvalidEdit(
                    "Cannot remove the document root".into(),
                ));
            }
            let node = old.ast.node(node).ok_or(DesignerError::UnknownNode(node))?;
            vec![TextPatch {
                span: node.span,
                replacement: String::new(),
            }]
        }
        Edit::MoveNode {
            node,
            parent,
            index,
        } => {
            if node == old.ast.root() {
                return Err(DesignerError::InvalidEdit(
                    "Cannot move the document root".into(),
                ));
            }
            if node == parent || old.ast.is_descendant_of(parent, node) {
                return Err(DesignerError::InvalidEdit(
                    "Moving this node would create an ancestor cycle".into(),
                ));
            }
            let node = element(&old.ast, node)?;
            let parent = element(&old.ast, parent)?;
            let children: Vec<_> = old
                .ast
                .element_children(parent.id)
                .into_iter()
                .filter(|id| *id != node.id)
                .collect();
            if index > children.len() {
                return Err(DesignerError::InvalidEdit(format!(
                    "Child index {index} exceeds {}",
                    children.len()
                )));
            }
            let current = old
                .ast
                .element_children(parent.id)
                .iter()
                .position(|id| *id == node.id);
            if node.parent == Some(parent.id) && current == Some(index) {
                return Ok(old);
            }
            let mut fragment = old.source[node.span.range()].to_owned();
            // Preserve the namespace environment when reparenting into a template
            // or subtree with different namespace declarations.
            let mut old_scope = old.ast.namespaces(node.id);
            // An absent default namespace is a meaningful part of the source
            // environment, including when the moved root itself is prefixed.
            // Materialize it once so unqualified descendants stay unqualified.
            old_scope.entry(String::new()).or_default();
            let new_scope = old.ast.namespaces(parent.id);
            let mut declarations = String::new();
            for (prefix, uri) in old_scope {
                let destination = new_scope.get(&prefix).map(String::as_str).unwrap_or("");
                if prefix == "xml" || destination == uri {
                    continue;
                }
                let attr_name = if prefix.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{prefix}")
                };
                if node.attribute(&attr_name).is_none() {
                    declarations
                        .push_str(&format!(" {attr_name}=\"{}\"", escape_attribute(&uri, '"')));
                }
            }
            let declaration_offset =
                node.opening_span.end - node.span.start - if node.self_closing { 2 } else { 1 };
            fragment.insert_str(declaration_offset, &declarations);
            let (mut patches, insert_offset, fragment_offset) =
                insertion_patches(parent, &old.ast, &children, index, &fragment)?;
            patches.push(TextPatch {
                span: node.span,
                replacement: String::new(),
            });
            // Pins are represented in old-source insertion coordinates here and
            // adjusted below after both deletion and insertion are applied.
            let deletion_delta = if node.span.end <= insert_offset {
                node.span.len()
            } else {
                0
            };
            let new_start = insert_offset - deletion_delta + fragment_offset;
            for descendant in old.ast.subtree(node.id) {
                let relative = descendant.span.start - node.span.start;
                let adjustment = if relative >= declaration_offset {
                    declarations.len()
                } else {
                    0
                };
                extra_pins.push((new_start + relative + adjustment, descendant.id));
            }
            patches
        }
    };
    let (source, mut pins) = patched_source(&old.source, &old.ast, patches, &extra_pins)?;
    pins.extend(extra_pins);
    parse_snapshot(source, &old, next_id, &pins)
}

/// Validate the fragment in its destination namespace scope. Validating only the
/// completed document would allow a fragment to close its parent and author
/// siblings outside the requested insertion boundary.
fn validate_fragment(ast: &Ast, parent: NodeId, fragment: &str) -> Result<(), DesignerError> {
    let mut source = String::from("<__designer_fragment__");
    for (prefix, uri) in ast.namespaces(parent) {
        if prefix == "xml" {
            continue;
        }
        let name = if prefix.is_empty() {
            "xmlns".to_owned()
        } else {
            format!("xmlns:{prefix}")
        };
        source.push_str(&format!(" {name}='{}'", escape_attribute(&uri, '\'')));
    }
    source.push('>');
    source.push_str(fragment);
    source.push_str("</__designer_fragment__>");
    crate::ast::validate_xml(&source)
        .map_err(|error| DesignerError::InvalidEdit(format!("Invalid child fragment: {error}")))
}

fn insertion_patches(
    parent: &Node,
    ast: &Ast,
    children: &[NodeId],
    index: usize,
    fragment: &str,
) -> Result<(Vec<TextPatch>, usize, usize), DesignerError> {
    if index > children.len() {
        return Err(DesignerError::InvalidEdit(format!(
            "Child index {index} exceeds {}",
            children.len()
        )));
    }
    if parent.self_closing {
        let start = parent.opening_span.end - 2;
        Ok((
            vec![TextPatch {
                span: SourceSpan::new(start, parent.opening_span.end),
                replacement: format!(">{fragment}</{}>", parent.name),
            }],
            start,
            1,
        ))
    } else {
        let offset = children
            .get(index)
            .and_then(|id| ast.node(*id))
            .map_or_else(|| parent.closing_span.unwrap().start, |n| n.span.start);
        Ok((
            vec![TextPatch {
                span: SourceSpan::new(offset, offset),
                replacement: fragment.to_owned(),
            }],
            offset,
            0,
        ))
    }
}

/// Apply nonoverlapping edits and map surviving node starts through their offset
/// deltas. Replaced nodes fall back to semantic reconciliation.
pub(super) fn patched_source(
    source: &str,
    ast: &Ast,
    mut patches: Vec<TextPatch>,
    extra_pins: &[(usize, NodeId)],
) -> Result<(String, Vec<(usize, NodeId)>), DesignerError> {
    patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
    for (index, patch) in patches.iter().enumerate() {
        let span = patch.span;
        if span.start > span.end
            || span.end > source.len()
            || !source.is_char_boundary(span.start)
            || !source.is_char_boundary(span.end)
        {
            return Err(DesignerError::InvalidRange(span));
        }
        if index > 0 {
            let previous = patches[index - 1].span;
            if span.start < previous.end
                || (span.start == previous.start && span.is_empty() && previous.is_empty())
            {
                return Err(DesignerError::InvalidEdit(
                    "Source patches overlap or have ambiguous insertion order".into(),
                ));
            }
        }
    }
    // Validate the complete request before dropping lexical no-ops: malformed
    // or overlapping ranges must never become valid merely by having no effect.
    patches.retain(|patch| source[patch.span.range()] != patch.replacement);
    if patches.is_empty() {
        return Ok((source.to_owned(), Vec::new()));
    }
    let removed: usize = patches.iter().map(|patch| patch.span.len()).sum();
    let final_len = patches
        .iter()
        .try_fold(source.len() - removed, |len, patch| {
            len.checked_add(patch.replacement.len())
                .ok_or_else(|| DesignerError::InvalidEdit("Patched source is too large".into()))
        })?;
    let mut result = String::with_capacity(final_len);
    let mut cursor = 0;
    let mut prefix_deltas = Vec::with_capacity(patches.len() + 1);
    prefix_deltas.push(0i128);
    for patch in &patches {
        result.push_str(&source[cursor..patch.span.start]);
        result.push_str(&patch.replacement);
        cursor = patch.span.end;
        prefix_deltas.push(
            prefix_deltas.last().unwrap() + patch.replacement.len() as i128
                - patch.span.len() as i128,
        );
    }
    result.push_str(&source[cursor..]);
    let explicitly_pinned: BTreeSet<_> = extra_pins.iter().map(|(_, id)| *id).collect();
    let mut pins = Vec::new();
    for node in ast.nodes() {
        if explicitly_pinned.contains(&node.id) {
            continue;
        }
        let old_offset = node.span.start;
        // Validated nonoverlapping patches also have nondecreasing ends. The first patch
        // ending after this node is the only one that could replace its start. Include empty
        // insertions at the node boundary in the preceding prefix so surviving nodes move
        // after their inserted siblings. One search replaces two scans per node in a batch.
        let preceding = patches.partition_point(|patch| patch.span.end <= old_offset);
        if patches
            .get(preceding)
            .is_some_and(|patch| patch.span.start <= old_offset)
        {
            continue;
        }
        let offset = old_offset as i128 + prefix_deltas[preceding];
        if offset >= 0 {
            pins.push((offset as usize, node.id));
        }
    }
    Ok((result, pins))
}

fn validate_name(name: &str) -> Result<(), DesignerError> {
    // XML itself validates Unicode NCNames and namespace declarations. Reject
    // delimiters before constructing source to prevent attribute injection.
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '=' | '\'' | '"' | '<' | '>' | '/' | '&'))
    {
        return Err(DesignerError::InvalidEdit(format!(
            "Invalid qualified attribute name {name:?}"
        )));
    }
    Ok(())
}

fn escape_attribute(value: &str, quote: char) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' if quote == '"' => out.push_str("&quot;"),
            '\'' if quote == '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(ch),
        }
    }
    out
}
