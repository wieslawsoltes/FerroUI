//! `ReactiveUIBindingExtensions.WhenChanging.WideArity.cs`: `WhenChanging`.

use super::when_any_value::WhenAnyPaths;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest values of `paths` on `source` before each change, starting with the current values
/// (`WhenChanging`); for a path of one property a value equal to the last one is reported too,
/// since each notification is a change of its own (a deeper path drops repeats). An object that does not raise `PropertyChanging` is
/// observed after each change.
pub(crate) fn when_changing<T: 'static, P: WhenAnyPaths<T>>(source: &Rc<T>, paths: P) -> Rc<dyn IObservable<P::Values>> {
    paths.observe_values(source, true)
}
