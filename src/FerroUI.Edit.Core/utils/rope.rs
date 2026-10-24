use std::sync::Arc;

use super::Utf16Memory;

const LEAF_CAPACITY: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Metrics {
    length: usize,
    breaks: usize,
    starts_lf: bool,
    ends_cr: bool,
}

impl Metrics {
    fn scan(text: &[u16]) -> Self {
        let mut breaks = 0;
        let mut previous_cr = false;
        for &unit in text {
            if unit == 13 || (unit == 10 && !previous_cr) {
                breaks += 1;
            }
            previous_cr = unit == 13;
        }
        Self {
            length: text.len(),
            breaks,
            starts_lf: text.first() == Some(&10),
            ends_cr: text.last() == Some(&13),
        }
    }

    fn join(self, right: Self) -> Self {
        if self.length == 0 {
            return right;
        }
        if right.length == 0 {
            return self;
        }
        Self {
            length: self.length + right.length,
            breaks: self.breaks + right.breaks - usize::from(self.ends_cr && right.starts_lf),
            starts_lf: self.starts_lf,
            ends_cr: right.ends_cr,
        }
    }
}

#[derive(Clone)]
struct Node {
    metrics: Metrics,
    height: u16,
    kind: NodeKind,
}

#[derive(Clone)]
enum NodeKind {
    Leaf {
        data: Arc<[u16]>,
        start: usize,
        length: usize,
    },
    Branch {
        left: Arc<Node>,
        right: Arc<Node>,
    },
}

impl Node {
    fn leaf(data: Arc<[u16]>, start: usize, length: usize) -> Arc<Self> {
        Arc::new(Self {
            metrics: Metrics::scan(&data[start..start + length]),
            height: 1,
            kind: NodeKind::Leaf {
                data,
                start,
                length,
            },
        })
    }

    fn branch(left: Arc<Self>, right: Arc<Self>) -> Arc<Self> {
        Arc::new(Self {
            metrics: left.metrics.join(right.metrics),
            height: 1 + left.height.max(right.height),
            kind: NodeKind::Branch { left, right },
        })
    }
}

type Link = Option<Arc<Node>>;

fn balance(left: Arc<Node>, right: Arc<Node>) -> Arc<Node> {
    if left.height > right.height + 1 {
        if let NodeKind::Branch {
            left: ll,
            right: lr,
        } = &left.kind
        {
            if ll.height >= lr.height {
                return Node::branch(ll.clone(), Node::branch(lr.clone(), right));
            }
            if let NodeKind::Branch {
                left: lrl,
                right: lrr,
            } = &lr.kind
            {
                return Node::branch(
                    Node::branch(ll.clone(), lrl.clone()),
                    Node::branch(lrr.clone(), right),
                );
            }
        }
    } else if right.height > left.height + 1 {
        if let NodeKind::Branch {
            left: rl,
            right: rr,
        } = &right.kind
        {
            if rr.height >= rl.height {
                return Node::branch(Node::branch(left, rl.clone()), rr.clone());
            }
            if let NodeKind::Branch {
                left: rll,
                right: rlr,
            } = &rl.kind
            {
                return Node::branch(
                    Node::branch(left, rll.clone()),
                    Node::branch(rlr.clone(), rr.clone()),
                );
            }
        }
    }
    Node::branch(left, right)
}

fn concat_nodes(left: Arc<Node>, right: Arc<Node>) -> Arc<Node> {
    if left.metrics.length + right.metrics.length <= LEAF_CAPACITY {
        let mut text = Vec::with_capacity(left.metrics.length + right.metrics.length);
        copy_node(&left, 0, left.metrics.length, &mut text);
        copy_node(&right, 0, right.metrics.length, &mut text);
        let length = text.len();
        return Node::leaf(text.into(), 0, length);
    }
    if left.height > right.height + 1 {
        if let NodeKind::Branch {
            left: ll,
            right: lr,
        } = &left.kind
        {
            return balance(ll.clone(), concat_nodes(lr.clone(), right));
        }
    }
    if right.height > left.height + 1 {
        if let NodeKind::Branch {
            left: rl,
            right: rr,
        } = &right.kind
        {
            return balance(concat_nodes(left, rl.clone()), rr.clone());
        }
    }
    Node::branch(left, right)
}

fn concat(left: Link, right: Link) -> Link {
    match (left, right) {
        (Some(left), Some(right)) => Some(concat_nodes(left, right)),
        (left, None) => left,
        (None, right) => right,
    }
}

fn split(node: Link, offset: usize) -> (Link, Link) {
    let Some(node) = node else {
        return (None, None);
    };
    if offset == 0 {
        return (None, Some(node));
    }
    if offset == node.metrics.length {
        return (Some(node), None);
    }
    match &node.kind {
        NodeKind::Leaf {
            data,
            start,
            length,
        } => (
            Some(Node::leaf(data.clone(), *start, offset)),
            Some(Node::leaf(data.clone(), start + offset, length - offset)),
        ),
        NodeKind::Branch { left, right } => {
            if offset < left.metrics.length {
                let (a, b) = split(Some(left.clone()), offset);
                (a, concat(b, Some(right.clone())))
            } else {
                let (a, b) = split(Some(right.clone()), offset - left.metrics.length);
                (concat(Some(left.clone()), a), b)
            }
        }
    }
}

fn copy_node(node: &Node, offset: usize, length: usize, result: &mut Vec<u16>) {
    if length == 0 {
        return;
    }
    match &node.kind {
        NodeKind::Leaf { data, start, .. } => {
            result.extend_from_slice(&data[start + offset..start + offset + length])
        }
        NodeKind::Branch { left, right } => {
            let left_length = left.metrics.length;
            if offset < left_length {
                let count = length.min(left_length - offset);
                copy_node(left, offset, count, result);
                if count < length {
                    copy_node(right, 0, length - count, result);
                }
            } else {
                copy_node(right, offset - left_length, length, result);
            }
        }
    }
}

fn prefix(node: &Node, length: usize) -> Metrics {
    if length == node.metrics.length {
        return node.metrics;
    }
    match &node.kind {
        NodeKind::Leaf { data, start, .. } => Metrics::scan(&data[*start..start + length]),
        NodeKind::Branch { left, right } => {
            if length <= left.metrics.length {
                prefix(left, length)
            } else {
                left.metrics
                    .join(prefix(right, length - left.metrics.length))
            }
        }
    }
}

fn break_end(node: &Node, index: usize) -> usize {
    match &node.kind {
        NodeKind::Leaf {
            data,
            start,
            length,
        } => {
            let data = &data[*start..start + length];
            let mut remaining = index;
            let mut i = 0;
            while i < data.len() {
                if data[i] == 13 || data[i] == 10 {
                    if data[i] == 13 && data.get(i + 1) == Some(&10) {
                        i += 1;
                    }
                    remaining -= 1;
                    if remaining == 0 {
                        return i + 1;
                    }
                }
                i += 1;
            }
            unreachable!("line break index is checked by the rope")
        }
        NodeKind::Branch { left, right } => {
            let merged = left.metrics.ends_cr && right.metrics.starts_lf;
            if merged && index == left.metrics.breaks {
                left.metrics.length + 1
            } else if index <= left.metrics.breaks {
                break_end(left, index)
            } else {
                left.metrics.length
                    + break_end(right, index - left.metrics.breaks + usize::from(merged))
            }
        }
    }
}

/// Persistent AVL rope indexed in UTF-16 code units, including lone surrogates.
/// Clones and snapshots share immutable nodes. Edits copy a logarithmic path and
/// at most two small boundary leaves; newline metrics join CRLF across leaves.
#[derive(Clone, Default)]
pub struct Utf16Rope {
    root: Link,
}

impl std::fmt::Debug for Utf16Rope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Utf16Rope")
            .field("length", &self.len())
            .field("lines", &self.line_count())
            .finish()
    }
}

impl Utf16Rope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_utf16(text: &[u16]) -> Self {
        fn build(text: &[u16]) -> Link {
            if text.is_empty() {
                return None;
            }
            if text.len() <= LEAF_CAPACITY {
                return Some(Node::leaf(Arc::from(text), 0, text.len()));
            }
            let middle = text.len() / 2;
            concat(build(&text[..middle]), build(&text[middle..]))
        }
        Self { root: build(text) }
    }

    pub fn from_text(text: &str) -> Self {
        Self::from_utf16(&text.encode_utf16().collect::<Vec<_>>())
    }
    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |n| n.metrics.length)
    }
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
    pub fn line_count(&self) -> usize {
        self.root.as_ref().map_or(1, |n| n.metrics.breaks + 1)
    }

    pub fn get(&self, mut index: usize) -> Option<u16> {
        if index >= self.len() {
            return None;
        }
        let mut node = self.root.as_deref()?;
        loop {
            match &node.kind {
                NodeKind::Leaf { data, start, .. } => return Some(data[start + index]),
                NodeKind::Branch { left, right } => {
                    if index < left.metrics.length {
                        node = left;
                    } else {
                        index -= left.metrics.length;
                        node = right;
                    }
                }
            }
        }
    }

    pub fn to_utf16(&self) -> Vec<u16> {
        let mut result = Vec::with_capacity(self.len());
        if let Some(node) = &self.root {
            copy_node(node, 0, self.len(), &mut result);
        }
        result
    }

    /// UTF-8 projection; lone UTF-16 surrogates project to U+FFFD. Use
    /// [`to_utf16`](Self::to_utf16) for lossless access to arbitrary document data.
    pub fn text(&self) -> String {
        String::from_utf16_lossy(&self.to_utf16())
    }

    pub fn slice(&self, offset: usize, length: usize) -> Option<Self> {
        if offset > self.len() || length > self.len() - offset {
            return None;
        }
        let (_, tail) = split(self.root.clone(), offset);
        let (root, _) = split(tail, length);
        Some(Self { root })
    }

    pub fn replace(&self, offset: usize, length: usize, insertion: &Self) -> Option<Self> {
        if offset > self.len() || length > self.len() - offset {
            return None;
        }
        self.len()
            .checked_sub(length)?
            .checked_add(insertion.len())?;
        let (left, tail) = split(self.root.clone(), offset);
        let (_, right) = split(tail, length);
        Some(Self {
            root: concat(concat(left, insertion.root.clone()), right),
        })
    }

    /// Zero-based logical line containing the insertion position. The position
    /// between CR and LF belongs to the preceding line.
    pub fn line_index_at(&self, offset: usize) -> Option<usize> {
        if offset > self.len() {
            return None;
        }
        let Some(root) = &self.root else {
            return Some(0);
        };
        let metrics = prefix(root, offset);
        Some(metrics.breaks - usize::from(metrics.ends_cr && self.get(offset) == Some(10)))
    }

    pub fn line_start(&self, line_index: usize) -> Option<usize> {
        if line_index == 0 {
            return Some(0);
        }
        let root = self.root.as_ref()?;
        if line_index > root.metrics.breaks {
            return None;
        }
        Some(break_end(root, line_index))
    }

    /// An owned, snapshot-retaining iterator over shared contiguous UTF-16 memory.
    /// It keeps a logarithmic traversal stack and does not flatten the rope.
    pub fn chunks(&self) -> RopeChunks {
        self.chunks_range(0, self.len()).expect("valid whole rope")
    }

    pub fn chunks_range(&self, offset: usize, length: usize) -> Option<RopeChunks> {
        if offset > self.len() || length > self.len() - offset {
            return None;
        }
        let mut stack = Vec::new();
        if length != 0 {
            stack.push((self.root.clone()?, offset, length));
        }
        Some(RopeChunks {
            stack,
            remaining: length,
        })
    }

    /// Shares an existing leaf for a contiguous range; only a range spanning
    /// multiple leaves needs a new allocation. Empty ranges allocate no data.
    pub fn get_memory(&self, offset: usize, length: usize) -> Option<Utf16Memory> {
        let mut chunks = self.chunks_range(offset, length)?;
        let Some(first) = chunks.next() else {
            return Some(Utf16Memory::default());
        };
        if first.len() == length {
            return Some(first);
        }
        let mut result = Vec::with_capacity(length);
        result.extend_from_slice(&first);
        for chunk in chunks {
            result.extend_from_slice(&chunk);
        }
        Some(Utf16Memory::from(result))
    }

    /// Visits immutable contiguous chunks without materializing the document.
    pub fn for_each_chunk(&self, mut visitor: impl FnMut(&[u16])) {
        fn visit(node: &Node, visitor: &mut impl FnMut(&[u16])) {
            match &node.kind {
                NodeKind::Leaf {
                    data,
                    start,
                    length,
                } => visitor(&data[*start..start + length]),
                NodeKind::Branch { left, right } => {
                    visit(left, visitor);
                    visit(right, visitor);
                }
            }
        }
        if let Some(root) = &self.root {
            visit(root, &mut visitor);
        }
    }
}

/// A forward iterator whose items retain their immutable leaf allocation.
/// Cloning gives an independent cursor; `size_hint` counts chunks, not units.
#[derive(Clone)]
pub struct RopeChunks {
    stack: Vec<(Arc<Node>, usize, usize)>,
    remaining: usize,
}

impl std::fmt::Debug for RopeChunks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RopeChunks")
            .field("remaining_units", &self.remaining)
            .field("pending_nodes", &self.stack.len())
            .finish()
    }
}

impl RopeChunks {
    pub fn remaining_units(&self) -> usize {
        self.remaining
    }
}

impl Iterator for RopeChunks {
    type Item = Utf16Memory;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, offset, length)) = self.stack.pop() {
            match &node.kind {
                NodeKind::Leaf { data, start, .. } => {
                    self.remaining -= length;
                    return Some(Utf16Memory::shared(data.clone(), start + offset, length));
                }
                NodeKind::Branch { left, right } => {
                    let left_length = left.metrics.length;
                    if offset < left_length {
                        let count = length.min(left_length - offset);
                        if count < length {
                            self.stack.push((right.clone(), 0, length - count));
                        }
                        self.stack.push((left.clone(), offset, count));
                    } else {
                        self.stack
                            .push((right.clone(), offset - left_length, length));
                    }
                }
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::from(self.remaining != 0), Some(self.remaining))
    }
}

impl std::iter::FusedIterator for RopeChunks {}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(node: &Node) {
        match &node.kind {
            NodeKind::Leaf {
                data,
                start,
                length,
            } => {
                assert!(*length > 0 && *length <= LEAF_CAPACITY);
                assert_eq!(node.metrics, Metrics::scan(&data[*start..start + length]));
                assert_eq!(node.height, 1);
            }
            NodeKind::Branch { left, right } => {
                assert!(left.height.abs_diff(right.height) <= 1);
                assert_eq!(node.metrics, left.metrics.join(right.metrics));
                assert_eq!(node.height, 1 + left.height.max(right.height));
                validate(left);
                validate(right);
            }
        }
    }

    fn line_starts(text: &[u16]) -> Vec<usize> {
        let mut result = vec![0];
        let mut i = 0;
        while i < text.len() {
            if text[i] == 13 || text[i] == 10 {
                if text[i] == 13 && text.get(i + 1) == Some(&10) {
                    i += 1;
                }
                result.push(i + 1);
            }
            i += 1;
        }
        result
    }

    #[test]
    fn random_edits_preserve_balance_text_and_crlf_metrics() {
        let mut rng = 0x492ace31u64;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng as usize
        };
        let mut text = vec![97u16; 8_000];
        let mut rope = Utf16Rope::from_utf16(&text);
        let snapshot = rope.clone();
        for step in 0..1600 {
            let offset = next() % (text.len() + 1);
            let removal = (next() % 93).min(text.len() - offset);
            let insertion: Vec<_> = (0..next() % 90)
                .map(|_| match next() % 10 {
                    0 => 13,
                    1 => 10,
                    2 => 0xd800,
                    _ => 97,
                })
                .collect();
            rope = rope
                .replace(offset, removal, &Utf16Rope::from_utf16(&insertion))
                .unwrap();
            text.splice(offset..offset + removal, insertion);
            assert_eq!(rope.to_utf16(), text, "step {step}");
            if let Some(root) = &rope.root {
                validate(root);
            }
            let starts = line_starts(&text);
            assert_eq!(rope.line_count(), starts.len());
            for (index, &start) in starts.iter().enumerate() {
                assert_eq!(rope.line_start(index), Some(start));
                assert_eq!(rope.line_index_at(start), Some(index));
            }
            for _ in 0..10 {
                let offset = next() % (text.len() + 1);
                let expected = starts.partition_point(|&s| s <= offset) - 1;
                assert_eq!(rope.line_index_at(offset), Some(expected));
            }
        }
        assert_eq!(snapshot.to_utf16(), vec![97; 8_000]);
    }

    #[test]
    fn repeated_tiny_appends_and_boundary_crlf() {
        let mut rope = Utf16Rope::new();
        for i in 0..5000 {
            let c = if i % 2 == 0 { 13 } else { 10 };
            rope = rope
                .replace(rope.len(), 0, &Utf16Rope::from_utf16(&[c]))
                .unwrap();
        }
        validate(rope.root.as_deref().unwrap());
        assert_eq!(rope.line_count(), 2501);
        for i in 0..=5000 {
            assert_eq!(rope.line_index_at(i), Some(i / 2));
        }
    }
}
