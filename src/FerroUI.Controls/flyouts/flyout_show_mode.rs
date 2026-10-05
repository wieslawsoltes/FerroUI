// Note: an automatic show mode was removed from the reference: there the
// show mode is determined automatically based on the method used to show the
// flyout, and flyouts generally open with the "Standard" behavior.

/// Defines how a flyout behaves when it is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlyoutShowMode {
    /// Behavior is typical of a flyout shown reactively, like a context
    /// menu. The open flyout takes focus. A command bar flyout opens in its
    /// expanded state.
    #[default]
    Standard = 0,

    /// Behavior is typical of a flyout shown proactively. The open flyout
    /// does not take focus.
    Transient = 1,

    /// The flyout exhibits transient behavior while the cursor is close to
    /// it, but is dismissed when the cursor moves away.
    TransientWithDismissOnPointerMoveAway = 2,
}
