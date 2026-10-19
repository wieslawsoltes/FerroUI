//! `ReactiveUIBindingExtensions.WhenAnyValue.Selectors.WideArity.cs`: `WhenAnyValue` with a selector.

use super::when_any_value::{when_any_value, WhenAnyPaths};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest values of `paths` on `source`, projected by `selector`, which takes them as one
/// value or one tuple (`WhenAnyValue(..., Func<T1, ..., TRet>)`).
pub(crate) fn when_any_value_select<T: 'static, P: WhenAnyPaths<T>, R: 'static>(
    source: &Rc<T>,
    paths: P,
    selector: impl Fn(P::Values) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    when_any_value(source, paths).select(selector)
}
