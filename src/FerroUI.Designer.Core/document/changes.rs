//! Ordered AST comparison and UTF-8-safe source synchronization patches.

use super::{ChangeSet, Snapshot, TextPatch};
use crate::{Node, SourceSpan};
use std::cmp::Ordering;
use std::sync::Arc;

pub(super) fn unchanged(snapshot: &Snapshot, label: String, revision: u64) -> ChangeSet {
    ChangeSet {
        label,
        previous_revision: revision,
        revision,
        inserted: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        patches: Vec::new(),
        valid: snapshot.diagnostics.is_empty(),
    }
}

pub(super) fn diff(
    before: &Snapshot,
    after: &Snapshot,
    label: String,
    previous_revision: u64,
    revision: u64,
) -> ChangeSet {
    let mut change = unchanged(after, label, revision);
    change.previous_revision = previous_revision;
    change
        .patches
        .extend(minimal_patch(&before.source, &after.source));
    if Arc::ptr_eq(&before.ast, &after.ast) {
        return change;
    }

    // Ast::nodes is ordered by stable ID. Merge the two indices directly,
    // without temporary sets or a second map lookup for each surviving node.
    let mut old = before.ast.nodes().peekable();
    let mut new = after.ast.nodes().peekable();
    while let (Some(a), Some(b)) = (old.peek(), new.peek()) {
        match a.id.cmp(&b.id) {
            Ordering::Less => change.removed.push(old.next().unwrap().id),
            Ordering::Greater => change.inserted.push(new.next().unwrap().id),
            Ordering::Equal => {
                if node_changed(a, b) {
                    change.changed.push(a.id);
                }
                old.next();
                new.next();
            }
        }
    }
    change.removed.extend(old.map(|node| node.id));
    change.inserted.extend(new.map(|node| node.id));
    change
}

fn node_changed(before: &Node, after: &Node) -> bool {
    before.parent != after.parent
        || before.children != after.children
        || before.kind != after.kind
        || before.name != after.name
        || before.namespace_uri != after.namespace_uri
        || before.text != after.text
        || before
            .attributes
            .iter()
            .map(|attribute| (&attribute.name, &attribute.namespace_uri, &attribute.value))
            .ne(after
                .attributes
                .iter()
                .map(|attribute| (&attribute.name, &attribute.namespace_uri, &attribute.value)))
}

fn minimal_patch(before: &str, after: &str) -> Option<TextPatch> {
    if before == after {
        return None;
    }
    let mut prefix = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(prefix) || !after.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let mut suffix = before[prefix..]
        .bytes()
        .rev()
        .zip(after[prefix..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(before.len() - suffix)
        || !after.is_char_boundary(after.len() - suffix)
    {
        suffix -= 1;
    }
    Some(TextPatch {
        span: SourceSpan::new(prefix, before.len() - suffix),
        replacement: after[prefix..after.len() - suffix].to_owned(),
    })
}
