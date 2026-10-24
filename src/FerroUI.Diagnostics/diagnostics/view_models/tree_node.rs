//! Port of `Diagnostics/ViewModels/TreeNode.cs`.

use super::{TreeNodeCollection, ViewModelBase};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, ModelTypes};
use ferroui_base::media::FontWeight;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{ferro_markup_type, FerroObject, Ref, StyledElement};
use ferroui_controls::primitives::{OverlayPopupHost, PopupRoot};
use ferroui_controls::{ContextMenu, TopLevel};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The class of a [`TreeNode`]: the abstract node of the original is
/// derived by [`LogicalTreeNode`](super::LogicalTreeNode) and
/// [`VisualTreeNode`](super::VisualTreeNode), whose constructors make nodes
/// of this type with the members they add.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeNodeKind {
    /// A node of the logical tree (`LogicalTreeNode`).
    Logical,
    /// A node of the visual tree (`VisualTreeNode`), with its `IsInTemplate`.
    Visual { is_in_template: bool },
    /// A node of the combined tree (an addition of the port, `combined_tree_node.rs`): an element
    /// of the logical tree, or a part of a template (`visual_only`), shown greyed out.
    Combined { visual_only: bool },
}

/// A node of a tree of the developer tools: an object of the inspected
/// application, its type, name and classes, whether it is expanded in the
/// tree view, and its children.
pub struct TreeNode {
    base: ViewModelBase,
    classes_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    classes: RefCell<String>,
    is_expanded: Cell<bool>,
    font_weight: FontWeight,
    children: Rc<TreeNodeCollection>,
    element_name: Option<String>,
    visual: Ref<FerroObject>,
    parent: Weak<TreeNode>,
    type_: String,
    kind: TreeNodeKind,
}

impl PartialEq for TreeNode {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TreeNode {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl TreeNode {
    /// `protected TreeNode(FerroObject ferroObject, TreeNode? parent, string? customTypeName = null)`
    /// and the constructor of the deriving class: `kind` is the class of the
    /// node and `children` creates its `Children`, the collection of the
    /// children of the node it is given.
    pub(crate) fn new_node(
        ferro_object: Ref<FerroObject>,
        parent: Option<&Rc<TreeNode>>,
        custom_type_name: Option<String>,
        kind: TreeNodeKind,
        children: impl FnOnce(Weak<TreeNode>) -> Rc<TreeNodeCollection>,
    ) -> Rc<TreeNode> {
        let type_ = custom_type_name.unwrap_or_else(|| ferro_object.get_type().name().to_string());
        let font_weight = if Self::is_root(&ferro_object) { FontWeight::Bold } else { FontWeight::Normal };
        let element_name = ferro_object.cast::<StyledElement>().and_then(|named| named.name());
        let parent = parent.map_or_else(Weak::new, Rc::downgrade);

        let this = Rc::new_cyclic(|this: &Weak<TreeNode>| TreeNode {
            base: ViewModelBase::new(),
            classes_subscription: RefCell::new(None),
            classes: RefCell::new(String::new()),
            is_expanded: Cell::new(false),
            font_weight,
            children: children(this.clone()),
            element_name,
            visual: ferro_object,
            parent,
            type_,
            kind,
        });

        if let Some(element) = this.visual.cast::<StyledElement>() {
            this.subscribe_to_classes(&element);
        }

        this
    }

    /// The subscription of the constructor to the classes of the element:
    /// `Classes` is updated now (`StartWith(null)`) and whenever the classes
    /// change. The original subscribes weakly; here the classes hold a
    /// handler that reaches the node and the element through weak
    /// references, and disposing the node removes it.
    fn subscribe_to_classes(self: &Rc<Self>, element: &Ref<StyledElement>) {
        let classes = element.classes();
        let node = Rc::downgrade(self);
        let weak_element = element.downgrade();
        let update: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(node), Some(element)) = (node.upgrade(), weak_element.upgrade()) else {
                return;
            };
            let classes = element.classes();
            if classes.count() > 0 {
                node.set_classes(format!("({})", classes.snapshot().join(" ")));
            } else {
                node.set_classes(String::new());
            }
        });

        update();

        let token = classes.add_collection_changed(Rc::new(move |_: &NotifyCollectionChangedEventArgs<'_, String>| update()));
        *self.classes_subscription.borrow_mut() = Some(Disposable::create(move || {
            classes.remove_collection_changed(token);
        }));
    }

    /// `IsRoot`: whether the object is the root of a tree of its own (a top
    /// level, a context menu or the host of a popup).
    fn is_root(visual: &Ref<FerroObject>) -> bool {
        visual.is::<TopLevel>()
            || visual.is::<ContextMenu>()
            || visual.is::<PopupRoot>()
            || visual.is::<OverlayPopupHost>()
    }

    /// The weight of the font of the node in the tree: bold for a root.
    pub fn font_weight(&self) -> FontWeight {
        self.font_weight
    }

    /// The children of the node.
    pub fn children(&self) -> Rc<TreeNodeCollection> {
        self.children.clone()
    }

    /// The classes of the element, in parentheses, or empty when it has
    /// none.
    pub fn classes(&self) -> String {
        self.classes.borrow().clone()
    }

    fn set_classes(&self, value: String) {
        self.base.raise_and_set_if_changed(&self.classes, value, "Classes");
    }

    /// The name of the element.
    pub fn element_name(&self) -> Option<String> {
        self.element_name.clone()
    }

    /// The object of the node.
    pub fn visual(&self) -> Ref<FerroObject> {
        self.visual.clone()
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded");
    }

    /// The parent node; `None` for a root, and once the parent is gone (the
    /// nodes hold their parents weakly).
    pub fn parent(&self) -> Option<Rc<TreeNode>> {
        self.parent.upgrade()
    }

    /// The `Type` property: the name of the type of the object, or the name
    /// the node was created with.
    pub fn type_(&self) -> String {
        self.type_.clone()
    }

    /// The class of the node: [`LogicalTreeNode`](super::LogicalTreeNode) or
    /// [`VisualTreeNode`](super::VisualTreeNode).
    pub fn kind(&self) -> TreeNodeKind {
        self.kind
    }

    /// `VisualTreeNode.IsInTemplate`: whether the element is part of a
    /// template (it has a templated parent). `false` for a node of the
    /// logical tree.
    pub fn is_in_template(&self) -> bool {
        matches!(self.kind, TreeNodeKind::Visual { is_in_template: true } | TreeNodeKind::Combined { visual_only: true })
    }

    /// Whether the node is a part of a template in the combined tree, which the tree shows
    /// greyed out (an addition of the port).
    pub fn is_visual_only(&self) -> bool {
        matches!(self.kind, TreeNodeKind::Combined { visual_only: true })
    }

    /// States what the TreeDataGrid of a tree page observes of the nodes: their property change
    /// notifications (the expansion of a node, its classes). Once per thread.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): the tree pages show the nodes in a TreeDataGrid,
    /// not in a tree view.
    pub(crate) fn register_model() {
        if !ModelTypes::is_registered(std::any::TypeId::of::<TreeNode>()) {
            ModelTypes::register::<TreeNode>(|builder| builder.notify_property_changed());
        }
    }

    /// `Dispose()`: ends the subscription to the classes and disposes the
    /// children.
    pub fn dispose(&self) {
        let classes_subscription = self.classes_subscription.borrow().clone();
        if let Some(classes_subscription) = classes_subscription {
            classes_subscription.dispose();
        }
        self.children.dispose();
    }
}

ferro_markup_type!(class TreeNode {
    this: Rc<TreeNode>,
    handles: [TreeNode, Rc<TreeNode>, Option<Rc<TreeNode>>],
    properties: [
        FontWeight: FontWeight { get: TreeNode::font_weight },
        Children: Rc<TreeNodeCollection> { get: TreeNode::children },
        Classes: String { get: TreeNode::classes },
        ElementName: Option<String> { get: TreeNode::element_name },
        Visual: Ref<FerroObject> { get: TreeNode::visual },
        IsExpanded: bool { get: TreeNode::is_expanded, set: TreeNode::set_is_expanded },
        Parent: Option<Rc<TreeNode>> { get: TreeNode::parent },
        Type: String { get: TreeNode::type_ },
        // Deviation (DEVIATIONS.md, Developer tools): not a member of the base class upstream (it
        // is `VisualTreeNode.IsInTemplate`); the cells of the tree pages show the nodes of
        // template parts in their own style.
        IsInTemplate: bool { get: TreeNode::is_in_template },
        IsVisualOnly: bool { get: TreeNode::is_visual_only },
    ],
    notify_property_changed: TreeNode,
});
