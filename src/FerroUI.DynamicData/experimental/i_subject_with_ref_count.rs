//! `ISubjectWithRefCount<T>` (`Experimental/ISubjectWithRefCount.cs`).

use ferroui_reactive::subjects::ISubject;

/// A subject that counts its subscriptions (`ISubjectWithRefCount<T>`).
pub(crate) trait ISubjectWithRefCount<T>: ISubject<T> {
    /// The number of subscriptions not yet disposed (`RefCount`).
    fn ref_count(&self) -> usize;
}
