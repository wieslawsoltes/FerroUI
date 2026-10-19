//! `ReactiveUIBindingExtensions.WhenChanged.Selectors.WideArity.cs`: `WhenChanged` with a
//! conversion.

use super::when_any_value::WhenAnyPaths;
use super::when_changed::when_changed;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest values of `paths` on `source` after each change, converted by `conversion`, which
/// takes them as one value or one tuple (`WhenChanged(..., Func<T1, ..., TReturn>)`).
pub(crate) fn when_changed_select<T: 'static, P: WhenAnyPaths<T>, R: 'static>(
    source: &Rc<T>,
    paths: P,
    conversion: impl Fn(P::Values) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    when_changed(source, paths).select(conversion)
}
