//! Port of `Diagnostics/ViewModels/EventTreeNodeBase.cs`.

use super::{EventOwnerTreeNode, EventTreeNode, ViewModelBase};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, ModelTypes};
use ferroui_base::ferro_markup_type;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

/// What the deriving class of a node adds to it: the owner of a group of
/// routed events ([`EventOwnerTreeNode`]) or one routed event
/// ([`EventTreeNode`]).
///
/// Upstream the node is an abstract class with these two deriving classes,
/// whose `IsEnabled` overrides the virtual property of the base. In the port
/// a node is one shared type, [`EventTreeNodeBase`], that carries the part of
/// its deriving class: the tree of the events page holds one model type
/// (its cell template is typed with the base class), and the type tests of
/// the original (`node is EventTreeNode eventNode`) are the hooks
/// [`EventTreeNodeBase::as_event_tree_node`] and
/// [`EventTreeNodeBase::as_event_owner_tree_node`].
pub enum EventTreeNodeKind {
    /// A node of an [`EventOwnerTreeNode`].
    Owner(EventOwnerTreeNode),
    /// A node of an [`EventTreeNode`].
    Event(EventTreeNode),
}

/// A node of the tree of routed events of the events page: the owner of a
/// group of events or one event, with the check box that enables the
/// recording of the events below it.
pub struct EventTreeNodeBase {
    base: ViewModelBase,
    this: Weak<EventTreeNodeBase>,
    pub(crate) update_children: Cell<bool>,
    pub(crate) update_parent: Cell<bool>,
    is_expanded: Cell<bool>,
    is_enabled: Cell<Option<bool>>,
    is_visible: Cell<bool>,
    parent: Option<Weak<EventTreeNodeBase>>,
    text: String,
    children: RefCell<Option<FerroList<Rc<EventTreeNodeBase>>>>,
    // Deviation (DEVIATIONS.md, Developer tools): the children the TreeDataGrid of the events
    // page shows (the visible ones, in their order), made the first time they are asked for.
    visible_children: OnceCell<Rc<FerroList<Rc<EventTreeNodeBase>>>>,
    kind: EventTreeNodeKind,
}

/// Reference equality, as the class of the original.
impl PartialEq for EventTreeNodeBase {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for EventTreeNodeBase {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl EventTreeNodeBase {
    /// C# `protected EventTreeNodeBase(EventTreeNodeBase? parent, string text)`, with the part
    /// of the deriving class, which `kind` creates from the weak reference to the new node.
    pub(crate) fn new(
        parent: Option<&Rc<EventTreeNodeBase>>,
        text: String,
        kind: impl FnOnce(&Weak<EventTreeNodeBase>) -> EventTreeNodeKind,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|this| Self {
            base: ViewModelBase::new(),
            this: this.clone(),
            update_children: Cell::new(true),
            update_parent: Cell::new(true),
            is_expanded: Cell::new(false),
            is_enabled: Cell::new(Some(false)),
            is_visible: Cell::new(false),
            parent: parent.map(Rc::downgrade),
            text,
            children: RefCell::new(None),
            visible_children: OnceCell::new(),
            kind: kind(this),
        });
        this.set_is_visible(true);
        this
    }

    /// The shared handle of this node.
    pub(crate) fn to_rc(&self) -> Rc<EventTreeNodeBase> {
        self.this.upgrade().expect("a node is reached through its shared handle")
    }

    /// The part of the deriving class.
    pub fn kind(&self) -> &EventTreeNodeKind {
        &self.kind
    }

    /// The node as an [`EventTreeNode`] (C# `node as EventTreeNode`).
    pub fn as_event_tree_node(&self) -> Option<&EventTreeNode> {
        match &self.kind {
            EventTreeNodeKind::Event(node) => Some(node),
            EventTreeNodeKind::Owner(_) => None,
        }
    }

    /// The node as an [`EventOwnerTreeNode`] (C# `node as EventOwnerTreeNode`).
    pub fn as_event_owner_tree_node(&self) -> Option<&EventOwnerTreeNode> {
        match &self.kind {
            EventTreeNodeKind::Owner(node) => Some(node),
            EventTreeNodeKind::Event(_) => None,
        }
    }

    /// The child nodes; `None` for a node without children.
    pub fn children(&self) -> Option<FerroList<Rc<EventTreeNodeBase>>> {
        self.children.borrow().clone()
    }

    /// The visible children, in their order: the child rows of the node in the TreeDataGrid of the
    /// events page, which follow the filter of the page (`sync_visible_nodes`); `None` for a node
    /// without children. The same list every time, so that the rows of the children are kept
    /// while the node notifies other changes.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): upstream's tree view hides the items of the
    /// nodes that are not visible; the rows of a TreeDataGrid are the nodes of its source.
    pub fn visible_children(&self) -> Option<Rc<FerroList<Rc<EventTreeNodeBase>>>> {
        let children = self.children()?;
        let visible = self.visible_children.get_or_init(|| {
            Rc::new(FerroList::from_items(children.to_vec().into_iter().filter(|child| child.is_visible())))
        });
        Some(visible.clone())
    }

    /// Updates the visible children (once they were asked for) after a change of the visibility
    /// of the children.
    pub(crate) fn update_visible_children(&self) {
        if let (Some(children), Some(visible)) = (self.children(), self.visible_children.get()) {
            sync_visible_nodes(&children.to_vec(), visible);
        }
    }

    /// States what the TreeDataGrid of the events page observes of the nodes: their property
    /// change notifications (the expansion of a node). Once per thread.
    pub(crate) fn register_model() {
        if !ModelTypes::is_registered(std::any::TypeId::of::<EventTreeNodeBase>()) {
            ModelTypes::register::<EventTreeNodeBase>(|builder| builder.notify_property_changed());
        }
    }

    /// The protected setter of `Children`.
    pub(crate) fn set_children(&self, value: Option<FerroList<Rc<EventTreeNodeBase>>>) {
        *self.children.borrow_mut() = value;
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded");
    }

    /// Whether the events of the node are recorded: `None` when some of the
    /// events below the node are and some are not.
    pub fn is_enabled(&self) -> Option<bool> {
        self.is_enabled.get()
    }

    /// The virtual setter of `IsEnabled`: the override of the deriving class
    /// ([`EventOwnerTreeNode`] or [`EventTreeNode`]).
    pub fn set_is_enabled(&self, value: Option<bool>) {
        match &self.kind {
            EventTreeNodeKind::Owner(_) => EventOwnerTreeNode::set_is_enabled(self, value),
            EventTreeNodeKind::Event(_) => EventTreeNode::set_is_enabled(self, value),
        }
    }

    /// The base implementation of the setter of `IsEnabled` (C# `base.IsEnabled = value`).
    pub(crate) fn base_set_is_enabled(&self, value: Option<bool>) {
        self.base.raise_and_set_if_changed_cell(&self.is_enabled, value, "IsEnabled");
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_visible, value, "IsVisible");
    }

    pub fn parent(&self) -> Option<Rc<EventTreeNodeBase>> {
        self.parent.as_ref().and_then(Weak::upgrade)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets `IsEnabled` from the children: their common value, or `None`
    /// when they differ.
    pub(crate) fn update_checked(&self) {
        let get_value = || -> Option<bool> {
            let Some(children) = self.children() else {
                return Some(false);
            };

            let mut value = Some(false);

            for i in 0..children.count() {
                if i == 0 {
                    value = children.get(i).is_enabled();
                    continue;
                }

                if value != children.get(i).is_enabled() {
                    value = None;
                    break;
                }
            }

            value
        };

        self.set_is_enabled(get_value());
    }
}

/// Makes `visible` the nodes of `all` that are visible, in the order of `all`: inserts the nodes
/// that became visible and removes the ones that were hidden, so that the rows of the others are
/// kept. `visible` holds nodes of `all` only, in their order.
///
/// Deviation (DEVIATIONS.md, Developer tools): the rows of the TreeDataGrid of the events page.
pub(crate) fn sync_visible_nodes(all: &[Rc<EventTreeNodeBase>], visible: &FerroList<Rc<EventTreeNodeBase>>) {
    let mut index = 0;
    for node in all {
        let shown = index < visible.count() && Rc::ptr_eq(&visible.get(index), node);
        if node.is_visible() {
            if !shown {
                visible.insert(index, node.clone());
            }
            index += 1;
        } else if shown {
            visible.remove_at(index);
        }
    }
}

// The list of the child nodes (the items of a tree view upstream; the TreeDataGrid of the events
// page shows the visible children, `visible_children`).
ferroui_controls::ferro_markup_list!(pub EventTreeNodeList: Rc<EventTreeNodeBase>);

ferro_markup_type!(class EventTreeNodeBase {
    this: Rc<EventTreeNodeBase>,
    handles: [EventTreeNodeBase, Rc<EventTreeNodeBase>, Option<Rc<EventTreeNodeBase>>],
    base: ViewModelBase,
    properties: [
        Children: Option<FerroList<Rc<EventTreeNodeBase>>> { get: EventTreeNodeBase::children },
        IsExpanded: bool { get: EventTreeNodeBase::is_expanded, set: EventTreeNodeBase::set_is_expanded },
        IsEnabled: Option<bool> { get: EventTreeNodeBase::is_enabled, set: EventTreeNodeBase::set_is_enabled },
        IsVisible: bool { get: EventTreeNodeBase::is_visible, set: EventTreeNodeBase::set_is_visible },
        Parent: Option<Rc<EventTreeNodeBase>> { get: EventTreeNodeBase::parent },
        Text: String { get: |this: &Rc<EventTreeNodeBase>| this.text().to_string() },
    ],
    notify_property_changed: EventTreeNodeBase,
});
