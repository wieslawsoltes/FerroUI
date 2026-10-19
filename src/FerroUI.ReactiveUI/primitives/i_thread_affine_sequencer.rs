//! `IThreadAffineSequencer` (ReactiveUI.Primitives, `Concurrency/IThreadAffineSequencer.cs`).

use ferroui_reactive::concurrency::IScheduler;

/// A scheduler bound to one thread, which can tell whether the calling thread is that thread
/// (`IThreadAffineSequencer`), as a dispatcher can. In the `Reactive` flavour a sequencer is an
/// [`IScheduler`].
///
/// [`SequencerViewThreadInvoker`](crate::SequencerViewThreadInvoker) asks it whether a write may
/// run at once.
pub trait IThreadAffineSequencer: IScheduler {
    /// Whether the calling thread owns the scheduler, so that work may run at once
    /// (`CheckAccess`).
    fn check_access(&self) -> bool;
}
