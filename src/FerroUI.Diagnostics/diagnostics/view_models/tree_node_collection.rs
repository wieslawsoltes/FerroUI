//! Port of `Diagnostics/ViewModels/TreeNodeCollection.cs`.

use super::TreeNode;
use ferroui_base::collections::{CollectionChangedHandler, FerroList, IFerroReadOnlyList};
use ferroui_base::data::model::{CollectionChange, Event, INotifyCollectionChanged, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::{IItemsList, ItemsChangedHandler};
use ferroui_treedatagrid_core::model_collection::{ModelCollection, ModelsChangedHandler};
use std::any::Any;
use std::cell::OnceCell;
use std::rc::{Rc, Weak};

/// The members a kind of [`TreeNodeCollection`] overrides: the abstract
/// `Initialize` and the virtual `Dispose` of the original.
pub trait TreeNodeCollectionImpl {
    /// `protected abstract void Initialize(FerroList<TreeNode> nodes)`:
    /// fills `nodes` and keeps them up to date. Called once, when the
    /// collection is first used.
    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>);

    /// `public virtual void Dispose()`. The base implementation disposes the
    /// nodes ([`TreeNodeCollection::base_dispose`]); an override calls it
    /// where the original calls `base.Dispose()`.
    fn dispose(&self, this: &TreeNodeCollection) {
        this.base_dispose();
    }
}

/// The children of a [`TreeNode`]: a read-only list of nodes that is filled
/// the first time it is used (its count, an item, an enumeration or a
/// subscription to its changes), and then follows the tree it shows.
///
/// The kinds of the original (the collections of the logical and visual
/// trees, of a group of top levels, of an application, and the empty one)
/// derive from it; here they implement [`TreeNodeCollectionImpl`], which the
/// collection holds.
pub struct TreeNodeCollection {
    this: Weak<TreeNodeCollection>,
    owner: Weak<TreeNode>,
    inner: OnceCell<FerroList<Rc<TreeNode>>>,
    kind: Box<dyn TreeNodeCollectionImpl>,
}

/// `EmptyTreeNodeCollection`: the collection of a node without children.
struct EmptyTreeNodeCollection;

impl TreeNodeCollectionImpl for EmptyTreeNodeCollection {
    fn initialize(&self, _this: &TreeNodeCollection, _nodes: &FerroList<Rc<TreeNode>>) {}
}

ferroui_base::ferro_thread_local! {
    static EMPTY: Rc<TreeNodeCollection> = TreeNodeCollection::new(Weak::new(), Box::new(EmptyTreeNodeCollection));
}

impl PartialEq for TreeNodeCollection {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TreeNodeCollection {
    /// `TreeNodeCollection(TreeNode owner)`: a collection of the children of
    /// `owner`, of the kind `kind`.
    pub fn new(owner: Weak<TreeNode>, kind: Box<dyn TreeNodeCollectionImpl>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), owner, inner: OnceCell::new(), kind })
    }

    /// The shared handle of the collection (a collection is always shared).
    pub(crate) fn to_rc(&self) -> Rc<TreeNodeCollection> {
        self.this.upgrade().expect("the collection is alive while it is used")
    }

    /// `static readonly internal TreeNodeCollection Empty`: the collection of
    /// the nodes without children (one per thread).
    pub fn empty() -> Rc<TreeNodeCollection> {
        EMPTY.with(Rc::clone)
    }

    /// `this[int index]`: the node at `index`.
    ///
    /// # Panics
    /// Panics when `index` is out of range.
    pub fn get(&self, index: usize) -> Rc<TreeNode> {
        self.ensure_initialized().get(index)
    }

    /// `Count`: the number of nodes.
    pub fn count(&self) -> usize {
        self.ensure_initialized().count()
    }

    /// `Owner`: the node whose children the collection holds; `None` for the
    /// empty collection, and once the node is gone.
    pub fn owner(&self) -> Option<Rc<TreeNode>> {
        self.owner.upgrade()
    }

    /// `Dispose()`: the override of the kind of the collection.
    pub fn dispose(&self) {
        self.kind.dispose(self);
    }

    /// The base implementation of `Dispose()`: disposes the nodes, if the
    /// collection was ever filled.
    pub fn base_dispose(&self) {
        if let Some(inner) = self.inner.get() {
            for node in inner.snapshot().iter() {
                node.dispose();
            }
        }
    }

    /// Whether the collection was filled: whether its nodes exist. A collection is filled the first
    /// time it is used (its count, an item, an enumeration or a subscription to its changes); the
    /// tree pages ask for the children of a node when its row is shown or expanded.
    pub fn is_initialized(&self) -> bool {
        self.inner.get().is_some()
    }

    /// `GetEnumerator()`: the nodes, as they are now.
    pub fn to_vec(&self) -> Vec<Rc<TreeNode>> {
        self.ensure_initialized().to_vec()
    }

    /// `IList.Contains(object)`.
    pub fn contains(&self, node: &Rc<TreeNode>) -> bool {
        self.ensure_initialized().contains(node)
    }

    /// `IList.IndexOf(object)`: the index of `node`, or `-1`.
    pub fn index_of(&self, node: &Rc<TreeNode>) -> i32 {
        self.ensure_initialized().index_of(node).map_or(-1, |index| index as i32)
    }

    /// The list of the nodes, created and filled by the kind of the
    /// collection the first time it is needed. As in the original, the list
    /// is stored before it is filled: what the filling triggers sees it.
    fn ensure_initialized(&self) -> &FerroList<Rc<TreeNode>> {
        if self.inner.get().is_none() {
            let nodes = FerroList::new();
            let _ = self.inner.set(nodes.clone());
            self.kind.initialize(self, &nodes);
        }
        self.inner.get().expect("the nodes were just created")
    }
}

impl INotifyCollectionChanged for TreeNodeCollection {
    /// `CollectionChanged`: subscribing fills the collection.
    fn collection_changed(&self) -> &Event<CollectionChange> {
        self.ensure_initialized().collection_changed()
    }
}

impl INotifyPropertyChanged for TreeNodeCollection {
    /// `PropertyChanged`: subscribing fills the collection.
    fn property_changed(&self) -> &Event<str> {
        self.ensure_initialized().property_changed()
    }
}

impl IFerroReadOnlyList<Rc<TreeNode>> for TreeNodeCollection {
    fn count(&self) -> usize {
        TreeNodeCollection::count(self)
    }

    fn get(&self, index: usize) -> Rc<TreeNode> {
        TreeNodeCollection::get(self, index)
    }

    fn snapshot(&self) -> Rc<Vec<Rc<TreeNode>>> {
        self.ensure_initialized().snapshot()
    }

    fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<Rc<TreeNode>>>) -> u64 {
        self.ensure_initialized().add_collection_changed(handler)
    }

    fn remove_collection_changed(&self, token: u64) -> bool {
        self.ensure_initialized().remove_collection_changed(token)
    }
}

/// The collection as the items of an items control (the untyped `IList` of
/// the original, which the tree view reads).
impl IItemsList for TreeNodeCollection {
    fn count(&self) -> usize {
        TreeNodeCollection::count(self)
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        <FerroList<Rc<TreeNode>> as IItemsList>::get_at(self.ensure_initialized(), index)
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        <FerroList<Rc<TreeNode>> as IItemsList>::index_of(self.ensure_initialized(), item)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        <FerroList<Rc<TreeNode>> as IItemsList>::add_collection_changed(self.ensure_initialized(), handler)
    }

    fn remove_collection_changed(&self, token: u64) {
        <FerroList<Rc<TreeNode>> as IItemsList>::remove_collection_changed(self.ensure_initialized(), token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Deviation (DEVIATIONS.md, Developer tools): the tree pages show the nodes in a TreeDataGrid, not
// in a tree view; the collection is the collection of the child models of a node for the
// hierarchical source of a page (`TreePageViewModel::nodes_source`), which subscribes to its
// changes.
impl ModelCollection<TreeNode> for TreeNodeCollection {
    fn count(&self) -> usize {
        TreeNodeCollection::count(self)
    }

    fn get(&self, index: usize) -> Rc<TreeNode> {
        TreeNodeCollection::get(self, index)
    }

    fn index_of(&self, model: &Rc<TreeNode>) -> Option<usize> {
        self.ensure_initialized().index_of(model)
    }

    fn add_collection_changed(&self, handler: Rc<ModelsChangedHandler<TreeNode>>) -> Option<u64> {
        Some(self.ensure_initialized().add_collection_changed(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.ensure_initialized().remove_collection_changed(token);
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn to_vec(&self) -> Vec<Rc<TreeNode>> {
        TreeNodeCollection::to_vec(self)
    }
}

ferro_markup_type!(class TreeNodeCollection {
    this: Rc<TreeNodeCollection>,
    handles: [TreeNodeCollection, Rc<TreeNodeCollection>, Option<Rc<TreeNodeCollection>>],
    properties: [
        Count: i32 { get: |this: &Rc<TreeNodeCollection>| this.count() as i32 },
    ],
});
