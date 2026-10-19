//! `ReactiveNotifyPropertyChangedMixins`: the observation of a property path, and the choice of the
//! property observer of an object.

use super::{ExpressionChainParameters, ExpressionChainSink, INPCObservableForProperty, ObservableForPropertySink, POCOObservableForProperty};
use crate::binding::interfaces::{ICreatesObservableForProperty, ObservedChange};
use ferroui_splat::AppLocator;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner, PropertyPath};
use ferroui_system::IObservable;
use std::rc::Rc;

const BROKEN_LOCATOR_ADVICE: &str = "This should never happen, your service locator is probably broken. Please make sure you have registered ICreatesObservableForProperty implementations.";

ferroui_system::ferro_thread_local! {
    /// The observers used while none is registered.
    static DEFAULT_OBSERVERS: Vec<Rc<dyn ICreatesObservableForProperty>> =
        vec![Rc::new(INPCObservableForProperty::new()), Rc::new(POCOObservableForProperty::new())];
}

/// The property observers registered in `AppLocator` (`GetServices<ICreatesObservableForProperty>`).
// Deviation (DEVIATIONS.md, ReactiveUI): while none is registered, the default ones of
// `ReactiveUIBindingModule` (`INPCObservableForProperty`, `POCOObservableForProperty`) are used,
// where upstream fails; so a property is observed before the application is built, as in a unit
// test of a view model.
pub fn observers_for_property() -> Vec<Rc<dyn ICreatesObservableForProperty>> {
    let registered = AppLocator::get_services::<dyn ICreatesObservableForProperty>();
    if registered.is_empty() {
        DEFAULT_OBSERVERS.with(|observers| observers.clone())
    } else {
        registered
    }
}

/// The observer with the highest positive affinity for `property_name` of `owner`, with its
/// affinity; of equal affinities, the first registered wins (`ResolveNotifyFactory`).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream memoizes the choice per type and property; here
// the affinity is computed on the object, each time a step is observed.
pub fn find_observer_for_property(owner: &PropertyOwner, property_name: &str, before_changed: bool) -> Option<(Rc<dyn ICreatesObservableForProperty>, i32)> {
    let mut best: Option<(Rc<dyn ICreatesObservableForProperty>, i32)> = None;
    for candidate in observers_for_property() {
        let score = candidate.get_affinity_for_object(owner, property_name, before_changed);
        let best_score = best.as_ref().map_or(0, |(_, score)| *score);
        if score <= best_score {
            continue;
        }
        best = Some((candidate, score));
    }
    best
}

/// The notifications of `property_name` of `owner`, from the observer with the highest affinity
/// (`NotifyForProperty`).
///
/// Panics when no observer can observe it (`InvalidOperationException`).
pub fn notify_for_property(
    owner: &PropertyOwner,
    property_name: &str,
    before_change: bool,
    suppress_warnings: bool,
) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
    match find_observer_for_property(owner, property_name, before_change) {
        Some((observer, _)) => observer.get_notification_for_property(owner, property_name, before_change, suppress_warnings),
        None => panic!("Could not find a ICreatesObservableForProperty for property {property_name}. {BROKEN_LOCATOR_ADVICE}"),
    }
}

/// The observation of the properties of an object, with the options of each observation
/// (`ReactiveNotifyPropertyChangedMixins`); implemented for every `Rc<T>`.
pub trait ReactiveNotifyPropertyChangedMixins<T: 'static> {
    /// The object observed.
    #[doc(hidden)]
    fn observed_object(&self) -> &Rc<T>;

    /// The changes of the value at the end of `path`, before or after each change; `skip_initial`
    /// drops the current value, `is_distinct` drops a value equal to the last one
    /// (`ObservableForProperty(Expression<Func<TSender, TValue>>, bool, bool, bool)`).
    fn observable_for_property<V: Clone + PartialEq + 'static>(
        &self,
        path: PropertyPath<Rc<T>, V>,
        before_change: bool,
        skip_initial: bool,
        is_distinct: bool,
    ) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>> {
        self.subscribe_to_expression_chain(path, before_change, skip_initial, is_distinct)
    }

    /// [`observable_for_property`](Self::observable_for_property) after each change, dropping
    /// repeats (`ObservableForProperty(Expression<Func<TSender, TValue>>, bool)`, and with
    /// `skip_initial` set, `ObservableForProperty(Expression<Func<TSender, TValue>>)`).
    fn observable_for_property_skip_initial<V: Clone + PartialEq + 'static>(
        &self,
        path: PropertyPath<Rc<T>, V>,
        skip_initial: bool,
    ) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>> {
        self.subscribe_to_expression_chain(path, false, skip_initial, true)
    }

    /// The changes of the property `property_name`, read with `getter`
    /// (`ObservableForProperty<TValue>(string, bool, bool, bool)`).
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream reads the property by reflection; the port
    // takes its getter.
    fn observable_for_property_named<V: Clone + PartialEq + 'static>(
        &self,
        property_name: &str,
        getter: impl Fn(&Rc<T>) -> V + 'static,
        before_change: bool,
        skip_initial: bool,
        is_distinct: bool,
    ) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>>
    where
        T: INotifyPropertyChanged,
    {
        let source = self.observed_object();
        let owner = PropertyOwner {
            object: Some(source.clone() as Rc<dyn std::any::Any>),
            notify: Some(source.clone() as Rc<dyn INotifyPropertyChanged>),
        };
        let notifications = notify_for_property(&owner, property_name, before_change, false);
        let comparer: Option<super::expression_chain_parameters::ValueComparer<V>> =
            if is_distinct { Some(Rc::new(|first: &V, second: &V| first == second)) } else { None };
        Rc::new(ObservableForPropertySink::new(source, property_name, notifications, getter, skip_initial, comparer))
    }

    /// The changes of the value at the end of `path`, step by step
    /// (`SubscribeToExpressionChain(Expression?, bool, bool, bool)`).
    fn subscribe_to_expression_chain<V: Clone + PartialEq + 'static>(
        &self,
        path: PropertyPath<Rc<T>, V>,
        before_change: bool,
        skip_initial: bool,
        is_distinct: bool,
    ) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>> {
        self.subscribe_to_expression_chain_with(path, before_change, skip_initial, is_distinct, false)
    }

    /// [`subscribe_to_expression_chain`](Self::subscribe_to_expression_chain) that can suppress
    /// the warning of a property that cannot notify
    /// (`SubscribeToExpressionChain(Expression?, bool, bool, bool, bool)`).
    fn subscribe_to_expression_chain_with<V: Clone + PartialEq + 'static>(
        &self,
        path: PropertyPath<Rc<T>, V>,
        before_change: bool,
        skip_initial: bool,
        is_distinct: bool,
        suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>> {
        Rc::new(ExpressionChainSink::new(ExpressionChainParameters::new(
            self.observed_object(),
            path,
            before_change,
            skip_initial,
            is_distinct,
            suppress_warnings,
        )))
    }
}

impl<T: 'static> ReactiveNotifyPropertyChangedMixins<T> for Rc<T> {
    fn observed_object(&self) -> &Rc<T> {
        self
    }
}
