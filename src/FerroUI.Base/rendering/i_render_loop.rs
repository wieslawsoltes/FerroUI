use std::sync::Arc;

use super::IRenderLoopTask;

/// The application render loop.
///
/// The render loop is responsible for advancing the animation timer and
/// updating the scene graph for visible windows.
pub trait IRenderLoop: Send + Sync {
    /// Adds an update task.
    ///
    /// Registered update tasks will be polled on each tick of the render loop
    /// after the animation timer has been pulsed.
    fn add(&self, i: Arc<dyn IRenderLoopTask>);

    /// Removes an update task.
    fn remove(&self, i: &Arc<dyn IRenderLoopTask>);

    /// Indicates if the rendering is done on a non-UI thread.
    fn runs_in_background(&self) -> bool;

    /// Wakes up the render loop to schedule the next tick.
    /// Thread-safe: can be called from any thread.
    fn wakeup(&self);
}
