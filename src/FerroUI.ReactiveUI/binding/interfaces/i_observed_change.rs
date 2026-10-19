//! `IObservedChange<TSender, TValue>`.

/// One property change: the object that raised it, the path of the property, and its value
/// (`IObservedChange<TSender, TValue>`).
pub trait IObservedChange<S, V> {
    /// The object that raised the change (`Sender`).
    fn sender(&self) -> &S;

    /// The path of the property that changed, `Address.Street` (`Expression`).
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream gives the member expression; the port gives
    // the text of the property path, which is what the expression is read for.
    fn expression(&self) -> &str;

    /// The value the change carries (`Value`).
    fn value(&self) -> &V;
}
