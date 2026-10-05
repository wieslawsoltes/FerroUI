use crate::{Ref, StyledElement};

/// Holds the event arguments for the `attached_to_logical_tree` and
/// `detached_from_logical_tree` events.
/// The args are immutable: copies compare equal when they describe the same
/// attachment (the same elements).
#[derive(Clone, PartialEq)]
pub struct LogicalTreeAttachmentEventArgs {
    root: Ref<StyledElement>,
    source: Ref<StyledElement>,
    parent: Option<Ref<StyledElement>>,
}

impl LogicalTreeAttachmentEventArgs {
    pub fn new(root: Ref<StyledElement>, source: Ref<StyledElement>, parent: Option<Ref<StyledElement>>) -> Self {
        Self { root, source, parent }
    }

    /// The root of the logical tree that the control is being attached to or
    /// detached from.
    pub fn root(&self) -> &Ref<StyledElement> {
        &self.root
    }

    /// The control that was attached or detached from the logical tree.
    ///
    /// Logical tree attachment events travel down the tree, so this is the
    /// control at the top of the subtree that was attached or detached.
    pub fn source(&self) -> &Ref<StyledElement> {
        &self.source
    }

    /// The control that the source is being attached to or detached from.
    /// For a detachment this is the old parent; `None` when the source is a
    /// logical root.
    pub fn parent(&self) -> Option<&Ref<StyledElement>> {
        self.parent.as_ref()
    }
}
