//! `ObservableChainHelpers`: the observation of a property that holds an observable.

use crate::binding::observable_for_property::{ExpressionChainParameters, ExpressionChainSink, ValueComparer};
use crate::binding::interfaces::ObservedChange;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The value of a property that holds an observable: the observable itself, or an `Option` of it
/// (a property that may hold null).
pub trait ObservableLeaf: Clone + 'static {
    /// The type of the values of the observable.
    type Item: 'static;

    /// The observable the property holds, if any.
    fn observable(&self) -> Option<Rc<dyn IObservable<Self::Item>>>;

    /// Whether two values are the same observable (reference equality, as upstream compares them).
    fn same(&self, other: &Self) -> bool;
}

impl<R: 'static> ObservableLeaf for Rc<dyn IObservable<R>> {
    type Item = R;

    fn observable(&self) -> Option<Rc<dyn IObservable<R>>> {
        Some(self.clone())
    }

    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}

impl<R: 'static> ObservableLeaf for Option<Rc<dyn IObservable<R>>> {
    type Item = R;

    fn observable(&self) -> Option<Rc<dyn IObservable<R>>> {
        self.clone()
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(first), Some(second)) => Rc::ptr_eq(first, second),
            (None, None) => true,
            _ => false,
        }
    }
}

/// The values of the latest observable that `path` holds on `sender`; an empty sequence stands in
/// while the path holds none (`SwitchLatest`).
pub fn switch_latest<T: 'static, L: ObservableLeaf>(sender: &Rc<T>, path: PropertyPath<Rc<T>, L>) -> Rc<dyn IObservable<L::Item>> {
    let comparer: ValueComparer<L> = Rc::new(|first: &L, second: &L| first.same(second));
    let chain: Rc<dyn IObservable<ObservedChange<Rc<T>, L>>> =
        Rc::new(ExpressionChainSink::new(ExpressionChainParameters::with_comparer(sender, path, false, false, true, false, Some(comparer))));
    chain
        .select(|change: ObservedChange<Rc<T>, L>| change.value.observable().unwrap_or_else(Observable::empty))
        .switch()
}
