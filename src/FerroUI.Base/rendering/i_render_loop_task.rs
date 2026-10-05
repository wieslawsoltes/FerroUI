/// A task polled by the render loop on each tick.
///
/// Tasks are shared with the render timer's thread, so they must be
/// thread-safe.
pub trait IRenderLoopTask: Send + Sync {
    /// Renders the task. Returns `true` when the task wants the next tick of
    /// the render loop.
    fn render(&self) -> bool;
}
