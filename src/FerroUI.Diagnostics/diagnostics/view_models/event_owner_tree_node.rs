//! Port of `Diagnostics/ViewModels/EventOwnerTreeNode.cs`.

use super::{EventTreeNode, EventTreeNodeBase, EventTreeNodeKind, EventsPageViewModel};
use ferroui_base::collections::FerroList;
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::TypeInfo;
use std::rc::{Rc, Weak};

/// The node of the type that owns a group of routed events: its children
/// are the nodes of the events, and enabling or disabling it enables or
/// disables all of them.
///
/// The part of the deriving class of a node (see
/// [`EventTreeNodeKind`]); the class adds no state of its own.
pub struct EventOwnerTreeNode {
    _private: (),
}

impl EventOwnerTreeNode {
    /// C# `new EventOwnerTreeNode(Type type, IEnumerable<RoutedEvent> events, EventsPageViewModel vm)`:
    /// the node of `type_` with a child for each of `events`, ordered by
    /// name, expanded.
    pub fn new(
        type_: &'static TypeInfo,
        events: impl IntoIterator<Item = RoutedEvent>,
        vm: &Weak<EventsPageViewModel>,
    ) -> Rc<EventTreeNodeBase> {
        let this = EventTreeNodeBase::new(None, type_.name().to_string(), |_| {
            EventTreeNodeKind::Owner(EventOwnerTreeNode { _private: () })
        });

        let mut events: Vec<RoutedEvent> = events.into_iter().collect();
        // `OrderBy` is a stable sort.
        events.sort_by(|x, y| x.name().cmp(y.name()));
        this.set_children(Some(FerroList::from_items(
            events.into_iter().map(|e| EventTreeNode::new(&this, e, vm.clone())),
        )));
        this.set_is_expanded(true);
        this
    }

    /// The override of the setter of `IsEnabled`: enables or disables every
    /// child with the node, unless the change comes from a child.
    pub(crate) fn set_is_enabled(this: &EventTreeNodeBase, value: Option<bool>) {
        if this.is_enabled() != value {
            this.base_set_is_enabled(value);

            if this.update_children.get() && value.is_some() {
                let children = this.children().expect("the children of an owner node are set when it is created");
                for child in children.to_vec() {
                    // `try { .. } finally { .. }` upstream.
                    child.update_parent.set(false);
                    child.set_is_enabled(value);
                    child.update_parent.set(true);
                }
            }
        }
    }
}
