use super::Layoutable;
use crate::Ref;

/// A bring-into-view request, processed by the
/// [`IBringIntoViewLayoutManager`](super::IBringIntoViewLayoutManager) at the
/// end of a layout pass.
pub trait BringIntoViewRequest {
    /// Gets the control this request is associated with.
    fn target(&self) -> Ref<Layoutable>;

    /// Attempts to execute the request.
    ///
    /// Returns true if the request has been executed (or abandoned) and
    /// should be removed from the queue; false if the target isn't ready yet
    /// and the request should be retained for a following layout pass.
    fn try_execute(&self) -> bool;
}
