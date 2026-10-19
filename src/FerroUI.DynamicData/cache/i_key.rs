//! `IKey<T>` (`Cache/IKey.cs`).

/// An object that knows its key (`IKey<out T>`): `StartWithItem(item)` and the groups read it.
pub trait IKey<K> {
    /// The key (`Key`).
    fn key(&self) -> K;
}
