//! `System.IProgress<T>`.

/// A receiver of progress updates (`IProgress<T>`): an operation reports its progress through it
/// while it runs.
///
/// The `Progress<T>` class of .NET, which posts each report to the synchronization context it was
/// made on, is not ported: the reactive libraries implement the trait with their own types
/// (`ObserverExt::to_progress` of `ferroui-reactive`).
pub trait IProgress<T> {
    /// Reports a progress update (`Report`).
    fn report(&self, value: T);
}
