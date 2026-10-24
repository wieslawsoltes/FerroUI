//! The combined tree of the developer tools: an addition of the port, with no counterpart upstream
//! (Deviation, DEVIATIONS.md, Developer tools).
//!
//! The nodes are those of the logical tree, and below each element come first the parts of its
//! template, shown greyed out: the visual children of the element that are not logical children
//! (a visual is a logical child when its logical parent lists it among its logical children; the
//! root of a template has the templated control as its logical parent but is not among its
//! children), each with the visual children below it that are not logical children either. The
//! logical children follow, in the order of the logical tree. A logical child that the template
//! shows (the content of a button, in a content presenter of its template) is shown once, under
//! its logical parent, not again under the part of the template it is in.

use super::{TopLevelGroupHostLogical, TreeNode, TreeNodeCollection, TreeNodeCollectionImpl, TreeNodeKind};
use crate::diagnostics::controls::TopLevelGroup;
use ferroui_base::collections::{FerroList, FerroListExtensions};
use ferroui_base::reactive::{CompositeDisposable, IDisposable};
use ferroui_base::{FerroObject, Nullable, Ref, StyledElement, Visual};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node of the combined tree.
pub struct CombinedTreeNode;

impl CombinedTreeNode {
    /// The node of an element of the logical tree: its children are the parts of its template,
    /// then its logical children; or the node of the top levels of a group.
    pub fn new_logical(ferro_object: Ref<FerroObject>, parent: Option<&Rc<TreeNode>>) -> Rc<TreeNode> {
        let element = ferro_object.cast::<StyledElement>();
        let host = ferro_object.cast::<TopLevelGroup>();
        TreeNode::new_node(ferro_object, parent, None, TreeNodeKind::Combined { visual_only: false }, move |this| {
            if let Some(element) = element {
                CombinedChildren::new(this, element, true)
            } else if let Some(host) = host {
                TopLevelGroupHostLogical::new_with(this, host, CombinedTreeNode::new_logical)
            } else {
                TreeNodeCollection::empty()
            }
        })
    }

    /// The node of a part of a template: its children are its visual children that are not
    /// logical children.
    fn new_visual_only(element: Ref<StyledElement>, parent: Option<&Rc<TreeNode>>) -> Rc<TreeNode> {
        let ferro_object: Ref<FerroObject> = element.clone().upcast();
        TreeNode::new_node(ferro_object, parent, None, TreeNodeKind::Combined { visual_only: true }, move |this| {
            CombinedChildren::new(this, element, false)
        })
    }

    /// The root node of the combined tree of `control`, or no node when it is not an object of
    /// the framework.
    pub fn create(control: impl Into<Nullable<FerroObject>>) -> Vec<Rc<TreeNode>> {
        match control.into().0 {
            Some(root) => vec![CombinedTreeNode::new_logical(root, None)],
            None => Vec::new(),
        }
    }

    /// Whether `element` is a node of the logical tree: it and each of its logical ancestors are
    /// among the logical children of their logical parents, up to the root (an element without a
    /// logical parent). The root of a template has the templated control as its logical parent but
    /// is not among its children, and neither it nor the elements logically below it (the child of
    /// a border of the template) are nodes of the logical tree.
    pub fn is_logical_child(element: &Ref<StyledElement>) -> bool {
        let mut current = element.clone();
        while let Some(parent) = current.get_logical_parent() {
            if !parent.logical_children().to_vec().iter().any(|child| child.ptr_eq(&current)) {
                return false;
            }
            current = parent;
        }
        true
    }
}

/// The children of a node of the combined tree, following the visual children of its element
/// (and its logical children, for a logical node).
struct CombinedChildren {
    element: Ref<StyledElement>,
    /// Whether the node is a logical node, whose logical children follow the parts of its
    /// template.
    logical: bool,
    subscriptions: CompositeDisposable,
    /// The nodes shown now, kept so that the node of an element survives a change of its
    /// siblings (with its expansion and its children).
    current: Rc<RefCell<Vec<Rc<TreeNode>>>>,
}

impl CombinedChildren {
    fn new(owner: Weak<TreeNode>, element: Ref<StyledElement>, logical: bool) -> Rc<TreeNodeCollection> {
        TreeNodeCollection::new(
            owner,
            Box::new(Self {
                element,
                logical,
                subscriptions: CompositeDisposable::with_capacity(2),
                current: Rc::new(RefCell::new(Vec::new())),
            }),
        )
    }
}

/// Makes the nodes of `element` again: the parts of its template, then its logical children
/// when `logical`, reusing the nodes it already has for the same elements.
fn rebuild(
    element: &Ref<StyledElement>,
    logical: bool,
    owner: &Weak<TreeNode>,
    current: &RefCell<Vec<Rc<TreeNode>>>,
    nodes: &FerroList<Rc<TreeNode>>,
) {
    let owner = owner.upgrade();
    let mut wanted: Vec<(Ref<StyledElement>, bool)> = Vec::new();
    if let Some(visual) = element.cast::<Visual>() {
        for child in visual.visual_children().to_vec() {
            if let Some(child) = child.cast::<StyledElement>() {
                if !CombinedTreeNode::is_logical_child(&child) {
                    wanted.push((child, true));
                }
            }
        }
    }
    if logical {
        for child in element.logical_children().to_vec() {
            wanted.push((child, false));
        }
    }

    let previous = current.borrow().clone();
    let next: Vec<Rc<TreeNode>> = wanted
        .into_iter()
        .map(|(child, visual_only)| {
            let kind = TreeNodeKind::Combined { visual_only };
            previous
                .iter()
                .find(|node| node.kind() == kind && node.visual().ptr_eq(&child))
                .cloned()
                .unwrap_or_else(|| {
                    if visual_only {
                        CombinedTreeNode::new_visual_only(child, owner.as_ref())
                    } else {
                        CombinedTreeNode::new_logical(child.upcast(), owner.as_ref())
                    }
                })
        })
        .collect();

    let same = previous.len() == next.len() && previous.iter().zip(&next).all(|(a, b)| Rc::ptr_eq(a, b));
    if same {
        return;
    }
    for node in &previous {
        if !next.iter().any(|kept| Rc::ptr_eq(kept, node)) {
            node.dispose();
        }
    }
    nodes.clear();
    for node in &next {
        nodes.add(node.clone());
    }
    *current.borrow_mut() = next;
}

impl TreeNodeCollectionImpl for CombinedChildren {
    fn dispose(&self, this: &TreeNodeCollection) {
        self.subscriptions.dispose();
        this.base_dispose();
    }

    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>) {
        let owner = this.owner();
        let owner = owner.as_ref().map(Rc::downgrade).unwrap_or_default();

        rebuild(&self.element, self.logical, &owner, &self.current, nodes);

        // A change of the visual or the logical children makes the nodes again.
        let refresh = {
            let element = self.element.clone();
            let logical = self.logical;
            let current = self.current.clone();
            let nodes = nodes.clone();
            Rc::new(move || rebuild(&element, logical, &owner, &current, &nodes))
        };
        if let Some(visual) = self.element.cast::<Visual>() {
            let (added, removed, reset) = (refresh.clone(), refresh.clone(), refresh.clone());
            self.subscriptions.add(FerroListExtensions::for_each_item_indexed(
                visual.visual_children(),
                move |_, _: &Ref<Visual>| added(),
                move |_, _| removed(),
                move || reset(),
                false,
            ));
        }
        if self.logical {
            let (added, removed, reset) = (refresh.clone(), refresh.clone(), refresh);
            self.subscriptions.add(FerroListExtensions::for_each_item_indexed(
                self.element.logical_children(),
                move |_, _: &Ref<StyledElement>| added(),
                move |_, _| removed(),
                move || reset(),
                false,
            ));
        }
    }
}

/// The subscriptions are disposed with the collection.
impl Drop for CombinedChildren {
    fn drop(&mut self) {
        self.subscriptions.dispose();
    }
}
