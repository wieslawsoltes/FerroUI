use super::ChildIndexChangedEventArgs;
use crate::reactive::IDisposable;
use crate::StyledElement;
use std::rc::Rc;

/// Child's index and total count information provider used by list controls
/// and panels. Used by the nth-child and nth-last-child selectors.
pub trait IChildIndexProvider {
    /// Gets the index of `child` in its container, or -1 if it is not a
    /// child.
    fn get_child_index(&self, child: &StyledElement) -> i32;

    /// The total number of children, if it can be determined.
    fn try_get_total_count(&self) -> Option<i32>;

    /// Raised when the index of one or more children changes. Disposing the
    /// returned handle unsubscribes.
    fn child_index_changed(&self, handler: Rc<dyn Fn(&ChildIndexChangedEventArgs)>) -> Rc<dyn IDisposable>;
}
