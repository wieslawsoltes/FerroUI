use std::sync::Arc;
use std::time::Duration;

/// The callback of an [`IRenderTimer`]. The argument is the timestamp of the
/// tick on the timer's monotonic clock.
pub type RenderTimerTick = Arc<dyn Fn(Duration) + Send + Sync>;

/// Defines the interface implemented by an application render timer.
pub trait IRenderTimer: Send + Sync {
    /// Gets the callback to be invoked when the timer ticks.
    fn tick(&self) -> Option<RenderTimerTick>;

    /// Sets the callback to be invoked when the timer ticks.
    ///
    /// This can be called from any thread, but it's guaranteed that it's not
    /// called concurrently (i. e. the render loop always does it under a
    /// lock). Setting the value to `None` suggests the timer to stop ticking,
    /// however the timer is allowed to produce ticks on the previously set
    /// value as long as it stops doing so.
    ///
    /// The callback can be invoked on any thread. Implementations must not
    /// invoke it synchronously from this method: the render loop calls it
    /// while holding a lock that the callback takes.
    fn set_tick(&self, value: Option<RenderTimerTick>);

    /// Indicates if the timer ticks on a non-UI thread.
    fn runs_in_background(&self) -> bool;

    /// Asks for a tick out of turn: as soon as the thread that ticks can
    /// produce one, without waiting for the next period of the timer (the
    /// next frame of a display). Not from upstream.
    ///
    /// Called by a thread that is about to block until a frame has been
    /// rendered. A timer whose next tick may be far away, or may depend on
    /// the caller returning to its event loop, invokes the callback it was
    /// given once on the thread it ticks on; it must not invoke it on the
    /// calling thread, and does nothing while no callback is set. The
    /// default does nothing: the caller then waits for the next tick.
    fn request_tick_out_of_turn(&self) {}
}
