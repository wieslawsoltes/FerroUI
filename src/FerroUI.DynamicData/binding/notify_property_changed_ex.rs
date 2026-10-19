//! `NotifyPropertyChangedEx` (`Binding/NotifyPropertyChangedEx.cs`): the sequences of the changes
//! of the properties of an object that raises `PropertyChanged`.

use super::observable_property_factory::{when_changed_of, when_property_changed_of};
use super::{IntoPropertyChain, PropertyValue};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::{IObservable, IObserver};
use std::rc::Rc;

/// A fallback value of a property whose path a null step interrupts (`Func<TProperty>?`).
pub type Fallback<V> = Option<Rc<dyn Fn() -> V>>;

/// The observation of the properties of an object that raises `PropertyChanged`
/// (`NotifyPropertyChangedEx`), implemented for the objects held in `Rc` (upstream's classes).
///
/// The paths are [`PropertyPath`](ferroui_system::component_model::PropertyPath)s from `prop!`,
/// or [`PropertyChain`](super::PropertyChain)s when a step may be null. Upstream's optional
/// parameters become methods of their own: `_with` takes `notify_on_initial_value`, and
/// `_with_fallback` the fallback value too.
pub trait NotifyPropertyChangedEx: Clone + 'static {
    /// The object, whenever one of `properties_to_monitor` changes, or any property when none is
    /// given (`WhenAnyPropertyChanged(params string[] propertiesToMonitor)`).
    fn when_any_property_changed(&self, properties_to_monitor: &[&str]) -> Rc<dyn IObservable<Self>>;

    /// The changes of the property of `property_accessor`, as the object with the value, the
    /// current value first; nothing while a null step interrupts the path
    /// (`WhenPropertyChanged(Expression<Func<TObject, TProperty>>, bool notifyOnInitialValue = true)`).
    fn when_property_changed<P: IntoPropertyChain<Self>>(&self, property_accessor: P) -> Rc<dyn IObservable<PropertyValue<Self, P::Value>>>
    where
        P::Value: 'static,
    {
        self.when_property_changed_with(property_accessor, true)
    }

    /// [`when_property_changed`](Self::when_property_changed) that starts with the current value
    /// only when `notify_on_initial_value` is true
    /// (`WhenPropertyChanged(Expression<Func<TObject, TProperty>>, bool notifyOnInitialValue)`).
    fn when_property_changed_with<P: IntoPropertyChain<Self>>(
        &self,
        property_accessor: P,
        notify_on_initial_value: bool,
    ) -> Rc<dyn IObservable<PropertyValue<Self, P::Value>>>
    where
        P::Value: 'static,
    {
        when_property_changed_of(property_accessor.into_property_chain(), self.clone(), notify_on_initial_value, false)
    }

    /// [`when_property_changed_with`](Self::when_property_changed_with) with a fallback value: the
    /// values that a null step interrupts pass too, without a value
    /// (`WhenPropertyChanged(Expression<Func<TObject, TProperty>>, bool, Func<TProperty?>? fallbackValue)`).
    ///
    /// As upstream, the fallback is not called: its presence lets the unobtainable values through.
    fn when_property_changed_with_fallback<P: IntoPropertyChain<Self>>(
        &self,
        property_accessor: P,
        notify_on_initial_value: bool,
        _fallback_value: impl Fn() -> P::Value + 'static,
    ) -> Rc<dyn IObservable<PropertyValue<Self, P::Value>>>
    where
        P::Value: 'static,
    {
        when_property_changed_of(property_accessor.into_property_chain(), self.clone(), notify_on_initial_value, true)
    }

    /// The values of the property of `property_accessor`, the current value first; nothing while a
    /// null step interrupts the path
    /// (`WhenValueChanged(Expression<Func<TObject, TProperty>>, bool notifyOnInitialValue = true)`).
    fn when_value_changed<P: IntoPropertyChain<Self>>(&self, property_accessor: P) -> Rc<dyn IObservable<P::Value>>
    where
        P::Value: 'static,
    {
        self.when_value_changed_with(property_accessor, true)
    }

    /// [`when_value_changed`](Self::when_value_changed) that starts with the current value only
    /// when `notify_on_initial_value` is true
    /// (`WhenValueChanged(Expression<Func<TObject, TProperty>>, bool notifyOnInitialValue)`).
    fn when_value_changed_with<P: IntoPropertyChain<Self>>(&self, property_accessor: P, notify_on_initial_value: bool) -> Rc<dyn IObservable<P::Value>>
    where
        P::Value: 'static,
    {
        when_changed_of(property_accessor.into_property_chain(), self.clone(), notify_on_initial_value, None)
    }

    /// [`when_value_changed_with`](Self::when_value_changed_with) with `fallback_value` for the
    /// values that a null step interrupts
    /// (`WhenValueChanged(Expression<Func<TObject, TProperty>>, bool, Func<TProperty>? fallbackValue)`).
    fn when_value_changed_with_fallback<P: IntoPropertyChain<Self>>(
        &self,
        property_accessor: P,
        notify_on_initial_value: bool,
        fallback_value: impl Fn() -> P::Value + 'static,
    ) -> Rc<dyn IObservable<P::Value>>
    where
        P::Value: 'static,
    {
        when_changed_of(property_accessor.into_property_chain(), self.clone(), notify_on_initial_value, Some(Rc::new(fallback_value)))
    }

    /// The value of `p1` with the object, through `result_selector`, the current one first
    /// (`WhenChanged(Expression<Func<TObject, TProperty1>>, Func<TObject, TProperty1?, TResult>)`).
    fn when_changed<P1: IntoPropertyChain<Self>, R: 'static>(&self, p1: P1, result_selector: impl Fn(&Self, P1::Value) -> R + 'static) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
    {
        self.when_changed_with(p1, result_selector, None)
    }

    /// [`when_changed`](Self::when_changed) with a fallback value for the values that a null step
    /// interrupts, which are dropped without one
    /// (`WhenChanged(.., Func<TProperty1>? p1Fallback)`).
    fn when_changed_with<P1: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        result_selector: impl Fn(&Self, P1::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback).select(move |v1| result_selector(&source, v1))
    }

    /// The latest values of `p1` and `p2` with the object, through `result_selector`, whenever one
    /// changes (`WhenChanged` of two properties).
    fn when_changed2<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        result_selector: impl Fn(&Self, P1::Value, P2::Value) -> R + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
    {
        self.when_changed2_with(p1, p2, result_selector, None, None)
    }

    /// [`when_changed2`](Self::when_changed2) with fallback values (`WhenChanged` of two
    /// properties, with `p1Fallback` and `p2Fallback`).
    fn when_changed2_with<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        result_selector: impl Fn(&Self, P1::Value, P2::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
        p2_fallback: Fallback<P2::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback)
            .combine_latest(when_changed_of(p2.into_property_chain(), self.clone(), true, p2_fallback), move |v1, v2| result_selector(&source, v1, v2))
    }

    /// The latest values of three properties with the object, through `result_selector`
    /// (`WhenChanged` of three properties).
    fn when_changed3<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value) -> R + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
    {
        self.when_changed3_with(p1, p2, p3, result_selector, None, None, None)
    }

    /// [`when_changed3`](Self::when_changed3) with fallback values.
    #[allow(clippy::too_many_arguments)]
    fn when_changed3_with<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
        p2_fallback: Fallback<P2::Value>,
        p3_fallback: Fallback<P3::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback).combine_latest3(
            when_changed_of(p2.into_property_chain(), self.clone(), true, p2_fallback),
            when_changed_of(p3.into_property_chain(), self.clone(), true, p3_fallback),
            move |v1, v2, v3| result_selector(&source, v1, v2, v3),
        )
    }

    /// The latest values of four properties with the object, through `result_selector`
    /// (`WhenChanged` of four properties).
    fn when_changed4<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value) -> R + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
    {
        self.when_changed4_with(p1, p2, p3, p4, result_selector, None, None, None, None)
    }

    /// [`when_changed4`](Self::when_changed4) with fallback values.
    #[allow(clippy::too_many_arguments)]
    fn when_changed4_with<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
        p2_fallback: Fallback<P2::Value>,
        p3_fallback: Fallback<P3::Value>,
        p4_fallback: Fallback<P4::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback).combine_latest4(
            when_changed_of(p2.into_property_chain(), self.clone(), true, p2_fallback),
            when_changed_of(p3.into_property_chain(), self.clone(), true, p3_fallback),
            when_changed_of(p4.into_property_chain(), self.clone(), true, p4_fallback),
            move |v1, v2, v3, v4| result_selector(&source, v1, v2, v3, v4),
        )
    }

    /// The latest values of five properties with the object, through `result_selector`
    /// (`WhenChanged` of five properties).
    #[allow(clippy::too_many_arguments)]
    fn when_changed5<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, P5: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        p5: P5,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value, P5::Value) -> R + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
        P5::Value: Clone + 'static,
    {
        self.when_changed5_with(p1, p2, p3, p4, p5, result_selector, None, None, None, None, None)
    }

    /// [`when_changed5`](Self::when_changed5) with fallback values.
    #[allow(clippy::too_many_arguments)]
    fn when_changed5_with<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, P5: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        p5: P5,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value, P5::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
        p2_fallback: Fallback<P2::Value>,
        p3_fallback: Fallback<P3::Value>,
        p4_fallback: Fallback<P4::Value>,
        p5_fallback: Fallback<P5::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
        P5::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback).combine_latest5(
            when_changed_of(p2.into_property_chain(), self.clone(), true, p2_fallback),
            when_changed_of(p3.into_property_chain(), self.clone(), true, p3_fallback),
            when_changed_of(p4.into_property_chain(), self.clone(), true, p4_fallback),
            when_changed_of(p5.into_property_chain(), self.clone(), true, p5_fallback),
            move |v1, v2, v3, v4, v5| result_selector(&source, v1, v2, v3, v4, v5),
        )
    }

    /// The latest values of six properties with the object, through `result_selector`
    /// (`WhenChanged` of six properties).
    #[allow(clippy::too_many_arguments)]
    fn when_changed6<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, P5: IntoPropertyChain<Self>, P6: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        p5: P5,
        p6: P6,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value, P5::Value, P6::Value) -> R + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
        P5::Value: Clone + 'static,
        P6::Value: Clone + 'static,
    {
        self.when_changed6_with(p1, p2, p3, p4, p5, p6, result_selector, None, None, None, None, None, None)
    }

    /// [`when_changed6`](Self::when_changed6) with fallback values.
    #[allow(clippy::too_many_arguments)]
    fn when_changed6_with<P1: IntoPropertyChain<Self>, P2: IntoPropertyChain<Self>, P3: IntoPropertyChain<Self>, P4: IntoPropertyChain<Self>, P5: IntoPropertyChain<Self>, P6: IntoPropertyChain<Self>, R: 'static>(
        &self,
        p1: P1,
        p2: P2,
        p3: P3,
        p4: P4,
        p5: P5,
        p6: P6,
        result_selector: impl Fn(&Self, P1::Value, P2::Value, P3::Value, P4::Value, P5::Value, P6::Value) -> R + 'static,
        p1_fallback: Fallback<P1::Value>,
        p2_fallback: Fallback<P2::Value>,
        p3_fallback: Fallback<P3::Value>,
        p4_fallback: Fallback<P4::Value>,
        p5_fallback: Fallback<P5::Value>,
        p6_fallback: Fallback<P6::Value>,
    ) -> Rc<dyn IObservable<R>>
    where
        P1::Value: Clone + 'static,
        P2::Value: Clone + 'static,
        P3::Value: Clone + 'static,
        P4::Value: Clone + 'static,
        P5::Value: Clone + 'static,
        P6::Value: Clone + 'static,
    {
        let source = self.clone();
        when_changed_of(p1.into_property_chain(), self.clone(), true, p1_fallback).combine_latest6(
            when_changed_of(p2.into_property_chain(), self.clone(), true, p2_fallback),
            when_changed_of(p3.into_property_chain(), self.clone(), true, p3_fallback),
            when_changed_of(p4.into_property_chain(), self.clone(), true, p4_fallback),
            when_changed_of(p5.into_property_chain(), self.clone(), true, p5_fallback),
            when_changed_of(p6.into_property_chain(), self.clone(), true, p6_fallback),
            move |v1, v2, v3, v4, v5, v6| result_selector(&source, v1, v2, v3, v4, v5, v6),
        )
    }
}

impl<M: INotifyPropertyChanged + 'static> NotifyPropertyChangedEx for Rc<M> {
    fn when_any_property_changed(&self, properties_to_monitor: &[&str]) -> Rc<dyn IObservable<Self>> {
        let source = self.clone();
        let properties_to_monitor: Rc<[String]> = properties_to_monitor.iter().map(|name| name.to_string()).collect();
        // `Observable.FromEventPattern` of `PropertyChanged`: a handler per subscription.
        Observable::create(move |observer: Rc<dyn IObserver<Rc<M>>>| {
            let (sender, properties_to_monitor) = (source.clone(), properties_to_monitor.clone());
            let token = source.property_changed().add(Rc::new(move |property_name: &str| {
                if properties_to_monitor.is_empty() || properties_to_monitor.iter().any(|name| name == property_name) {
                    observer.on_next(sender.clone());
                }
            }));
            let target = source.clone();
            ferroui_reactive::disposables::Disposable::create(move || {
                target.property_changed().remove(token);
            })
        })
    }
}
