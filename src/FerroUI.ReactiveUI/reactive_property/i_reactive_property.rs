//! `IReactiveProperty<T>` (`ReactiveUI.Core/ReactiveProperty/IReactiveProperty.cs`).

use super::DataErrors;
use ferroui_reactive::disposables::ICancelable;
use ferroui_system::component_model::{INotifyDataErrorInfo, INotifyPropertyChanged};
use ferroui_system::IObservable;
use std::rc::Rc;

/// A property that notifies the observers of its value and of its validation errors, and that
/// can be disposed (`IReactiveProperty<T>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's `IsDisposed` interface of
/// ReactiveUI.Primitives is [`ICancelable`], the disposable that says whether it was disposed.
pub trait IReactiveProperty<T>: IObservable<T> + ICancelable + INotifyDataErrorInfo + INotifyPropertyChanged {
    /// The value (`Value { get; }`).
    fn value(&self) -> T;

    /// Sets the value (`Value { set; }`).
    fn set_value(&self, value: T);

    /// The errors of the property each time they change (`ObserveErrorChanged`); `None` when it
    /// has none.
    fn observe_error_changed(&self) -> Rc<dyn IObservable<Option<DataErrors>>>;

    /// Whether the property has validation errors, each time its errors change
    /// (`ObserveHasErrors`).
    fn observe_has_errors(&self) -> Rc<dyn IObservable<bool>>;

    /// Notifies the current value again, even when it did not change (`Refresh`).
    fn refresh(&self);
}
