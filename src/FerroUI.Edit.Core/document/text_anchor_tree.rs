use super::text_anchor::AnchorInner;
use super::{AnchorMovementType, OffsetChangeMapEntry};
use std::rc::{Rc, Weak};

type Link = Option<usize>;

struct Node {
    anchor: Weak<AnchorInner>,
    offset: i128,
    lazy: i128,
    priority: u64,
    parent: Link,
    left: Link,
    right: Link,
}

/// An arena-backed ordered treap. Subtree offsets are shifted lazily, so an edit
/// visits only the logarithmic boundary paths and anchors inside the edit.
/// Anchor handles keep stable arena indexes and read their current offset by
/// accumulating pending offsets on the parent path.
pub(crate) struct TextAnchorTree {
    nodes: Vec<Option<Node>>,
    free: Vec<usize>,
    root: Link,
    random: u64,
}

impl Default for TextAnchorTree {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: None,
            random: 0xd1b54a32d192ed03,
        }
    }
}

impl TextAnchorTree {
    fn node(&self, index: usize) -> &Node {
        self.nodes[index].as_ref().expect("live anchor tree index")
    }
    fn node_mut(&mut self, index: usize) -> &mut Node {
        self.nodes[index].as_mut().expect("live anchor tree index")
    }

    fn priority(&mut self) -> u64 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        self.random
    }

    fn set_parent(&mut self, link: Link, parent: Link) {
        if let Some(index) = link {
            self.node_mut(index).parent = parent;
        }
    }

    fn shift(&mut self, link: Link, delta: i128) {
        if let Some(index) = link {
            let node = self.node_mut(index);
            node.offset += delta;
            node.lazy += delta;
        }
    }

    fn push(&mut self, index: usize) {
        let (delta, left, right) = {
            let node = self.node_mut(index);
            let delta = std::mem::take(&mut node.lazy);
            (delta, node.left, node.right)
        };
        if delta != 0 {
            self.shift(left, delta);
            self.shift(right, delta);
        }
    }

    fn merge(&mut self, left: Link, right: Link) -> Link {
        let (Some(a), Some(b)) = (left, right) else {
            let result = left.or(right);
            self.set_parent(result, None);
            return result;
        };
        if self.node(a).priority <= self.node(b).priority {
            self.push(a);
            let tail = self.node(a).right;
            let child = self.merge(tail, Some(b));
            self.node_mut(a).right = child;
            self.node_mut(a).parent = None;
            self.set_parent(child, Some(a));
            Some(a)
        } else {
            self.push(b);
            let head = self.node(b).left;
            let child = self.merge(Some(a), head);
            self.node_mut(b).left = child;
            self.node_mut(b).parent = None;
            self.set_parent(child, Some(b));
            Some(b)
        }
    }

    /// Left contains offsets below `offset`, or also equal when inclusive.
    fn split(&mut self, root: Link, offset: i128, inclusive: bool) -> (Link, Link) {
        let Some(index) = root else {
            return (None, None);
        };
        self.push(index);
        let value = self.node(index).offset;
        if value < offset || (inclusive && value == offset) {
            let child = self.node(index).right;
            let (left, right) = self.split(child, offset, inclusive);
            self.node_mut(index).right = left;
            self.node_mut(index).parent = None;
            self.set_parent(left, Some(index));
            self.set_parent(right, None);
            (Some(index), right)
        } else {
            let child = self.node(index).left;
            let (left, right) = self.split(child, offset, inclusive);
            self.node_mut(index).left = right;
            self.node_mut(index).parent = None;
            self.set_parent(right, Some(index));
            self.set_parent(left, None);
            (left, Some(index))
        }
    }

    pub fn insert(&mut self, offset: usize, anchor: Weak<AnchorInner>) -> usize {
        let priority = self.priority();
        let node = Node {
            anchor,
            offset: offset as i128,
            lazy: 0,
            priority,
            parent: None,
            left: None,
            right: None,
        };
        let index = if let Some(index) = self.free.pop() {
            self.nodes[index] = Some(node);
            index
        } else {
            self.nodes.push(Some(node));
            self.nodes.len() - 1
        };
        let (left, right) = self.split(self.root, offset as i128, true);
        let left = self.merge(left, Some(index));
        self.root = self.merge(left, right);
        index
    }

    pub fn offset(&self, index: usize) -> Option<usize> {
        let node = self.nodes.get(index)?.as_ref()?;
        let mut value = node.offset;
        let mut parent = node.parent;
        while let Some(index) = parent {
            let node = self.node(index);
            value += node.lazy;
            parent = node.parent;
        }
        usize::try_from(value).ok()
    }

    pub fn remove(&mut self, index: usize) {
        if self.nodes.get(index).is_none_or(Option::is_none) {
            return;
        }
        let mut path = vec![index];
        let mut current = self.node(index).parent;
        while let Some(parent) = current {
            path.push(parent);
            current = self.node(parent).parent;
        }
        for &ancestor in path.iter().rev() {
            self.push(ancestor);
        }
        let (parent, left, right) = {
            let node = self.node(index);
            (node.parent, node.left, node.right)
        };
        let replacement = self.merge(left, right);
        self.set_parent(replacement, parent);
        if let Some(parent) = parent {
            let parent = self.node_mut(parent);
            if parent.left == Some(index) {
                parent.left = replacement;
            } else {
                parent.right = replacement;
            }
        } else {
            self.root = replacement;
        }
        self.nodes[index] = None;
        self.free.push(index);
    }

    fn drain(&mut self, root: Link, result: &mut Vec<usize>) {
        let Some(index) = root else {
            return;
        };
        self.push(index);
        let (left, right) = {
            let node = self.node(index);
            (node.left, node.right)
        };
        self.drain(left, result);
        result.push(index);
        self.drain(right, result);
        let node = self.node_mut(index);
        node.left = None;
        node.right = None;
        node.parent = None;
    }

    pub fn handle_text_change(
        &mut self,
        entry: &OffsetChangeMapEntry,
        deleted: &mut Vec<Rc<AnchorInner>>,
    ) {
        if entry.removal_length == 0 && entry.insertion_length == 0 {
            return;
        }
        let start = entry.offset as i128;
        let end = start + entry.removal_length as i128;
        // For replacements the start boundary stays before and the end boundary
        // stays after. Pure insertions resolve equal positions by affinity.
        let (before, rest) = self.split(self.root, start, entry.removal_length != 0);
        let (middle, after) = self.split(rest, end, entry.removal_length == 0);
        self.shift(
            after,
            entry.insertion_length as i128 - entry.removal_length as i128,
        );
        let mut affected = Vec::new();
        self.drain(middle, &mut affected);
        let mut stay_before = None;
        let mut stay_after = None;
        for index in affected {
            let Some(anchor) = self.node(index).anchor.upgrade() else {
                self.nodes[index] = None;
                self.free.push(index);
                continue;
            };
            if entry.removal_length != 0
                && !entry.removal_never_causes_anchor_deletion
                && !anchor.survive.get()
            {
                anchor.node.set(None);
                anchor.deleted.set(true);
                self.nodes[index] = None;
                self.free.push(index);
                deleted.push(anchor);
                continue;
            }
            let before = match anchor.movement.get() {
                AnchorMovementType::BeforeInsertion => true,
                AnchorMovementType::AfterInsertion => false,
                AnchorMovementType::Default => entry.default_anchor_movement_is_before_insertion,
            };
            self.node_mut(index).offset = start
                + if before {
                    0
                } else {
                    entry.insertion_length as i128
                };
            if before {
                stay_before = self.merge(stay_before, Some(index));
            } else {
                stay_after = self.merge(stay_after, Some(index));
            }
        }
        let before = self.merge(before, stay_before);
        let after = self.merge(stay_after, after);
        self.root = self.merge(before, after);
    }

    #[cfg(test)]
    pub fn validate(&self) {
        fn visit(
            tree: &TextAnchorTree,
            root: Link,
            parent: Link,
            lazy: i128,
        ) -> (Option<i128>, Option<i128>) {
            let Some(index) = root else {
                return (None, None);
            };
            let node = tree.node(index);
            assert_eq!(node.parent, parent);
            if let Some(left) = node.left {
                assert!(node.priority <= tree.node(left).priority);
            }
            if let Some(right) = node.right {
                assert!(node.priority <= tree.node(right).priority);
            }
            let (first, left_end) = visit(tree, node.left, Some(index), lazy + node.lazy);
            let (right_start, last) = visit(tree, node.right, Some(index), lazy + node.lazy);
            let offset = node.offset + lazy;
            assert!(left_end.is_none_or(|v| v <= offset));
            assert!(right_start.is_none_or(|v| v >= offset));
            assert_eq!(tree.offset(index), Some(offset as usize));
            (first.or(Some(offset)), last.or(Some(offset)))
        }
        visit(self, self.root, None, 0);
    }
}
