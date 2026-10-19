//! `ReactiveUIBindingExtensions.WhenChanged.WideArity.cs`: `WhenChanged`.

use super::when_any_value::WhenAnyPaths;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest values of `paths` on `source` after each change, starting with the current values
/// and skipping repeats (`WhenChanged`).
pub(crate) fn when_changed<T: 'static, P: WhenAnyPaths<T>>(source: &Rc<T>, paths: P) -> Rc<dyn IObservable<P::Values>> {
    paths.observe_values(source, false)
}
