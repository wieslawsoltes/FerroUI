//! `ReactiveUIBindingExtensions.WhenAny.WideArity.cs`: `WhenAny`.

use super::when_any_value::WhenAnyPaths;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The latest changes of `paths` on `source`, projected by `selector`, which takes the changes
/// (`ObservedChange`s, with their senders and paths) as one change or one tuple
/// (`WhenAny(..., Func<IObservedChange<TSender, T1>, ..., TRet>)`).
pub(crate) fn when_any<T: 'static, P: WhenAnyPaths<T>, R: 'static>(
    source: &Rc<T>,
    paths: P,
    selector: impl Fn(P::Changes) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    paths.observe_changes(source, false).select(selector)
}
