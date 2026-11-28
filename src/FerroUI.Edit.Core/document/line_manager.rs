use super::live_document_line::LiveLineInner;
use super::{
    DocumentChangeEventArgs, DocumentError, DocumentLine, DocumentSubscription, ILineTracker,
    LiveDocumentLine, TextSnapshot,
};
use crate::utils::{Event, Utf16Rope};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Link = Option<usize>;
struct LineNode {
    line: Rc<LiveLineInner>,
    count: usize,
    length: usize,
    priority: u64,
    parent: Link,
    left: Link,
    right: Link,
}

/// An implicit-order treap; subtree line counts and text lengths make both
/// coordinate lookups and edits logarithmic without shifting suffix objects.
struct DocumentLineTree {
    nodes: Vec<Option<LineNode>>,
    free: Vec<usize>,
    root: Link,
    random: u64,
}
impl Default for DocumentLineTree {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            root: None,
            random: 0xa0761d6478bd642f,
        }
    }
}
impl DocumentLineTree {
    fn node(&self, index: usize) -> &LineNode {
        self.nodes[index].as_ref().expect("live line index")
    }
    fn node_mut(&mut self, index: usize) -> &mut LineNode {
        self.nodes[index].as_mut().expect("live line index")
    }
    fn count(&self, root: Link) -> usize {
        root.map_or(0, |index| self.node(index).count)
    }
    fn length(&self, root: Link) -> usize {
        root.map_or(0, |index| self.node(index).length)
    }
    fn parent(&mut self, child: Link, parent: Link) {
        if let Some(index) = child {
            self.node_mut(index).parent = parent;
        }
    }
    fn fix(&mut self, index: usize) {
        let node = self.node(index);
        let count = 1 + self.count(node.left) + self.count(node.right);
        let length =
            node.line.total_length.get() + self.length(node.left) + self.length(node.right);
        let node = self.node_mut(index);
        node.count = count;
        node.length = length;
    }
    fn fix_ancestors(&mut self, mut node: Link) {
        while let Some(index) = node {
            self.fix(index);
            node = self.node(index).parent;
        }
    }
    fn allocate(&mut self, line: Rc<LiveLineInner>) -> usize {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        let node = LineNode {
            length: line.total_length.get(),
            line,
            count: 1,
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
        self.node(index).line.node.set(Some(index));
        index
    }
    fn merge(&mut self, a: Link, b: Link) -> Link {
        let (Some(left), Some(right)) = (a, b) else {
            let result = a.or(b);
            self.parent(result, None);
            return result;
        };
        if self.node(left).priority <= self.node(right).priority {
            let child = self.merge(self.node(left).right, Some(right));
            self.node_mut(left).right = child;
            self.node_mut(left).parent = None;
            self.parent(child, Some(left));
            self.fix(left);
            Some(left)
        } else {
            let child = self.merge(Some(left), self.node(right).left);
            self.node_mut(right).left = child;
            self.node_mut(right).parent = None;
            self.parent(child, Some(right));
            self.fix(right);
            Some(right)
        }
    }
    fn split(&mut self, root: Link, count: usize) -> (Link, Link) {
        let Some(index) = root else {
            return (None, None);
        };
        let before = self.count(self.node(index).left);
        if count > before {
            let (a, b) = self.split(self.node(index).right, count - before - 1);
            self.node_mut(index).right = a;
            self.node_mut(index).parent = None;
            self.parent(a, Some(index));
            self.parent(b, None);
            self.fix(index);
            (Some(index), b)
        } else {
            let (a, b) = self.split(self.node(index).left, count);
            self.node_mut(index).left = b;
            self.node_mut(index).parent = None;
            self.parent(b, Some(index));
            self.parent(a, None);
            self.fix(index);
            (a, Some(index))
        }
    }
    fn coordinates(&self, index: usize) -> (usize, usize) {
        let node = self.node(index);
        let mut number = self.count(node.left);
        let mut offset = self.length(node.left);
        let mut child = index;
        while let Some(parent) = self.node(child).parent {
            let node = self.node(parent);
            if node.right == Some(child) {
                number += 1 + self.count(node.left);
                offset += node.line.total_length.get() + self.length(node.left);
            }
            child = parent;
        }
        (number, offset)
    }
    fn by_number(&self, mut number: usize) -> usize {
        let mut current = self.root.expect("document has a line");
        loop {
            let node = self.node(current);
            let before = self.count(node.left);
            if number < before {
                current = node.left.expect("line in left subtree");
            } else if number == before {
                return current;
            } else {
                number -= before + 1;
                current = node.right.expect("line in right subtree");
            }
        }
    }
    fn by_offset(&self, mut offset: usize) -> usize {
        let mut current = self.root.expect("document has a line");
        loop {
            let node = self.node(current);
            let before = self.length(node.left);
            if offset < before {
                current = node.left.expect("offset in left subtree");
            } else if offset < before + node.line.total_length.get() || node.right.is_none() {
                return current;
            } else {
                offset -= before + node.line.total_length.get();
                current = node.right.expect("offset in right subtree");
            }
        }
    }
    fn insert_after(&mut self, index: usize, line: Rc<LiveLineInner>) -> usize {
        let number = self.coordinates(index).0;
        let new = self.allocate(line);
        let (a, b) = self.split(self.root, number + 1);
        let a = self.merge(a, Some(new));
        self.root = self.merge(a, b);
        new
    }
    fn remove(&mut self, index: usize) {
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
        let node = self.nodes[index].take().expect("live line index");
        node.line.deleted.set(true);
        node.line.node.set(None);
        self.free.push(index);
    }
    fn rebuild(&mut self, lines: Vec<Rc<LiveLineInner>>) {
        for node in self.nodes.iter().flatten() {
            node.line.node.set(None);
            node.line.deleted.set(true);
        }
        *self = Self::default();
        let mut stack = Vec::new();
        for line in lines {
            line.deleted.set(false);
            let index = self.allocate(line);
            let mut left = None;
            while stack
                .last()
                .is_some_and(|&old| self.node(old).priority > self.node(index).priority)
            {
                left = stack.pop();
            }
            self.node_mut(index).left = left;
            self.parent(left, Some(index));
            if let Some(&parent) = stack.last() {
                self.node_mut(parent).right = Some(index);
                self.node_mut(index).parent = Some(parent);
            }
            stack.push(index);
        }
        self.root = stack.first().copied();
        fn finish(tree: &mut DocumentLineTree, root: Link) {
            if let Some(index) = root {
                let node = tree.node(index);
                let (left, right) = (node.left, node.right);
                finish(tree, left);
                finish(tree, right);
                tree.fix(index);
            }
        }
        finish(self, self.root);
    }
    #[cfg(test)]
    fn validate(&self, snapshot: &TextSnapshot) {
        fn visit(tree: &DocumentLineTree, root: Link, parent: Link) -> (usize, usize) {
            let Some(index) = root else { return (0, 0) };
            let node = tree.node(index);
            assert_eq!(node.parent, parent);
            assert_eq!(node.line.node.get(), Some(index));
            assert!(!node.line.deleted.get());
            for child in [node.left, node.right].into_iter().flatten() {
                assert!(node.priority <= tree.node(child).priority);
            }
            let (a, b) = visit(tree, node.left, Some(index));
            let (c, d) = visit(tree, node.right, Some(index));
            assert_eq!(node.count, a + c + 1);
            assert_eq!(node.length, b + d + node.line.total_length.get());
            (node.count, node.length)
        }
        let (count, length) = visit(self, self.root, None);
        assert_eq!(
            (count, length),
            (snapshot.line_count(), snapshot.text_length())
        );
        for number in 0..count {
            let index = self.by_number(number);
            let line = snapshot.get_line_by_number(number + 1).unwrap();
            assert_eq!(self.coordinates(index), (number, line.offset));
            assert_eq!(
                self.node(index).line.total_length.get(),
                line.total_length()
            );
            assert_eq!(
                self.node(index).line.delimiter_length.get(),
                line.delimiter_length
            );
            assert_eq!(self.by_offset(line.offset), index);
        }
    }
}

pub(crate) enum LineEvent {
    BeforeRemove(LiveDocumentLine),
    SetLength(LiveDocumentLine, usize),
    Inserted(LiveDocumentLine, LiveDocumentLine),
    Rebuild,
    Complete(DocumentChangeEventArgs),
}
impl LineEvent {
    pub(crate) fn dispatch(&self, tracker: &dyn ILineTracker) {
        match self {
            Self::BeforeRemove(line) => tracker.before_remove_line(line),
            Self::SetLength(line, length) => tracker.set_line_length(line, *length),
            Self::Inserted(previous, line) => tracker.line_inserted(previous, line),
            Self::Rebuild => tracker.rebuild_document(),
            Self::Complete(change) => tracker.change_complete(change),
        }
    }
}

fn new_line(total_length: usize, delimiter_length: usize) -> Rc<LiveLineInner> {
    Rc::new(LiveLineInner {
        node: Cell::new(None),
        deleted: Cell::new(false),
        total_length: Cell::new(total_length),
        delimiter_length: Cell::new(delimiter_length),
    })
}
fn newlines(text: &TextSnapshot) -> Vec<(usize, usize)> {
    let mut delimiters = Vec::<(usize, usize)>::new();
    let mut offset = 0;
    let mut cr = false;
    text.for_each_chunk(|chunk| {
        for &unit in chunk {
            if unit == 10 && cr {
                delimiters.last_mut().expect("preceding CR").1 = 2;
                cr = false;
            } else {
                if unit == 10 || unit == 13 {
                    delimiters.push((offset, 1));
                }
                cr = unit == 13;
            }
            offset += 1;
        }
    });
    delimiters
}

pub(crate) struct LineManager {
    tree: RefCell<DocumentLineTree>,
    events: Event<LineEvent>,
}
impl LineManager {
    pub fn new(snapshot: &TextSnapshot) -> Rc<Self> {
        let manager = Rc::new(Self {
            tree: RefCell::new(DocumentLineTree::default()),
            events: Event::default(),
        });
        manager.rebuild(snapshot, false);
        manager
    }
    pub fn subscribe(&self, handler: impl Fn(&LineEvent) + 'static) -> DocumentSubscription {
        self.events.subscribe(handler)
    }
    fn handle(self: &Rc<Self>, index: usize) -> LiveDocumentLine {
        LiveDocumentLine {
            inner: self.tree.borrow().node(index).line.clone(),
            manager: self.clone(),
        }
    }
    pub fn by_number(self: &Rc<Self>, number: usize) -> Result<LiveDocumentLine, DocumentError> {
        let index = {
            let tree = self.tree.borrow();
            let count = tree.count(tree.root);
            if number == 0 || number > count {
                return Err(DocumentError::InvalidLine {
                    line_number: number,
                    line_count: count,
                });
            }
            tree.by_number(number - 1)
        };
        Ok(self.handle(index))
    }
    pub fn by_offset(self: &Rc<Self>, offset: usize) -> Result<LiveDocumentLine, DocumentError> {
        let index = {
            let tree = self.tree.borrow();
            let length = tree.length(tree.root);
            if offset > length {
                return Err(DocumentError::InvalidOffset {
                    offset,
                    text_length: length,
                });
            }
            tree.by_offset(offset)
        };
        Ok(self.handle(index))
    }
    pub fn snapshot(&self, line: &LiveDocumentLine) -> Result<DocumentLine, DocumentError> {
        let index = line.inner.node.get().ok_or(DocumentError::DeletedLine)?;
        let (number, offset) = self.tree.borrow().coordinates(index);
        Ok(DocumentLine {
            line_number: number + 1,
            offset,
            length: line.length(),
            delimiter_length: line.delimiter_length(),
        })
    }
    pub fn neighbor(
        self: &Rc<Self>,
        line: &LiveDocumentLine,
        next: bool,
    ) -> Option<LiveDocumentLine> {
        let index = line.inner.node.get()?;
        let target = {
            let tree = self.tree.borrow();
            let number = tree.coordinates(index).0;
            let number = if next {
                number + 1
            } else {
                number.checked_sub(1)?
            };
            if number >= tree.count(tree.root) {
                return None;
            }
            tree.by_number(number)
        };
        Some(self.handle(target))
    }
    pub fn lines(self: &Rc<Self>) -> Vec<LiveDocumentLine> {
        let count = {
            let tree = self.tree.borrow();
            tree.count(tree.root)
        };
        (1..=count)
            .map(|number| self.by_number(number).expect("known line number"))
            .collect()
    }
    fn rebuild(self: &Rc<Self>, text: &TextSnapshot, notify: bool) {
        let first = {
            let tree = self.tree.borrow();
            tree.root.map(|_| tree.node(tree.by_number(0)).line.clone())
        };
        let mut lines = Vec::with_capacity(text.line_count());
        let mut previous = 0;
        for (offset, length) in newlines(text) {
            lines.push(new_line(offset + length - previous, length));
            previous = offset + length;
        }
        lines.push(new_line(text.text_length() - previous, 0));
        if let Some(first) = first {
            first.total_length.set(lines[0].total_length.get());
            first.delimiter_length.set(lines[0].delimiter_length.get());
            lines[0] = first;
        }
        self.tree.borrow_mut().rebuild(lines);
        if notify {
            self.events.raise(&LineEvent::Rebuild);
        }
    }
    fn remove_line(self: &Rc<Self>, line: &LiveDocumentLine) {
        self.events.raise(&LineEvent::BeforeRemove(line.clone()));
        self.tree
            .borrow_mut()
            .remove(line.inner.node.get().expect("live removed line"));
    }
    fn insert_line(
        self: &Rc<Self>,
        previous: &LiveDocumentLine,
        length: usize,
    ) -> LiveDocumentLine {
        let index = self.tree.borrow_mut().insert_after(
            previous.inner.node.get().expect("live insertion line"),
            new_line(length, 0),
        );
        let line = self.handle(index);
        self.events
            .raise(&LineEvent::Inserted(previous.clone(), line.clone()));
        line
    }
    fn set_length(
        self: &Rc<Self>,
        line: &LiveDocumentLine,
        length: usize,
        text: &TextSnapshot,
    ) -> LiveDocumentLine {
        if line.total_length() != length {
            self.events
                .raise(&LineEvent::SetLength(line.clone(), length));
            line.inner.total_length.set(length);
            self.tree.borrow_mut().fix_ancestors(line.inner.node.get());
        }
        let delimiter = if length == 0 {
            0
        } else {
            let offset = line.offset().expect("live changed line");
            let last = text
                .get_char_at(offset + length - 1)
                .expect("line belongs to change snapshot");
            if last == 13 {
                1
            } else if last == 10 {
                if length >= 2 && text.get_char_at(offset + length - 2) == Ok(13) {
                    2
                } else if length == 1 && offset > 0 && text.get_char_at(offset - 1) == Ok(13) {
                    let previous = line.previous_line().expect("CR preceding LF line");
                    self.remove_line(line);
                    return self.set_length(&previous, previous.total_length() + 1, text);
                } else {
                    1
                }
            } else {
                0
            }
        };
        line.inner.delimiter_length.set(delimiter);
        line.clone()
    }
    fn remove_text(self: &Rc<Self>, offset: usize, length: usize, text: &TextSnapshot) {
        if length == 0 {
            return;
        }
        let start = self.by_offset(offset).expect("validated removal offset");
        let start_offset = start.offset().expect("live removal line");
        if offset > start_offset + start.length() {
            self.set_length(&start, start.total_length() - 1, text);
            self.remove_text(offset, length - 1, text);
            return;
        }
        if offset + length < start_offset + start.total_length() {
            self.set_length(&start, start.total_length() - length, text);
            return;
        }
        let removed_in_start = start_offset + start.total_length() - offset;
        let end = self
            .by_offset(offset + length)
            .expect("validated removal end");
        if start == end {
            self.set_length(&start, start.total_length() - length, text);
            return;
        }
        let left_in_end =
            end.offset().expect("live end line") + end.total_length() - (offset + length);
        loop {
            let line = start.next_line().expect("line through removal end");
            let finished = line == end;
            self.remove_line(&line);
            if finished {
                break;
            }
        }
        self.set_length(
            &start,
            start.total_length() - removed_in_start + left_in_end,
            text,
        );
    }
    fn insert_text(self: &Rc<Self>, offset: usize, inserted: &TextSnapshot, text: &TextSnapshot) {
        let mut line = self.by_offset(offset).expect("validated insertion offset");
        if offset > line.offset().expect("live insertion line") + line.length() {
            self.set_length(&line, line.total_length() - 1, text);
            line = self.insert_line(&line, 1);
            line = self.set_length(&line, 1, text);
        }
        let delimiters = newlines(inserted);
        if delimiters.is_empty() {
            self.set_length(&line, line.total_length() + inserted.text_length(), text);
            return;
        }
        let mut last_end = 0;
        for (delimiter_offset, delimiter_length) in delimiters {
            let line_offset = line.offset().expect("live insertion line");
            let tail = line_offset + line.total_length() - (offset + last_end);
            line = self.set_length(
                &line,
                offset + delimiter_offset + delimiter_length - line_offset,
                text,
            );
            let next = self.insert_line(&line, tail);
            line = self.set_length(&next, tail, text);
            last_end = delimiter_offset + delimiter_length;
        }
        if last_end != inserted.text_length() {
            self.set_length(
                &line,
                line.total_length() + inserted.text_length() - last_end,
                text,
            );
        }
    }
    pub fn apply_change(
        self: &Rc<Self>,
        old: &TextSnapshot,
        new: &TextSnapshot,
        change: &DocumentChangeEventArgs,
    ) {
        if change.offset == 0 && change.removal_length() == old.text_length() {
            self.rebuild(new, true);
            return;
        }
        if change.removal_length() > 0 {
            let rope = old
                .rope
                .replace(change.offset, change.removal_length(), &Utf16Rope::new())
                .expect("validated removal");
            let intermediate = TextSnapshot {
                rope,
                version: None,
            };
            self.remove_text(change.offset, change.removal_length(), &intermediate);
        }
        self.insert_text(change.offset, &change.inserted_text, new);
    }
    pub fn change_complete(&self, change: &DocumentChangeEventArgs) {
        self.events.raise(&LineEvent::Complete(change.clone()));
    }
}

#[cfg(test)]
mod tests;
