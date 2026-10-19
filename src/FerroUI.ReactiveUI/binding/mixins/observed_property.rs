//! `ObservedProperty`: the observation of properties without the rest of the binding machinery.

use crate::binding::bindings::BindingAffinity;
use crate::binding::observable_for_property::find_observer_for_property;
use crate::binding::observables::{PluginPropertyObservable, PropertyObservable, UnchangingPropertyObservable};
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// Observes properties with the behaviour of `WhenAnyValue`, one property at a time
/// (`ObservedProperty`).
///
/// Upstream offers it to the code that another source generator writes, which cannot rely on the
/// generator of `WhenAnyValue`; it takes the property as a lambda and a delegate that reads it.
/// Here a one-step [`PropertyPath`] carries both. [`then`](Self::then) continues a path one
/// property further, so `x => x.A.B` is `then(create(source, a), b)`; [`switch`](Self::switch)
/// follows a property that holds an observable, as `WhenAnyObservable` does.
pub struct ObservedProperty;

impl ObservedProperty {
    /// The current value of one property, then each value it changes to, skipping repeats
    /// (`Create(TSource, Expression<Func<TSource, TValue>>, Func<TSource, TValue>)`).
    ///
    /// Panics when `path` has more than one step (`ArgumentException`).
    pub fn create<S: 'static, V: Clone + PartialEq + 'static>(source: &Rc<S>, path: PropertyPath<Rc<S>, V>) -> Rc<dyn IObservable<V>> {
        Self::observe(source, path, true)
    }

    /// The current values of two properties, then a pair each time either changes
    /// (`Create(TSource, ..., ...)` of two properties).
    pub fn create2<S: 'static, V1: Clone + PartialEq + 'static, V2: Clone + PartialEq + 'static>(
        source: &Rc<S>,
        path1: PropertyPath<Rc<S>, V1>,
        path2: PropertyPath<Rc<S>, V2>,
    ) -> Rc<dyn IObservable<(V1, V2)>> {
        Self::create2_select(source, path1, path2, |value1, value2| (value1, value2))
    }

    /// The values of two properties projected by `selector`
    /// (`Create(TSource, ..., ..., Func<T1, T2, TResult>)`).
    pub fn create2_select<S: 'static, V1: Clone + PartialEq + 'static, V2: Clone + PartialEq + 'static, R: 'static>(
        source: &Rc<S>,
        path1: PropertyPath<Rc<S>, V1>,
        path2: PropertyPath<Rc<S>, V2>,
        selector: impl Fn(V1, V2) -> R + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::observe(source, path1, true).combine_latest(Self::observe(source, path2, true), selector)
    }

    /// The property of whatever object `parent` holds now, following each replacement of that
    /// object, skipping repeats; nothing is produced while `parent` holds `None`
    /// (`Then(IObservable<TParent?>, Expression<Func<TParent, TValue>>, Func<TParent, TValue>)`).
    pub fn then<P: 'static, X: Into<Option<Rc<P>>> + 'static, V: Clone + PartialEq + 'static>(
        parent: Rc<dyn IObservable<X>>,
        path: PropertyPath<Rc<P>, V>,
    ) -> Rc<dyn IObservable<V>> {
        Self::check_one_step(&path);
        parent
            .select(move |owner: X| match Into::<Option<Rc<P>>>::into(owner) {
                Some(owner) => Self::observe(&owner, path.clone(), false),
                None => Observable::empty(),
            })
            .switch()
            .distinct_until_changed()
    }

    /// The values of the latest observable that a property holds; nothing is produced while it
    /// holds `None` (`Switch(IObservable<IObservable<TValue>?>)`).
    pub fn switch<V: 'static>(source: Rc<dyn IObservable<Option<Rc<dyn IObservable<V>>>>>) -> Rc<dyn IObservable<V>> {
        source.select(|inner: Option<Rc<dyn IObservable<V>>>| inner.unwrap_or_else(Observable::empty)).switch()
    }

    fn check_one_step<S: 'static, V: 'static>(path: &PropertyPath<S, V>) {
        if path.steps().len() != 1 {
            panic!("The lambda must name a property or field of its parameter, as in x => x.Name. (Parameter 'property')");
        }
    }

    /// One property of one object, through the mechanism a generated binding chooses: a source
    /// that raises `PropertyChanged` is followed through it, another one is read once, and a
    /// registered observer that scores higher takes over (`Observe`).
    fn observe<S: 'static, V: Clone + PartialEq + 'static>(source: &Rc<S>, path: PropertyPath<Rc<S>, V>, distinct: bool) -> Rc<dyn IObservable<V>> {
        Self::check_one_step(&path);
        let step = &path.steps()[0];
        let name = step.name.clone();
        let root: Rc<dyn Any> = Rc::new(source.clone());
        let owner = (step.owner)(&*root);
        let affinity = if owner.notify.is_some() { BindingAffinity::EXPLICIT } else { 0 };
        let weak_source = Rc::downgrade(source);
        let getter = path.getter();
        if let Some((plugin, score)) = find_observer_for_property(&owner, &name, false) {
            if score > affinity {
                let read = getter.clone();
                let read_source = weak_source.clone();
                return Rc::new(PluginPropertyObservable::new(
                    plugin,
                    &owner,
                    name,
                    move || read_source.upgrade().map(|source| read(&source)),
                    false,
                    distinct,
                ));
            }
        }
        match owner.notify.clone() {
            Some(notify) => {
                let read_source = weak_source;
                Rc::new(PropertyObservable::<dyn INotifyPropertyChanged, V>::new(
                    &notify,
                    name,
                    move |_: &Rc<dyn INotifyPropertyChanged>| {
                        let source = read_source.upgrade().expect("the object that raised the notification is alive");
                        getter(&source)
                    },
                    distinct,
                ))
            }
            None => Rc::new(UnchangingPropertyObservable::new(getter(source))),
        }
    }
}
