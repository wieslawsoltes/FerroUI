//! `IgnoreSameReferenceUpdate` (`Cache/ObservableCacheEx.IgnoreSameReferenceUpdate.cs`).

macro_rules! ignore_same_reference_update_methods {
    () => {
        /// The changes without the updates whose current item is the previous one
        /// (`IgnoreSameReferenceUpdate`, `IgnoreUpdateWhen((c, p) => ReferenceEquals(c, p))`).
        ///
        /// Deviation (DEVIATIONS.md, DynamicData): the reference of an item is the object its
        /// pointer refers to (`T: Deref`, an `Rc<..>` for the classes of upstream); two items are
        /// the same reference when they point to the same object.
        fn ignore_same_reference_update(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: std::ops::Deref,
        {
            self.ignore_update_when(|current: &T, previous: &T| std::ptr::eq(std::ops::Deref::deref(current), std::ops::Deref::deref(previous)))
        }
    };
}
pub(crate) use ignore_same_reference_update_methods;
