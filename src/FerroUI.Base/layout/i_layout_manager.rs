use super::{IBringIntoViewLayoutManager, Layoutable};
use std::rc::Rc;

/// Manages measuring and arranging of controls.
pub trait ILayoutManager {
    /// Subscribes to the event raised when the layout manager completes a
    /// layout pass. Returns a token for
    /// [`remove_layout_updated`](Self::remove_layout_updated).
    fn add_layout_updated(&self, handler: Rc<dyn Fn()>) -> u64;

    /// Unsubscribes a layout updated handler.
    fn remove_layout_updated(&self, token: u64);

    /// Notifies the layout manager that a control requires a measure.
    fn invalidate_measure(&self, control: &Layoutable);

    /// Notifies the layout manager that a control requires an arrange.
    fn invalidate_arrange(&self, control: &Layoutable);

    /// Executes a layout pass.
    ///
    /// You should not usually need to call this method explicitly: the layout
    /// manager schedules layout passes itself.
    fn execute_layout_pass(&self);

    /// Executes the initial layout pass on a layout root.
    ///
    /// You should not usually need to call this method explicitly: it is
    /// called by the layout root when it is first shown.
    fn execute_initial_layout_pass(&self);

    /// Registers a control as wanting to receive effective viewport
    /// notifications.
    fn register_effective_viewport_listener(&self, control: &Layoutable);

    /// Unregisters a control from receiving effective viewport notifications.
    fn unregister_effective_viewport_listener(&self, control: &Layoutable);

    /// Releases the layout manager.
    fn dispose(&self);

    /// The layout manager viewed as one that handles bring-into-view
    /// requests, if it does.
    fn as_bring_into_view_layout_manager(&self) -> Option<&dyn IBringIntoViewLayoutManager> {
        None
    }
}
