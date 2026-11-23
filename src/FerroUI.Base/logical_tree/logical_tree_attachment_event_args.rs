use crate::controls::ResourcesChangedEventArgs;
use crate::{Ref, StyledElement};

/// Holds the event arguments for the `attached_to_logical_tree` and
/// `detached_from_logical_tree` events.
/// The args are immutable: copies compare equal when they describe the same
/// attachment (the same elements).
#[derive(Clone)]
pub struct LogicalTreeAttachmentEventArgs {
    root: Ref<StyledElement>,
    source: Ref<StyledElement>,
    parent: Option<Ref<StyledElement>>,
    /// The notification of the resources the attachment makes visible, which
    /// every element of the attached subtree receives.
    resources_changed: ResourcesChangedEventArgs,
}

impl PartialEq for LogicalTreeAttachmentEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.source == other.source && self.parent == other.parent
    }
}

impl LogicalTreeAttachmentEventArgs {
    pub fn new(root: Ref<StyledElement>, source: Ref<StyledElement>, parent: Option<Ref<StyledElement>>) -> Self {
        Self::with_resources_changed(root, source, parent, ResourcesChangedEventArgs::create())
    }

    /// The args of an attachment whose change of the visible resources is
    /// `resources_changed`, made when the source got its new parent.
    pub(crate) fn with_resources_changed(
        root: Ref<StyledElement>,
        source: Ref<StyledElement>,
        parent: Option<Ref<StyledElement>>,
        resources_changed: ResourcesChangedEventArgs,
    ) -> Self {
        Self { root, source, parent, resources_changed }
    }

    /// The notification of the resources the attachment makes visible.
    pub(crate) fn resources_changed(&self) -> ResourcesChangedEventArgs {
        self.resources_changed
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
