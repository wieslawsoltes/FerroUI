//! [`ObservableExt`]: the operators of a sequence as methods of its handle.

use ferroui_system::IObservable;
use std::rc::Rc;

/// The operators of an observable sequence, as methods of its handle (the extension methods of
/// upstream's `Observable` and `ObservableExtensions`).
///
/// The one required method gives the sequence as a handle; every operator is a provided method,
/// declared in the module of its family (`select`, `where_`, ...). The trait is implemented for
/// `Rc<dyn IObservable<T>>`, and for the subjects and test observables through it.
pub trait ObservableExt<T: 'static> {
    /// The sequence as the handle the operators take.
    fn observable(&self) -> Rc<dyn IObservable<T>>;

    super::subscribe::subscribe_methods!();
    super::select::select_methods!();
    super::where_::where_methods!();
}

impl<T: 'static> ObservableExt<T> for Rc<dyn IObservable<T>> {
    fn observable(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}
