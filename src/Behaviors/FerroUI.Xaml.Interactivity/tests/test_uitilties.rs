//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/TestUitilties.cs` of Xaml.Behaviors.

use super::stub_behavior::StubBehavior;
use super::support::panics;
use ferroui_base::{FerroObject, Ref};

/// `TestUtilities`.
pub(crate) struct TestUtilities;

impl TestUtilities {
    /// Handles the difference between invalid operations in managed and native code: the invalid
    /// operation is a panic.
    pub(crate) fn assert_throws_invalid_operation_exception(action: impl FnOnce()) {
        assert!(panics(action), "an invalid operation was expected");
    }

    #[allow(dead_code)]
    pub(crate) fn assert_throws_argument_exception(action: impl FnOnce()) {
        assert!(panics(action), "an argument error was expected");
    }

    pub(crate) fn assert_detached(behavior: &StubBehavior) {
        assert_eq!(1, behavior.detach_count(), "The Behavior should be detached.");
        assert!(behavior.associated_object().is_none(), "A Detached Behavior should have a null AssociatedObject.");
    }

    pub(crate) fn assert_not_detached(behavior: &StubBehavior) {
        assert_eq!(0, behavior.detach_count(), "The Behavior should not be detached.");
    }

    pub(crate) fn assert_attached(behavior: &StubBehavior, associated_object: Option<Ref<FerroObject>>) {
        assert_eq!(1, behavior.attach_count(), "The behavior should be attached.");
        assert_eq!(
            associated_object,
            behavior.associated_object(),
            "The AssociatedObject of the Behavior should be what it was attached to."
        );
    }

    pub(crate) fn assert_not_attached(behavior: &StubBehavior) {
        assert_eq!(0, behavior.attach_count(), "The behavior should not be attached.");
        assert!(behavior.associated_object().is_none(), "The AssociatedObject should be null for a non-attached Behavior.");
    }
}
