/// Describes how focus should be moved by directional or tab keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyboardNavigationMode {
    /// Items in the container will be cycled through, and focus will be
    /// moved to the previous/next container after the first/last control in
    /// the container.
    #[default]
    Continue,
    /// Items in the container will be cycled through, and moving past the
    /// first or last control in the container will cause the last/first
    /// control to be focused.
    Cycle,
    /// Items in the container will be cycled through and focus will stop
    /// moving when the edge of the container is reached.
    Contained,
    /// When focus is moved into the container, the control described by the
    /// `TabOnceActiveElement` attached property on the container will be
    /// focused. When focus moves away from this control, focus will move to
    /// the previous/next container.
    Once,
    /// The container's children will not be focused when using the tab key.
    None,
    /// TabIndexes are considered on local subtree only inside this container.
    Local,
}
