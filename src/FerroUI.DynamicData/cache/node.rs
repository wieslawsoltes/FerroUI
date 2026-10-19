//! `Node<TObject, TKey>` (`Cache/Node.cs`).

use super::{IObservableCache, ISourceUpdater, ObservableCache, SourceCache};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

/// A node of a tree (`Node<TObject, TKey>`): an item, its key, its parent and its children, as
/// `TransformToTree` builds them. Nodes are held in `Rc`.
///
/// Deviation (DEVIATIONS.md, DynamicData): a node holds its parent weakly, so that a parent and
/// its children (which it holds) do not keep each other alive; the parent of a node is the node
/// of the tree that is its parent, which the tree keeps.
pub struct Node<T, K> {
    children_source: SourceCache<Rc<Node<T, K>>, K>,
    children: ObservableCache<Rc<Node<T, K>>, K>,
    item: T,
    key: K,
    parent: RefCell<Option<Weak<Node<T, K>>>>,
    is_disposed: Cell<bool>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Node<T, K> {
    /// A node without a parent (`Node(TObject, TKey)`).
    pub fn new(item: T, key: K) -> Rc<Self> {
        Self::with_parent(item, key, None)
    }

    /// A node of `parent` (`Node(TObject, TKey, in Optional<Node<TObject, TKey>>)`).
    pub fn with_parent(item: T, key: K, parent: Option<&Rc<Node<T, K>>>) -> Rc<Self> {
        let children_source = SourceCache::new(|node: &Rc<Node<T, K>>| node.key.clone());
        let children = children_source.as_observable_cache_inner().clone();
        Rc::new(Self { children_source, children, item, key, parent: RefCell::new(parent.map(Rc::downgrade)), is_disposed: Cell::new(false) })
    }

    /// The child nodes (`Children`).
    pub fn children(&self) -> &ObservableCache<Rc<Node<T, K>>, K> {
        &self.children
    }

    /// The number of ancestors of the node (`Depth`).
    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut parent = self.parent();
        while let Some(node) = parent {
            depth += 1;
            parent = node.parent();
        }
        depth
    }

    /// Whether the node has no parent (`IsRoot`).
    pub fn is_root(&self) -> bool {
        self.parent().is_none()
    }

    /// The item (`Item`).
    pub fn item(&self) -> &T {
        &self.item
    }

    /// The key of the item (`Key`).
    pub fn key(&self) -> &K {
        &self.key
    }

    /// The parent node (`Parent`).
    pub fn parent(&self) -> Option<Rc<Node<T, K>>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// Sets the parent node (the internal setter of `Parent`).
    pub(crate) fn set_parent(&self, parent: Option<&Rc<Node<T, K>>>) {
        *self.parent.borrow_mut() = parent.map(Rc::downgrade);
    }

    /// Edits the children (`Update(Action<ISourceUpdater>)`).
    pub(crate) fn update(&self, update_action: impl FnOnce(&mut dyn ISourceUpdater<Rc<Node<T, K>>, K>)) {
        self.children_source.edit(update_action);
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for Node<T, K> {
    /// Disposes the children (`Dispose`).
    fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }
        // `Children` is a view of the source of the children here: one disposal ends both.
        self.children_source.dispose();
    }
}

impl<T, K: PartialEq> PartialEq for Node<T, K> {
    /// Nodes of the same key are equal (`Equals(Node<TObject, TKey>)`).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.key == other.key
    }
}

impl<T, K: Eq> Eq for Node<T, K> {}

impl<T, K: Hash> Hash for Node<T, K> {
    /// `GetHashCode`: the hash of the key.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<T: fmt::Display + Clone + 'static, K: Eq + Hash + Clone + 'static> fmt::Display for Node<T, K> {
    /// `ToString`: the item, and the number of children when there are any.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.children.count();
        if count == 0 {
            write!(f, "{}", self.item)
        } else {
            write!(f, "{} ({} children)", self.item, count)
        }
    }
}

impl<T: fmt::Debug, K: fmt::Debug> fmt::Debug for Node<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node").field("key", &self.key).field("item", &self.item).finish()
    }
}
