/// Priority for cross-thread dispatch between UI and Wayland threads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaylandDispatchPriority {
    /// UI→worker: batched with the next compositor commit.
    /// Worker→UI: posted at default dispatcher priority.
    #[default]
    Normal,

    /// UI→worker: out-of-band, processed immediately by the worker.
    /// Worker→UI: posted at Send dispatcher priority (highest).
    Oob,
}
