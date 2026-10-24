use super::text_segment::ISegmentTree;
use super::{
    DocumentChangeEventArgs, DocumentSubscription, ITextSegment, OffsetChangeMapEntry,
    SegmentError, TextDocument, TextSegment,
};
use std::cell::RefCell;
use std::rc::Rc;

type Link = Option<usize>;
struct Node<T> {
    item: Rc<T>,
    segment: TextSegment,
    offset: i128,
    length: usize,
    max_end: i128,
    lazy: i128,
    priority: u64,
    parent: Link,
    left: Link,
    right: Link,
}
struct IntervalTree<T> {
    nodes: Vec<Option<Node<T>>>,
    free: Vec<usize>,
    root: Link,
    random: u64,
}
impl<T: ITextSegment> Default for IntervalTree<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: None,
            random: 0x9e3779b97f4a7c15,
        }
    }
}
impl<T: ITextSegment> IntervalTree<T> {
    fn node(&self, index: usize) -> &Node<T> {
        self.nodes[index].as_ref().expect("live segment index")
    }
    fn node_mut(&mut self, index: usize) -> &mut Node<T> {
        self.nodes[index].as_mut().expect("live segment index")
    }
    fn len(&self) -> usize {
        self.nodes.len() - self.free.len()
    }
    fn parent(&mut self, child: Link, parent: Link) {
        if let Some(index) = child {
            self.node_mut(index).parent = parent;
        }
    }
    fn shift(&mut self, root: Link, delta: i128) {
        if let Some(index) = root {
            let node = self.node_mut(index);
            node.offset += delta;
            node.max_end += delta;
            node.lazy += delta;
        }
    }
    fn push(&mut self, index: usize) {
        let node = self.node_mut(index);
        let (delta, left, right) = (std::mem::take(&mut node.lazy), node.left, node.right);
        if delta != 0 {
            self.shift(left, delta);
            self.shift(right, delta);
        }
    }
    fn fix(&mut self, index: usize) {
        let node = self.node(index);
        let mut end = node.offset + node.length as i128;
        for child in [node.left, node.right].into_iter().flatten() {
            end = end.max(self.node(child).max_end + node.lazy);
        }
        self.node_mut(index).max_end = end;
    }
    fn fix_ancestors(&mut self, mut node: Link) {
        while let Some(index) = node {
            self.fix(index);
            node = self.node(index).parent;
        }
    }
    fn push_path(&mut self, index: usize) {
        let mut path = vec![index];
        let mut current = self.node(index).parent;
        while let Some(parent) = current {
            path.push(parent);
            current = self.node(parent).parent;
        }
        for index in path.into_iter().rev() {
            self.push(index);
        }
    }
    fn merge(&mut self, a: Link, b: Link) -> Link {
        let (Some(left), Some(right)) = (a, b) else {
            let result = a.or(b);
            self.parent(result, None);
            return result;
        };
        if self.node(left).priority <= self.node(right).priority {
            self.push(left);
            let tail = self.node(left).right;
            let child = self.merge(tail, Some(right));
            self.node_mut(left).right = child;
            self.node_mut(left).parent = None;
            self.parent(child, Some(left));
            self.fix(left);
            Some(left)
        } else {
            self.push(right);
            let head = self.node(right).left;
            let child = self.merge(Some(left), head);
            self.node_mut(right).left = child;
            self.node_mut(right).parent = None;
            self.parent(child, Some(right));
            self.fix(right);
            Some(right)
        }
    }
    fn split(&mut self, root: Link, offset: i128, inclusive: bool) -> (Link, Link) {
        let Some(index) = root else {
            return (None, None);
        };
        self.push(index);
        let node = self.node(index);
        if node.offset < offset || (inclusive && node.offset == offset) {
            let (a, b) = self.split(node.right, offset, inclusive);
            self.node_mut(index).right = a;
            self.node_mut(index).parent = None;
            self.parent(a, Some(index));
            self.parent(b, None);
            self.fix(index);
            (Some(index), b)
        } else {
            let (a, b) = self.split(node.left, offset, inclusive);
            self.node_mut(index).left = b;
            self.node_mut(index).parent = None;
            self.parent(b, Some(index));
            self.parent(a, None);
            self.fix(index);
            (a, Some(index))
        }
    }
    fn attach(&mut self, index: usize) {
        let (before, after) = self.split(self.root, self.node(index).offset, true);
        let before = self.merge(before, Some(index));
        self.root = self.merge(before, after);
    }
    fn insert(&mut self, item: Rc<T>, segment: TextSegment) -> usize {
        let offset = segment.start_offset() as i128;
        let length = segment.length();
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        let node = Node {
            item,
            segment,
            offset,
            length,
            max_end: offset + length as i128,
            lazy: 0,
            priority: self.random,
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
        self.attach(index);
        index
    }
    fn offset(&self, index: usize) -> usize {
        let mut offset = self.node(index).offset;
        let mut parent = self.node(index).parent;
        while let Some(index) = parent {
            let node = self.node(index);
            offset += node.lazy;
            parent = node.parent;
        }
        offset as usize
    }
    fn detach_node(&mut self, index: usize) {
        self.push_path(index);
        let node = self.node(index);
        let (parent, left, right) = (node.parent, node.left, node.right);
        let replacement = self.merge(left, right);
        self.parent(replacement, parent);
        if let Some(parent) = parent {
            let node = self.node_mut(parent);
            if node.left == Some(index) {
                node.left = replacement;
            } else {
                node.right = replacement;
            }
            self.fix_ancestors(Some(parent));
        } else {
            self.root = replacement;
        }
        let node = self.node_mut(index);
        node.parent = None;
        node.left = None;
        node.right = None;
        node.lazy = 0;
        node.max_end = node.offset + node.length as i128;
    }
    fn remove(&mut self, index: usize) -> Rc<T> {
        self.detach_node(index);
        let node = self.nodes[index].take().expect("live segment index");
        node.segment.detach(node.offset as usize);
        self.free.push(index);
        node.item
    }
    fn move_start(&mut self, index: usize, offset: usize) {
        self.detach_node(index);
        let node = self.node_mut(index);
        node.offset = offset as i128;
        node.max_end = node.offset + node.length as i128;
        self.attach(index);
    }
    fn change_length(&mut self, index: usize, length: usize) {
        self.node_mut(index).length = length;
        self.node(index).segment.0.length.set(length);
        self.fix_ancestors(Some(index));
    }
    fn query(&self, low: usize, high: usize) -> Vec<usize> {
        fn visit<T: ITextSegment>(
            tree: &IntervalTree<T>,
            root: Link,
            inherited: i128,
            low: i128,
            high: i128,
            result: &mut Vec<usize>,
        ) {
            let Some(index) = root else {
                return;
            };
            let node = tree.node(index);
            if node.max_end + inherited < low {
                return;
            }
            visit(tree, node.left, inherited + node.lazy, low, high, result);
            let offset = node.offset + inherited;
            if offset > high {
                return;
            }
            if offset + node.length as i128 >= low {
                result.push(index);
            }
            visit(tree, node.right, inherited + node.lazy, low, high, result);
        }
        let mut result = Vec::new();
        visit(self, self.root, 0, low as i128, high as i128, &mut result);
        result
    }
    fn first_after(&self, offset: usize) -> Link {
        let mut current = self.root;
        let mut inherited = 0;
        let mut found = None;
        while let Some(index) = current {
            let node = self.node(index);
            let start = node.offset + inherited;
            inherited += node.lazy;
            if start >= offset as i128 {
                found = Some(index);
                current = node.left;
            } else {
                current = node.right;
            }
        }
        found
    }
    fn extreme(&self, mut root: Link, last: bool) -> Link {
        let mut found = None;
        while let Some(index) = root {
            found = Some(index);
            root = if last {
                self.node(index).right
            } else {
                self.node(index).left
            };
        }
        found
    }
    fn neighbor(&self, index: usize, next: bool) -> Link {
        let child = if next {
            self.node(index).right
        } else {
            self.node(index).left
        };
        if child.is_some() {
            return self.extreme(child, !next);
        }
        let mut current = index;
        while let Some(parent) = self.node(current).parent {
            let side = if next {
                self.node(parent).left
            } else {
                self.node(parent).right
            };
            if side == Some(current) {
                return Some(parent);
            }
            current = parent;
        }
        None
    }
    fn preflight(&self, entry: &OffsetChangeMapEntry) -> Result<(), SegmentError> {
        entry
            .offset
            .checked_add(entry.removal_length)
            .ok_or(SegmentError::RangeOverflow)?;
        entry
            .offset
            .checked_add(entry.insertion_length)
            .ok_or(SegmentError::RangeOverflow)?;
        if entry.insertion_length > entry.removal_length {
            if let Some(root) = self.root {
                let max = self.node(root).max_end;
                if max >= entry.offset as i128
                    && max + entry.insertion_length as i128 - entry.removal_length as i128
                        > usize::MAX as i128
                {
                    return Err(SegmentError::RangeOverflow);
                }
            }
        }
        Ok(())
    }
    fn update(&mut self, entry: &OffsetChangeMapEntry) {
        if entry.removal_length == 0 && entry.insertion_length == 0 {
            return;
        }
        let start = entry.offset;
        let end = start + entry.removal_length;
        let affected = self.query(start, end);
        for index in affected {
            let offset = self.offset(index);
            let segment_end = offset + self.node(index).length;
            if entry.removal_length == 0 {
                if offset < start && start < segment_end {
                    self.change_length(index, self.node(index).length + entry.insertion_length);
                }
            } else if offset <= start {
                let length = if segment_end >= end {
                    self.node(index).length - entry.removal_length + entry.insertion_length
                } else {
                    start - offset
                };
                self.change_length(index, length);
            } else {
                let length = segment_end.saturating_sub(end);
                self.move_start(index, end);
                self.change_length(index, length);
            }
        }
        let (before, after) = self.split(self.root, start as i128, entry.removal_length != 0);
        self.shift(
            after,
            entry.insertion_length as i128 - entry.removal_length as i128,
        );
        self.root = self.merge(before, after);
    }
    fn clear(&mut self) {
        for index in self.query(0, usize::MAX) {
            self.node(index).segment.detach(self.offset(index));
        }
        self.nodes.clear();
        self.free.clear();
        self.root = None;
    }
    #[cfg(test)]
    fn validate(&self) {
        fn visit<T: ITextSegment>(
            tree: &IntervalTree<T>,
            root: Link,
            parent: Link,
            inherited: i128,
            output: &mut Vec<usize>,
        ) -> i128 {
            let Some(index) = root else {
                return -1;
            };
            let node = tree.node(index);
            assert_eq!(node.parent, parent);
            for child in [node.left, node.right].into_iter().flatten() {
                assert!(node.priority <= tree.node(child).priority);
            }
            let a = visit(tree, node.left, Some(index), inherited + node.lazy, output);
            output.push(tree.offset(index));
            let b = visit(tree, node.right, Some(index), inherited + node.lazy, output);
            let end = (node.offset + inherited + node.length as i128)
                .max(a)
                .max(b);
            assert_eq!(node.max_end + inherited, end);
            assert_eq!(node.segment.length(), node.length);
            end
        }
        let mut offsets = Vec::new();
        visit(self, self.root, None, 0, &mut offsets);
        assert!(offsets.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(offsets.len(), self.len());
    }
}

struct CollectionInner<T: ITextSegment> {
    tree: RefCell<IntervalTree<T>>,
    document: RefCell<Option<TextDocument>>,
    subscription: RefCell<Option<DocumentSubscription>>,
}
impl<T: ITextSegment> CollectionInner<T> {
    fn check_range(&self, offset: usize, length: usize) -> Result<(), SegmentError> {
        let end = offset
            .checked_add(length)
            .ok_or(SegmentError::RangeOverflow)?;
        if let Some(document) = &*self.document.borrow() {
            if end > document.text_length() {
                return Err(SegmentError::OutsideDocument {
                    end_offset: end,
                    text_length: document.text_length(),
                });
            }
        }
        Ok(())
    }
    fn on_changed(&self, change: &DocumentChangeEventArgs) {
        // Connected segments are validated against the document's extent, so
        // every committed document map keeps their offsets representable.
        let mut tree = self.tree.borrow_mut();
        for entry in change.offset_change_map.entries() {
            tree.update(entry);
        }
    }
}
impl<T: ITextSegment> ISegmentTree for CollectionInner<T> {
    fn offset(&self, node: usize) -> usize {
        self.tree.borrow().offset(node)
    }
    fn set_start(&self, node: usize, offset: usize) -> Result<(), SegmentError> {
        self.check_range(offset, self.tree.borrow().node(node).length)?;
        self.tree.borrow_mut().move_start(node, offset);
        Ok(())
    }
    fn set_length(&self, node: usize, length: usize) -> Result<(), SegmentError> {
        self.check_range(self.tree.borrow().offset(node), length)?;
        self.tree.borrow_mut().change_length(node, length);
        Ok(())
    }
}
impl<T: ITextSegment> Drop for CollectionInner<T> {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
        self.tree.get_mut().clear();
    }
}

/// A shared interval index. Queries include touching endpoints and return owned
/// snapshots, so callers may edit the collection while iterating query results.
/// The treap visits affected intervals and lazily shifts an untouched suffix.
pub struct TextSegmentCollection<T: ITextSegment = TextSegment>(Rc<CollectionInner<T>>);
impl<T: ITextSegment> Clone for TextSegmentCollection<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: ITextSegment> PartialEq for TextSegmentCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: ITextSegment> Eq for TextSegmentCollection<T> {}
impl<T: ITextSegment> Default for TextSegmentCollection<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: ITextSegment> std::fmt::Debug for TextSegmentCollection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSegmentCollection")
            .field("count", &self.count())
            .field("connected", &self.is_connected_to_document())
            .finish()
    }
}
impl<T: ITextSegment> TextSegmentCollection<T> {
    pub fn new() -> Self {
        Self(Rc::new(CollectionInner {
            tree: RefCell::new(IntervalTree::default()),
            document: RefCell::new(None),
            subscription: RefCell::new(None),
        }))
    }
    pub fn with_document(document: &TextDocument) -> Self {
        let collection = Self::new();
        let weak = Rc::downgrade(&collection.0);
        let subscription = document.changed(move |change| {
            if let Some(inner) = weak.upgrade() {
                inner.on_changed(change);
            }
        });
        *collection.0.document.borrow_mut() = Some(document.clone());
        *collection.0.subscription.borrow_mut() = Some(subscription);
        collection
    }
    pub fn is_connected_to_document(&self) -> bool {
        self.0.document.borrow().is_some()
    }
    pub fn disconnect(&self) {
        if let Some(subscription) = self.0.subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.0.document.borrow_mut().take();
    }
    pub fn count(&self) -> usize {
        self.0.tree.borrow().len()
    }
    pub fn len(&self) -> usize {
        self.count()
    }
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
    pub fn add(&self, item: T) -> Result<(), SegmentError> {
        let segment = item.text_segment().clone();
        if segment.is_connected_to_collection() {
            return Err(SegmentError::AlreadyConnected);
        }
        self.0
            .check_range(segment.start_offset(), segment.length())?;
        let index = self
            .0
            .tree
            .borrow_mut()
            .insert(Rc::new(item), segment.clone());
        let owner: Rc<dyn ISegmentTree> = self.0.clone();
        *segment.0.owner.borrow_mut() = Some(Rc::downgrade(&owner));
        segment.0.node.set(Some(index));
        Ok(())
    }
    fn index(&self, item: &T) -> Option<usize> {
        let segment = item.text_segment();
        let index = segment.0.node.get()?;
        let tree = self.0.tree.borrow();
        let candidate = tree.nodes.get(index)?.as_ref()?;
        (&candidate.segment == segment).then_some(index)
    }
    pub fn contains(&self, item: &T) -> bool {
        self.index(item).is_some()
    }
    pub fn remove(&self, item: &T) -> bool {
        let Some(index) = self.index(item) else {
            return false;
        };
        let removed = self.0.tree.borrow_mut().remove(index);
        drop(removed);
        true
    }
    pub fn clear(&self) {
        let mut tree = std::mem::take(&mut *self.0.tree.borrow_mut());
        tree.clear();
    }
    pub fn first_segment(&self) -> Option<T> {
        let item = {
            let tree = self.0.tree.borrow();
            tree.extreme(tree.root, false)
                .map(|index| tree.node(index).item.clone())
        };
        item.map(|item| (*item).clone())
    }
    pub fn last_segment(&self) -> Option<T> {
        let item = {
            let tree = self.0.tree.borrow();
            tree.extreme(tree.root, true)
                .map(|index| tree.node(index).item.clone())
        };
        item.map(|item| (*item).clone())
    }
    pub fn get_next_segment(&self, item: &T) -> Result<Option<T>, SegmentError> {
        self.neighbor(item, true)
    }
    pub fn get_previous_segment(&self, item: &T) -> Result<Option<T>, SegmentError> {
        self.neighbor(item, false)
    }
    fn neighbor(&self, item: &T, next: bool) -> Result<Option<T>, SegmentError> {
        let index = self.index(item).ok_or(SegmentError::NotInCollection)?;
        let item = {
            let tree = self.0.tree.borrow();
            tree.neighbor(index, next)
                .map(|index| tree.node(index).item.clone())
        };
        Ok(item.map(|item| (*item).clone()))
    }
    pub fn find_first_segment_with_start_after(&self, offset: usize) -> Option<T> {
        let item = {
            let tree = self.0.tree.borrow();
            tree.first_after(offset)
                .map(|index| tree.node(index).item.clone())
        };
        item.map(|item| (*item).clone())
    }
    pub fn find_segments_containing(&self, offset: usize) -> Vec<T> {
        self.query(offset, offset)
    }
    pub fn find_overlapping_segments(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<Vec<T>, SegmentError> {
        let end = offset
            .checked_add(length)
            .ok_or(SegmentError::RangeOverflow)?;
        Ok(self.query(offset, end))
    }
    pub fn segments(&self) -> Vec<T> {
        self.query(0, usize::MAX)
    }
    fn query(&self, low: usize, high: usize) -> Vec<T> {
        let items = {
            let tree = self.0.tree.borrow();
            tree.query(low, high)
                .into_iter()
                .map(|index| tree.node(index).item.clone())
                .collect::<Vec<_>>()
        };
        items.into_iter().map(|item| (*item).clone()).collect()
    }
    pub fn update_offsets(&self, change: &DocumentChangeEventArgs) -> Result<(), SegmentError> {
        if self.is_connected_to_document() {
            return Err(SegmentError::AutomaticUpdates);
        }
        // Validate all maps before mutation. A conservative accumulated positive
        // shift bounds the maximum representable endpoint throughout the batch.
        let mut max = self
            .0
            .tree
            .borrow()
            .root
            .map(|root| self.0.tree.borrow().node(root).max_end)
            .unwrap_or(0);
        for entry in change.offset_change_map.entries() {
            entry
                .offset
                .checked_add(entry.removal_length)
                .ok_or(SegmentError::RangeOverflow)?;
            entry
                .offset
                .checked_add(entry.insertion_length)
                .ok_or(SegmentError::RangeOverflow)?;
            max += (entry.insertion_length as i128 - entry.removal_length as i128).max(0);
            if max > usize::MAX as i128 {
                return Err(SegmentError::RangeOverflow);
            }
        }
        self.0.on_changed(change);
        Ok(())
    }
    pub fn update_offsets_entry(&self, entry: OffsetChangeMapEntry) -> Result<(), SegmentError> {
        if self.is_connected_to_document() {
            return Err(SegmentError::AutomaticUpdates);
        }
        let mut tree = self.0.tree.borrow_mut();
        tree.preflight(&entry)?;
        tree.update(&entry);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
