//! `TreeBuilder<TObject, TKey>` (`Cache/Internal/TreeBuilder.cs`).

use super::transform::transform;
use crate::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, Node, ObservableCacheEx};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// A predicate of the nodes of a tree (`Func<Node<TObject, TKey>, bool>`).
pub(crate) type NodePredicate<T, K> = Rc<dyn Fn(&Rc<Node<T, K>>) -> bool>;

/// The default filter of the tree: its roots (`DefaultPredicate`).
fn default_predicate<T: Clone + 'static, K: Eq + Hash + Clone + 'static>() -> NodePredicate<T, K> {
    Rc::new(|node: &Rc<Node<T, K>>| node.is_root())
}

/// The items of the source as the nodes of a tree, each the child of the node whose key
/// `pivot_on` gives for it, filtered by the latest predicate of `predicate_changed` (by default
/// the roots) (`TreeBuilder.Run`).
///
/// The nodes are kept as the items change: an addition finds the node of its parent and its
/// child nodes; an update moves the children to the new node and moves it to its new parent; a
/// removal makes its children roots; a refresh moves the node when its parent changed. After each
/// change set every node is filtered again. A node that leaves the tree is disposed.
pub(crate) fn tree_builder<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    pivot_on: Rc<dyn Fn(&T) -> K>,
    predicate_changed: Option<Rc<dyn IObservable<NodePredicate<T, K>>>>,
) -> Rc<dyn IObservable<ChangeSet<Rc<Node<T, K>>, K>>> {
    let predicate_changed = predicate_changed.unwrap_or_else(|| Observable::return_(default_predicate::<T, K>()));
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Node<T, K>>, K>>>| {
        // Upstream serializes the sources through a delivery queue; on one thread they are
        // delivered in order as they come (docs/porting/reactive.md, section 5.4).
        let re_filter_observable = BehaviorSubject::new(());

        let all_data = source.as_observable_cache();

        // For each item, a node, which holds the parent and the children.
        let all_nodes = transform(all_data.connect(), |item: &T, _previous: Option<&T>, key: &K| Ok(Node::new(item.clone(), key.clone())), None, false).as_observable_cache();

        // `allNodes.Connect().Group(x => pivotOn(x.Item)).AsObservableCache()`: the nodes of each
        // key of a parent. Upstream's group subscribes to the nodes before the parent setter, so
        // the groups are brought up to date at the start of each change set of the setter.
        let grouped_by_pivot = Rc::new(RefCell::new(PivotGroups::<T, K>::new()));

        // The nodes that are in the tree, to dispose those that leave it (`DisposeMany`).
        let tracked = Rc::new(RefCell::new(Dictionary::<K, Rc<Node<T, K>>>::new()));

        // As the nodes change, keep their parents and children.
        let (setter_nodes, setter_groups, setter_pivot, setter_re_filter, setter_tracked) =
            (all_nodes.clone(), grouped_by_pivot.clone(), pivot_on.clone(), re_filter_observable.clone(), tracked.clone());
        let parent_setter = all_nodes.connect().subscribe_next(move |changes: ChangeSet<Rc<Node<T, K>>, K>| {
            setter_groups.borrow_mut().update(&changes, &*setter_pivot);
            for (parent_key, group) in group_by(&changes, |change| setter_pivot(change.current().item())) {
                match setter_nodes.lookup(&parent_key) {
                    // The items without a parent.
                    None => {
                        for change in group {
                            if change.reason() != ChangeReason::Refresh {
                                change.current().set_parent(None);
                            }
                            match change.reason() {
                                ChangeReason::Add => update_children(&setter_groups, change.current()),
                                ChangeReason::Update => {
                                    // Copy the children to the new node and set their parent.
                                    let previous = change.previous().expect("an update has a previous item");
                                    let children = previous.children().items();
                                    change.current().update(|updater| updater.add_or_update_items(&mut children.iter().cloned()));
                                    for child in &children {
                                        child.set_parent(Some(change.current()));
                                    }

                                    // Remove it from the old parent if different.
                                    let previous_parent = setter_pivot(previous.item());
                                    if previous_parent != *previous.key() {
                                        if let Some(node) = setter_nodes.lookup(&previous_parent) {
                                            node.update(|updater| updater.remove_key(change.key()));
                                        }
                                    }
                                }
                                ChangeReason::Remove => {
                                    // Remove the children and clear their parent.
                                    let children = change.current().children().items();
                                    change.current().update(|updater| updater.remove_items(&mut children.iter().cloned()));
                                    for child in &children {
                                        child.set_parent(None);
                                    }
                                }
                                ChangeReason::Refresh => {
                                    let previous_parent = change.current().parent();
                                    // `previousParent.Equals(parent)`, `parent` being none here.
                                    if let Some(previous_parent) = previous_parent {
                                        previous_parent.update(|updater| updater.remove_key(change.key()));
                                        change.current().set_parent(None);
                                    }
                                }
                                ChangeReason::Moved => {}
                            }
                        }
                    }
                    // The items with a parent.
                    Some(parent) => {
                        parent.update(|updater| {
                            for change in &group {
                                let node = change.current();
                                let key = node.key().clone();
                                match change.reason() {
                                    ChangeReason::Add => {
                                        // Update the parent node.
                                        node.set_parent(Some(&parent));
                                        updater.add_or_update(node.clone());
                                        update_children(&setter_groups, node);
                                    }
                                    ChangeReason::Update => {
                                        // Copy the children to the new node and set their parent.
                                        let previous = change.previous().expect("an update has a previous item");
                                        let children = previous.children().items();
                                        node.update(|child_updater| child_updater.add_or_update_items(&mut children.iter().cloned()));
                                        for child in &children {
                                            child.set_parent(Some(node));
                                        }

                                        // Check whether the item has a new parent.
                                        let previous_parent = setter_pivot(previous.item());
                                        if previous_parent != *previous.key() {
                                            if let Some(previous_parent_node) = setter_nodes.lookup(&previous_parent) {
                                                previous_parent_node.update(|parent_updater| parent_updater.remove_key(&key));
                                            }
                                        }

                                        // Finally update the parent.
                                        node.set_parent(Some(&parent));
                                        updater.add_or_update(node.clone());
                                    }
                                    ChangeReason::Remove => {
                                        node.set_parent(None);
                                        updater.remove_key(&key);

                                        let children = node.children().items();
                                        node.update(|child_updater| child_updater.remove_items(&mut children.iter().cloned()));
                                        for child in &children {
                                            child.set_parent(None);
                                        }
                                    }
                                    ChangeReason::Refresh => {
                                        let previous_parent = node.parent();
                                        let same_parent = previous_parent.as_ref().is_some_and(|previous_parent| previous_parent.key() == parent.key());
                                        if !same_parent {
                                            if let Some(previous_parent) = previous_parent {
                                                previous_parent.update(|parent_updater| parent_updater.remove_key(&key));
                                            }
                                            node.set_parent(Some(&parent));
                                            updater.add_or_update(node.clone());
                                        }
                                    }
                                    ChangeReason::Moved => {}
                                }
                            }
                        });
                    }
                }
            }

            setter_re_filter.on_next(());

            // `DisposeMany`: after the parents are set, the nodes that left are disposed.
            dispose_many(&setter_tracked, &changes);
        });

        // Both inputs are delivered in order on one thread; `CombineLatest` keeps the latest
        // predicate and filters again on each value of the refilter.
        let filter = predicate_changed.combine_latest(re_filter_observable.as_observable(), |predicate: NodePredicate<T, K>, _: ()| predicate);
        let result = all_nodes.connect().filter_dynamic(filter).subscribe_safe(observer);

        let (end_tracked, end_re_filter) = (tracked.clone(), re_filter_observable.clone());
        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([
            result,
            parent_setter,
            // The nodes still in the tree, as `DisposeMany` disposes them with its subscription.
            Disposable::create(move || {
                let nodes: Vec<Rc<Node<T, K>>> = end_tracked.borrow_mut().values().cloned().collect();
                end_tracked.borrow_mut().clear();
                for node in nodes {
                    node.dispose();
                }
            }),
            Rc::new(all_data) as Rc<dyn IDisposable>,
            Rc::new(all_nodes) as Rc<dyn IDisposable>,
            Disposable::create(move || end_re_filter.on_completed()),
        ]);
        disposables
    })
}

/// `UpdateChildren`: the nodes whose parent is `parent_node` become its children.
fn update_children<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(grouped_by_pivot: &RefCell<PivotGroups<T, K>>, parent_node: &Rc<Node<T, K>>) {
    let children = grouped_by_pivot.borrow().children_of(parent_node.key());
    if let Some(children) = children {
        parent_node.update(|updater| updater.add_or_update_items(&mut children.iter().cloned()));
        for child in &children {
            child.set_parent(Some(parent_node));
        }
    }
}

/// `DisposeMany`: a node is disposed when it is removed or replaced by another.
fn dispose_many<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(tracked: &RefCell<Dictionary<K, Rc<Node<T, K>>>>, changes: &ChangeSet<Rc<Node<T, K>>, K>) {
    for change in changes {
        match change.reason() {
            ChangeReason::Add | ChangeReason::Update => {
                let replaced = tracked.borrow_mut().insert(change.key().clone(), change.current().clone());
                if let Some(replaced) = replaced {
                    if !Rc::ptr_eq(&replaced, change.current()) {
                        replaced.dispose();
                    }
                }
            }
            ChangeReason::Remove => {
                let removed = tracked.borrow_mut().remove(change.key());
                if let Some(removed) = removed {
                    removed.dispose();
                }
            }
            ChangeReason::Refresh | ChangeReason::Moved => {}
        }
    }
}

/// The changes grouped by a key, the groups in the order of their first change
/// (`Enumerable.GroupBy`).
fn group_by<'a, T, K, G: PartialEq>(changes: &'a ChangeSet<T, K>, group_key: impl Fn(&Change<T, K>) -> G) -> Vec<(G, Vec<&'a Change<T, K>>)> {
    let mut groups: Vec<(G, Vec<&'a Change<T, K>>)> = Vec::new();
    for change in changes {
        let key = group_key(change);
        match groups.iter_mut().find(|(candidate, _)| *candidate == key) {
            Some((_, group)) => group.push(change),
            None => groups.push((key, vec![change])),
        }
    }
    groups
}

/// The nodes of each key of a parent (the groups of `Group(x => pivotOn(x.Item))`, kept as
/// `GroupOn.Grouper` keeps them): the nodes of a group in the order of their cache, and the
/// group of each node.
struct PivotGroups<T, K> {
    groups: Dictionary<K, Dictionary<K, Rc<Node<T, K>>>>,
    item_groups: Dictionary<K, K>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> PivotGroups<T, K> {
    fn new() -> Self {
        Self { groups: Dictionary::new(), item_groups: Dictionary::new() }
    }

    /// The nodes of the group of `parent_key` (`groupedByPivot.Lookup(key).Cache.Items`).
    fn children_of(&self, parent_key: &K) -> Option<Vec<Rc<Node<T, K>>>> {
        self.groups.get(parent_key).map(|group| group.values().cloned().collect())
    }

    /// `Grouper.HandleUpdates`, without the change sets of the groups.
    fn update(&mut self, changes: &ChangeSet<Rc<Node<T, K>>, K>, pivot_on: &dyn Fn(&T) -> K) {
        for (group_key, group) in group_by(changes, |change| pivot_on(change.current().item())) {
            if !self.groups.contains_key(&group_key) {
                self.groups.insert(group_key.clone(), Dictionary::new());
            }
            for change in group {
                let key = change.key();
                match change.reason() {
                    ChangeReason::Add | ChangeReason::Update => {
                        self.group_mut(&group_key).insert(key.clone(), change.current().clone());
                        // Remove it from its previous group when that is another.
                        if let Some(previous_group) = self.item_groups.get(key).cloned() {
                            if previous_group != group_key {
                                self.remove_from_group(&previous_group, key);
                            }
                        }
                        self.item_groups.insert(key.clone(), group_key.clone());
                    }
                    ChangeReason::Remove => {
                        let in_same_group = self.groups.get(&group_key).is_some_and(|group| group.contains_key(key));
                        if in_same_group {
                            self.group_mut(&group_key).remove(key);
                        } else if let Some(previous_group) = self.item_groups.get(key).cloned() {
                            // Removed by an evaluation that moved it to another group.
                            self.remove_from_group(&previous_group, key);
                        }
                        self.item_groups.remove(key);
                    }
                    ChangeReason::Refresh => {
                        match self.item_groups.get(key).cloned() {
                            Some(previous_group) if previous_group == group_key => {}
                            Some(previous_group) => {
                                self.remove_from_group(&previous_group, key);
                                self.group_mut(&group_key).insert(key.clone(), change.current().clone());
                            }
                            None => {
                                self.group_mut(&group_key).insert(key.clone(), change.current().clone());
                            }
                        }
                        self.item_groups.insert(key.clone(), group_key.clone());
                    }
                    ChangeReason::Moved => {}
                }
            }
            if self.groups.get(&group_key).is_some_and(|group| group.is_empty()) {
                self.groups.remove(&group_key);
            }
        }
    }

    /// The group of `group_key`, made when it is not there (`GetCache`).
    fn group_mut(&mut self, group_key: &K) -> &mut Dictionary<K, Rc<Node<T, K>>> {
        if !self.groups.contains_key(group_key) {
            self.groups.insert(group_key.clone(), Dictionary::new());
        }
        self.groups.get_mut(group_key).expect("the group was made")
    }

    /// Removes `key` from the group of `group_key`, and the group when it is left empty.
    fn remove_from_group(&mut self, group_key: &K, key: &K) {
        let now_empty = match self.groups.get_mut(group_key) {
            Some(group) => {
                group.remove(key);
                group.is_empty()
            }
            None => false,
        };
        if now_empty {
            self.groups.remove(group_key);
        }
    }
}
