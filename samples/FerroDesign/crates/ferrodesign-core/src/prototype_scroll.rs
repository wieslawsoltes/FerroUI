//! Scroll offsets are preview state. Moving direct children keeps native
//! rendering, clipping, and hit testing in the same coordinate system.

use crate::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PrototypeScrollRange {
    pub minimum: Point,
    pub maximum: Point,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrototypeScrollState {
    offsets: BTreeMap<NodeId, Point>,
}

impl PrototypeScrollState {
    pub fn offset(&self, container: NodeId) -> Point {
        self.offsets.get(&container).copied().unwrap_or_default()
    }

    pub fn offsets(&self) -> &BTreeMap<NodeId, Point> {
        &self.offsets
    }

    pub fn clear(&mut self) {
        self.offsets.clear();
    }

    /// The visible content extent is computed in container coordinates. Nested
    /// clipped frames contribute their viewport, not inaccessible overflow.
    pub fn range(&self, document: &Document, container: NodeId) -> Option<PrototypeScrollRange> {
        let frame = document.nodes.get(&container)?;
        let (horizontal, vertical) = axes(frame.prototype_overflow);
        if !horizontal && !vertical { return None; }
        let offset = self.offset(container);
        let mut bounds = Rect::new(0.0, 0.0, frame.width, frame.height);
        let mut stack: Vec<_> = frame.children.iter().rev().filter_map(|id| {
            document.nodes.get(id).map(|node| (*id, node.local_transform(), 1usize))
        }).collect();
        while let Some((id, transform, depth)) = stack.pop() {
            let Some(node) = document.nodes.get(&id) else { continue };
            if !node.visible || depth >= MAX_DOCUMENT_DEPTH { continue; }
            let current = transform.transform_rect(node.local_bounds());
            if current.is_finite() {
                bounds = bounds.union(Rect::new(current.x + offset.x, current.y + offset.y, current.width, current.height));
            }
            if !node.clip_content && node.prototype_overflow == PrototypeOverflow::None {
                for &child in node.children.iter().rev() {
                    if let Some(child_node) = document.nodes.get(&child) {
                        stack.push((child, transform * child_node.local_transform(), depth + 1));
                    }
                }
            }
        }
        Some(PrototypeScrollRange {
            minimum: Point::new(if horizontal { bounds.x.min(0.0) } else { 0.0 }, if vertical { bounds.y.min(0.0) } else { 0.0 }),
            maximum: Point::new(if horizontal { (bounds.right() - frame.width).max(0.0) } else { 0.0 }, if vertical { (bounds.bottom() - frame.height).max(0.0) } else { 0.0 }),
        })
    }

    /// Consumes wheel/drag movement in the nearest eligible frame, then passes
    /// unconsumed axes and boundary movement to its enclosing scroll frames.
    pub fn scroll_by(&mut self, document: &mut Document, root: NodeId, node: NodeId, delta: Point) -> bool {
        if !delta.x.is_finite() || !delta.y.is_finite() || !within(document, node, root) {
            return false;
        }
        let mut current = Some(node);
        let mut remaining = delta;
        let mut changed = false;
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(id) = current else { break };
            let parent = document.nodes.get(&id).and_then(|node| node.parent);
            let requested = self.offset(id) + remaining;
            let consumed = self.set_offset(document, id, requested);
            remaining = remaining - consumed;
            changed |= consumed != Point::default();
            if id == root || (remaining.x.abs() < GEOMETRY_EPSILON && remaining.y.abs() < GEOMETRY_EPSILON) { break; }
            current = parent;
        }
        changed
    }

    /// Aligns a target's leading edge in every scrollable ancestor, clamping
    /// near the content end. Inner frames are resolved before their parents.
    pub fn scroll_to(&mut self, document: &mut Document, root: NodeId, target: NodeId) -> bool {
        if !within(document, target, root) { return false; }
        let mut current = document.nodes.get(&target).and_then(|node| node.parent);
        let mut changed = false;
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(id) = current else { break };
            let parent = document.nodes.get(&id).and_then(|node| node.parent);
            if self.range(document, id).is_some() {
                if let (Some(inverse), Some(world), Some(node)) = (
                    document.world_transform(id).and_then(Transform::inverse),
                    document.world_transform(target),
                    document.nodes.get(&target),
                ) {
                    let bounds = (inverse * world).transform_rect(node.local_bounds());
                    let requested = self.offset(id) + Point::new(bounds.x, bounds.y);
                    changed |= self.set_offset(document, id, requested) != Point::default();
                }
            }
            if id == root { break; }
            current = parent;
        }
        changed
    }

    /// A fresh navigation starts the destination at its authored scroll
    /// position. Back navigation can retain the previous frame's offsets.
    pub fn reset_subtree(&mut self, document: &mut Document, root: NodeId) {
        let containers: Vec<_> = self.offsets.keys().copied().filter(|id| within(document, *id, root)).collect();
        for id in containers {
            let offset = self.offsets.remove(&id).unwrap_or_default();
            translate_children(document, id, offset);
        }
    }

    fn set_offset(&mut self, document: &mut Document, container: NodeId, requested: Point) -> Point {
        let Some(range) = self.range(document, container) else { return Point::default() };
        let current = self.offset(container);
        let target = Point::new(
            requested.x.clamp(range.minimum.x, range.maximum.x),
            requested.y.clamp(range.minimum.y, range.maximum.y),
        );
        let consumed = target - current;
        if consumed.x.abs() < GEOMETRY_EPSILON && consumed.y.abs() < GEOMETRY_EPSILON {
            return Point::default();
        }
        self.offsets.insert(container, target);
        if let Some(frame) = document.nodes.get_mut(&container) { frame.clip_content = true; }
        translate_children(document, container, consumed * -1.0);
        consumed
    }
}

fn axes(overflow: PrototypeOverflow) -> (bool, bool) {
    match overflow {
        PrototypeOverflow::None => (false, false),
        PrototypeOverflow::Horizontal => (true, false),
        PrototypeOverflow::Vertical => (false, true),
        PrototypeOverflow::Both => (true, true),
    }
}

fn translate_children(document: &mut Document, parent: NodeId, offset: Point) {
    let children = document.nodes.get(&parent).map(|node| node.children.clone()).unwrap_or_default();
    for child in children {
        if let Some(node) = document.nodes.get_mut(&child) {
            node.x += offset.x;
            node.y += offset.y;
        }
    }
}

fn within(document: &Document, mut node: NodeId, root: NodeId) -> bool {
    for _ in 0..MAX_DOCUMENT_DEPTH {
        if node == root { return true; }
        let Some(parent) = document.nodes.get(&node).and_then(|node| node.parent) else { return false };
        node = parent;
    }
    false
}
