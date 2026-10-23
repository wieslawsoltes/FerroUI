//! Port of `Diagnostics/ViewModels/LogicalTreeNode.cs`.

use super::{TreeNode, TreeNodeCollection, TreeNodeCollectionImpl, TreeNodeKind};
use crate::diagnostics::controls::TopLevelGroup;
use crate::diagnostics::views::MainWindow;
use ferroui_base::collections::{FerroList, FerroListExtensions};
use ferroui_base::reactive::{CompositeDisposable, Disposable, IDisposable};
use ferroui_base::{FerroObject, Nullable, Ref, StyledElement};
use ferroui_controls::TopLevel;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node of the logical tree.
///
/// The original derives the node class from [`TreeNode`]; here a logical
/// tree node is a [`TreeNode`] of the kind [`TreeNodeKind::Logical`], made
/// by the functions of this type.
pub struct LogicalTreeNode;

impl LogicalTreeNode {
    /// `LogicalTreeNode(FerroObject ferroObject, TreeNode? parent)`: the
    /// node of an object of the logical tree, whose children are the logical
    /// children of the element, or the top levels of a group of top levels.
    pub fn new(ferro_object: Ref<FerroObject>, parent: Option<&Rc<TreeNode>>) -> Rc<TreeNode> {
        let logical = ferro_object.cast::<StyledElement>();
        let host = ferro_object.cast::<TopLevelGroup>();
        TreeNode::new_node(ferro_object, parent, None, TreeNodeKind::Logical, move |this| {
            if let Some(logical) = logical {
                LogicalTreeNodeCollection::new(this, logical)
            } else if let Some(host) = host {
                TopLevelGroupHostLogical::new(this, host)
            } else {
                TreeNodeCollection::empty()
            }
        })
    }

    /// `Create(object control)`: the root node of the logical tree of
    /// `control`, or no node when it is not an object of the framework.
    pub fn create(control: impl Into<Nullable<FerroObject>>) -> Vec<Rc<TreeNode>> {
        match control.into().0 {
            Some(logical) => vec![LogicalTreeNode::new(logical, None)],
            None => Vec::new(),
        }
    }
}

/// `LogicalTreeNode.LogicalTreeNodeCollection`: the nodes of the logical
/// children of an element, following them.
pub struct LogicalTreeNodeCollection {
    control: Ref<StyledElement>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl LogicalTreeNodeCollection {
    pub fn new(owner: Weak<TreeNode>, control: Ref<StyledElement>) -> Rc<TreeNodeCollection> {
        TreeNodeCollection::new(owner, Box::new(Self { control, subscription: RefCell::new(None) }))
    }
}

impl TreeNodeCollectionImpl for LogicalTreeNodeCollection {
    fn dispose(&self, this: &TreeNodeCollection) {
        this.base_dispose();
        let subscription = self.subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>) {
        let owner = this.owner();
        let owner = owner.as_ref().map(Rc::downgrade).unwrap_or_default();
        let (added, removed, reset) = (nodes.clone(), nodes.clone(), nodes.clone());
        let subscription = FerroListExtensions::for_each_item_indexed(
            self.control.logical_children(),
            move |i, item: &Ref<StyledElement>| {
                let owner = owner.upgrade();
                added.insert(i, LogicalTreeNode::new(item.clone().upcast(), owner.as_ref()));
            },
            move |i, _| removed.remove_at(i),
            move || reset.clear(),
            false,
        );
        *self.subscription.borrow_mut() = Some(subscription);
    }
}

/// `LogicalTreeNode.TopLevelGroupHostLogical`: the nodes of the top levels
/// of a group, except the window of the developer tools.
pub struct TopLevelGroupHostLogical {
    group: Ref<TopLevelGroup>,
    subscriptions: CompositeDisposable,
    /// Makes the node of a top level: a node of the logical tree, or of the combined tree (an
    /// addition of the port).
    factory: fn(Ref<FerroObject>, Option<&Rc<TreeNode>>) -> Rc<TreeNode>,
}

impl TopLevelGroupHostLogical {
    pub fn new(owner: Weak<TreeNode>, host: Ref<TopLevelGroup>) -> Rc<TreeNodeCollection> {
        Self::new_with(owner, host, LogicalTreeNode::new)
    }

    /// The nodes of the top levels of a group, each made by `factory`.
    pub fn new_with(
        owner: Weak<TreeNode>,
        host: Ref<TopLevelGroup>,
        factory: fn(Ref<FerroObject>, Option<&Rc<TreeNode>>) -> Rc<TreeNode>,
    ) -> Rc<TreeNodeCollection> {
        TreeNodeCollection::new(
            owner,
            Box::new(Self { group: host, subscriptions: CompositeDisposable::with_capacity(1), factory }),
        )
    }
}

impl TreeNodeCollectionImpl for TopLevelGroupHostLogical {
    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>) {
        let owner = this.owner();
        let items = self.group.items();
        for window in items.iter() {
            if window.is::<MainWindow>() {
                continue;
            }
            nodes.add((self.factory)(window.clone().upcast(), owner.as_ref()));
        }

        let owner = owner.as_ref().map(Rc::downgrade).unwrap_or_default();
        let factory = self.factory;

        let group_on_added = {
            let nodes = nodes.clone();
            let owner = owner.clone();
            move |e: &Ref<TopLevel>| {
                if e.is::<MainWindow>() {
                    return;
                }

                nodes.add(factory(e.clone().upcast(), owner.upgrade().as_ref()));
            }
        };
        let group_on_removed = {
            let nodes = nodes.clone();
            move |e: &Ref<TopLevel>| {
                if e.is::<MainWindow>() {
                    return;
                }

                // As in the original, a removed top level adds a node.
                nodes.add(factory(e.clone().upcast(), owner.upgrade().as_ref()));
            }
        };

        let added = self.group.added(group_on_added);
        let removed = self.group.removed(group_on_removed);

        self.subscriptions.add(Disposable::create(move || {
            added.dispose();
            removed.dispose();
        }));
    }

    fn dispose(&self, this: &TreeNodeCollection) {
        self.subscriptions.dispose();
        this.base_dispose();
    }
}
