//! `ReactiveUIBindingExtensions`: the property observation of an object, as methods of its `Rc`.
//!
//! Upstream's static partial class spreads over one file per family; each family is a module of
//! its own here (`when_any_value.rs`, `when_any.rs`, ...), and this trait gathers their methods.

use super::when_any_observable::WhenAnyObservablePaths;
use super::when_any_observable_extended::WhenAnyObservableCombinePaths;
use super::when_any_value::WhenAnyPaths;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The observation of the properties of an object (`ReactiveUIBindingExtensions`: `WhenAnyValue`,
/// `WhenAny`, `WhenAnyObservable`, `WhenChanged`, `WhenChanging`); implemented for every `Rc<T>`,
/// so that `vm.when_any_value(prop!(VM => name))` works.
///
/// The paths are one argument: one [`PropertyPath`](ferroui_system::component_model::PropertyPath)
/// or a tuple of 2 to 16 ([`WhenAnyPaths`]). The observation holds the object weakly.
pub trait ReactiveUIBindingExtensions<T: 'static> {
    /// The object observed.
    #[doc(hidden)]
    fn binding_source(&self) -> &Rc<T>;

    /// The latest values of `paths`: the current values on subscription, then the values after
    /// each change of any step of any path, skipping repeats (`WhenAnyValue`). The values of a
    /// tuple of paths are a tuple (upstream's `PropertyValues<T1, ..., Tn>`).
    fn when_any_value<P: WhenAnyPaths<T>>(&self, paths: P) -> Rc<dyn IObservable<P::Values>> {
        super::when_any_value::when_any_value(self.binding_source(), paths)
    }

    /// [`when_any_value`](Self::when_any_value) projected by `selector`, which takes the values
    /// as one value or one tuple (`WhenAnyValue(..., Func<T1, ..., TRet>)`).
    fn when_any_value_select<P: WhenAnyPaths<T>, R: 'static>(&self, paths: P, selector: impl Fn(P::Values) -> R + 'static) -> Rc<dyn IObservable<R>> {
        super::when_any_value_selectors::when_any_value_select(self.binding_source(), paths, selector)
    }

    /// The latest changes of `paths` projected by `selector`, which takes the changes (with their
    /// senders and paths) as one change or one tuple (`WhenAny`).
    fn when_any<P: WhenAnyPaths<T>, R: 'static>(&self, paths: P, selector: impl Fn(P::Changes) -> R + 'static) -> Rc<dyn IObservable<R>> {
        super::when_any::when_any(self.binding_source(), paths, selector)
    }

    /// The values of the latest observable that each of `paths` holds, merged
    /// (`WhenAnyObservable`); a path may hold `Rc<dyn IObservable<R>>` or an `Option` of it.
    fn when_any_observable<P: WhenAnyObservablePaths<T>>(&self, paths: P) -> Rc<dyn IObservable<P::Item>> {
        super::when_any_observable::when_any_observable(self.binding_source(), paths)
    }

    /// The latest values of the latest observable that each of `paths` holds, combined by
    /// `selector`, which takes them as one tuple (`WhenAnyObservable(..., Func<T1?, ..., TRet>)`).
    fn when_any_observable_select<P: WhenAnyObservableCombinePaths<T>, R: 'static>(
        &self,
        paths: P,
        selector: impl Fn(P::Items) -> R + 'static,
    ) -> Rc<dyn IObservable<R>> {
        super::when_any_observable_extended::when_any_observable_select(self.binding_source(), paths, selector)
    }

    /// The latest values of `paths` after each change, starting with the current values and
    /// skipping repeats (`WhenChanged`).
    fn when_changed<P: WhenAnyPaths<T>>(&self, paths: P) -> Rc<dyn IObservable<P::Values>> {
        super::when_changed::when_changed(self.binding_source(), paths)
    }

    /// [`when_changed`](Self::when_changed) converted by `conversion`
    /// (`WhenChanged(..., Func<T1, ..., TReturn>)`).
    fn when_changed_select<P: WhenAnyPaths<T>, R: 'static>(&self, paths: P, conversion: impl Fn(P::Values) -> R + 'static) -> Rc<dyn IObservable<R>> {
        super::when_changed_selectors::when_changed_select(self.binding_source(), paths, conversion)
    }

    /// The latest values of `paths` before each change, starting with the current values
    /// (`WhenChanging`); for a path of one property each notification is reported, even with the
    /// value of the last one (a deeper path drops repeats).
    fn when_changing<P: WhenAnyPaths<T>>(&self, paths: P) -> Rc<dyn IObservable<P::Values>> {
        super::when_changing::when_changing(self.binding_source(), paths)
    }

    /// [`when_changing`](Self::when_changing) converted by `conversion`
    /// (`WhenChanging(..., Func<T1, ..., TReturn>)`).
    fn when_changing_select<P: WhenAnyPaths<T>, R: 'static>(&self, paths: P, conversion: impl Fn(P::Values) -> R + 'static) -> Rc<dyn IObservable<R>> {
        super::when_changing_selectors::when_changing_select(self.binding_source(), paths, conversion)
    }
}

impl<T: 'static> ReactiveUIBindingExtensions<T> for Rc<T> {
    fn binding_source(&self) -> &Rc<T> {
        self
    }
}

