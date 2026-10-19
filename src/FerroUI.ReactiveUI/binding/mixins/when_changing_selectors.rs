//! `ReactiveUIBindingExtensions.WhenChanging.Selectors.WideArity.cs`: `WhenChanging` with a
//! conversion.

use super::when_any_value::WhenAnyPaths;
use super::when_changing::when_changing;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest values of `paths` on `source` before each change, converted by `conversion`, which
/// takes them as one value or one tuple (`WhenChanging(..., Func<T1, ..., TReturn>)`).
pub(crate) fn when_changing_select<T: 'static, P: WhenAnyPaths<T>, R: 'static>(
    source: &Rc<T>,
    paths: P,
    conversion: impl Fn(P::Values) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    when_changing(source, paths).select(conversion)
}
