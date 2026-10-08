//! Extension members for [`FerroObject`]: observables over property values
//! and property-kind-agnostic access to typed properties.

use crate::data::core::{UntypedObservableBindingExpression, ValueTypes};
use crate::data::{BindingBase, BindingExpressionBase, BindingPriority, BindingValue};
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver, ObservableExt};
use crate::{
    AttachedProperty, BoxedValue, DirectProperty, DirectPropertyBase, FerroObject, FerroProperty,
    FerroPropertyChangedEventArgs, ObjectType, PropertyValue, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Converts an observable to a binding (`source.ToBinding()`).
///
/// The values of the observable are delivered untyped: a nullable value
/// (`Option<T>`) as null or the value it holds.
pub fn to_binding<T: PropertyValue>(source: Rc<dyn IObservable<T>>) -> Rc<dyn BindingBase> {
    Rc::new(BindingAdaptor { source: source.select(|x| ValueTypes::normalize(Rc::new(x))) })
}

/// The binding [`to_binding`] returns: its instances read the values of the
/// observable with local value priority.
struct BindingAdaptor {
    source: Rc<dyn IObservable<Option<BoxedValue>>>,
}

impl BindingBase for BindingAdaptor {
    fn create_instance(
        &self,
        _target: &FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&crate::Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let expression: Rc<dyn BindingExpressionBase> =
            UntypedObservableBindingExpression::new(self.source.clone(), BindingPriority::LocalValue);
        expression
    }
}

/// A property with a statically known value type: a styled, attached or
/// direct property.
///
/// This is what lets the members of [`FerroObjectExtensions`] accept any kind
/// of typed property and route to the matching [`FerroObject`] member.
pub trait TypedProperty: 'static {
    /// The value type of the property.
    type Value: PropertyValue;

    /// The untyped base of the property.
    fn untyped(&'static self) -> &'static FerroProperty;

    #[doc(hidden)]
    fn typed_get_value(&'static self, target: &FerroObject) -> Self::Value;

    #[doc(hidden)]
    fn typed_get_base_value(&'static self, target: &FerroObject) -> Option<Self::Value>;

    #[doc(hidden)]
    fn typed_bind(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<Self::Value>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;

    #[doc(hidden)]
    fn typed_bind_value(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<BindingValue<Self::Value>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;
}

impl<T: PropertyValue> TypedProperty for StyledProperty<T> {
    type Value = T;

    fn untyped(&'static self) -> &'static FerroProperty {
        self
    }

    fn typed_get_value(&'static self, target: &FerroObject) -> T {
        target.get_value(self)
    }

    fn typed_get_base_value(&'static self, target: &FerroObject) -> Option<T> {
        target.get_base_value(self)
    }

    fn typed_bind(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<T>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind(self, source, priority)
    }

    fn typed_bind_value(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<BindingValue<T>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_value(self, source, priority)
    }
}

impl<T: PropertyValue> TypedProperty for AttachedProperty<T> {
    type Value = T;

    fn untyped(&'static self) -> &'static FerroProperty {
        self
    }

    fn typed_get_value(&'static self, target: &FerroObject) -> T {
        target.get_value(self)
    }

    fn typed_get_base_value(&'static self, target: &FerroObject) -> Option<T> {
        target.get_base_value(self)
    }

    fn typed_bind(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<T>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind(self, source, priority)
    }

    fn typed_bind_value(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<BindingValue<T>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_value(self, source, priority)
    }
}

impl<T: PropertyValue> TypedProperty for DirectPropertyBase<T> {
    type Value = T;

    fn untyped(&'static self) -> &'static FerroProperty {
        self
    }

    fn typed_get_value(&'static self, target: &FerroObject) -> T {
        target.get_direct_value(self)
    }

    fn typed_get_base_value(&'static self, target: &FerroObject) -> Option<T> {
        Some(target.get_direct_value(self))
    }

    fn typed_bind(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<T>>,
        _priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_direct(self, source)
    }

    fn typed_bind_value(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<BindingValue<T>>>,
        _priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_direct_value(self, source)
    }
}

impl<TOwner: ObjectType, T: PropertyValue> TypedProperty for DirectProperty<TOwner, T> {
    type Value = T;

    fn untyped(&'static self) -> &'static FerroProperty {
        self
    }

    fn typed_get_value(&'static self, target: &FerroObject) -> T {
        target.get_direct_value(self)
    }

    fn typed_get_base_value(&'static self, target: &FerroObject) -> Option<T> {
        Some(target.get_direct_value(self))
    }

    fn typed_bind(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<T>>,
        _priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_direct(self, source)
    }

    fn typed_bind_value(
        &'static self,
        target: &FerroObject,
        source: Rc<dyn IObservable<BindingValue<T>>>,
        _priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        target.bind_direct_value(self, source)
    }
}

/// Extension members for [`FerroObject`].
pub trait FerroObjectExtensions {
    /// Gets an observable for a property.
    ///
    /// The observable fires with the current value of the property on the
    /// object immediately upon subscription, and then every time the value
    /// changes. It holds a weak reference to the object.
    fn get_observable<P: TypedProperty>(&self, property: &'static P) -> Rc<dyn IObservable<P::Value>>;

    /// Gets an observable for a property, projecting each value with
    /// `converter`.
    fn get_observable_with<P: TypedProperty, TResult: PropertyValue>(
        &self,
        property: &'static P,
        converter: impl Fn(P::Value) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>>;

    /// Gets an observable of untyped values for a property.
    fn get_observable_untyped(&self, property: &'static FerroProperty) -> Rc<dyn IObservable<BoxedValue>>;

    /// Gets an observable of untyped values for a property, projecting each
    /// value with `converter`.
    fn get_observable_untyped_with<TResult: PropertyValue>(
        &self,
        property: &'static FerroProperty,
        converter: impl Fn(BoxedValue) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>>;

    /// Gets an observable of binding values for a property.
    fn get_binding_observable<P: TypedProperty>(
        &self,
        property: &'static P,
    ) -> Rc<dyn IObservable<BindingValue<P::Value>>>;

    /// Gets an observable of binding values for a property, projecting each
    /// value with `converter`.
    fn get_binding_observable_with<P: TypedProperty, TResult: PropertyValue>(
        &self,
        property: &'static P,
        converter: impl Fn(P::Value) -> TResult + 'static,
    ) -> Rc<dyn IObservable<BindingValue<TResult>>>;

    /// Gets an observable of untyped binding values for a property.
    fn get_binding_observable_untyped(
        &self,
        property: &'static FerroProperty,
    ) -> Rc<dyn IObservable<BindingValue<BoxedValue>>>;

    /// Gets an observable of untyped binding values for a property,
    /// projecting each value with `converter`.
    fn get_binding_observable_untyped_with<TResult: PropertyValue>(
        &self,
        property: &'static FerroProperty,
        converter: impl Fn(BoxedValue) -> TResult + 'static,
    ) -> Rc<dyn IObservable<BindingValue<TResult>>>;

    /// Gets an observable that listens for property changed events of a
    /// property on this object. It fires each time the property changes and
    /// does not fire with the current value on subscription.
    fn get_property_changed_observable(&self, property: &'static FerroProperty) -> ObjectPropertyChangedObservable;

    /// Binds a typed property of any kind to an observable. The priority is
    /// ignored for direct properties.
    fn bind_typed<P: TypedProperty>(
        &self,
        property: &'static P,
        source: Rc<dyn IObservable<P::Value>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;

    /// Binds a typed property of any kind to an observable of binding values.
    /// The priority is ignored for direct properties.
    fn bind_typed_value<P: TypedProperty>(
        &self,
        property: &'static P,
        source: Rc<dyn IObservable<BindingValue<P::Value>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;

    /// Gets the value of a typed property of any kind.
    fn get_typed_value<P: TypedProperty>(&self, property: &'static P) -> P::Value;

    /// Gets the base value of a typed property of any kind: the value
    /// excluding animated values. For direct properties this is the value.
    fn get_typed_base_value<P: TypedProperty>(&self, property: &'static P) -> Option<P::Value>;

    /// Gets the base value of a property as an untyped value; the unset
    /// marker when there is none.
    fn get_base_value_untyped(&self, property: &'static FerroProperty) -> BoxedValue;
}

impl FerroObjectExtensions for FerroObject {
    fn get_observable<P: TypedProperty>(&self, property: &'static P) -> Rc<dyn IObservable<P::Value>> {
        PropertyObservable::create(
            self,
            property.untyped(),
            move |o| property.typed_get_value(o),
            |a, b| a == b,
            true,
        )
    }

    fn get_observable_with<P: TypedProperty, TResult: PropertyValue>(
        &self,
        property: &'static P,
        converter: impl Fn(P::Value) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>> {
        PropertyObservable::create(
            self,
            property.untyped(),
            move |o| converter(property.typed_get_value(o)),
            |a, b| a == b,
            true,
        )
    }

    fn get_observable_untyped(&self, property: &'static FerroProperty) -> Rc<dyn IObservable<BoxedValue>> {
        PropertyObservable::create(self, property, move |o| o.get_value_untyped(property), |a, b| a == b, true)
    }

    fn get_observable_untyped_with<TResult: PropertyValue>(
        &self,
        property: &'static FerroProperty,
        converter: impl Fn(BoxedValue) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>> {
        PropertyObservable::create(
            self,
            property,
            move |o| converter(o.get_value_untyped(property)),
            |a, b| a == b,
            true,
        )
    }

    fn get_binding_observable<P: TypedProperty>(
        &self,
        property: &'static P,
    ) -> Rc<dyn IObservable<BindingValue<P::Value>>> {
        PropertyObservable::create(
            self,
            property.untyped(),
            move |o| BindingValue::new(property.typed_get_value(o)),
            |a, b| a == b,
            false,
        )
    }

    fn get_binding_observable_with<P: TypedProperty, TResult: PropertyValue>(
        &self,
        property: &'static P,
        converter: impl Fn(P::Value) -> TResult + 'static,
    ) -> Rc<dyn IObservable<BindingValue<TResult>>> {
        PropertyObservable::create(
            self,
            property.untyped(),
            move |o| BindingValue::new(converter(property.typed_get_value(o))),
            |a, b| a == b,
            false,
        )
    }

    fn get_binding_observable_untyped(
        &self,
        property: &'static FerroProperty,
    ) -> Rc<dyn IObservable<BindingValue<BoxedValue>>> {
        PropertyObservable::create(
            self,
            property,
            move |o| BindingValue::new(o.get_value_untyped(property)),
            |a, b| a == b,
            false,
        )
    }

    fn get_binding_observable_untyped_with<TResult: PropertyValue>(
        &self,
        property: &'static FerroProperty,
        converter: impl Fn(BoxedValue) -> TResult + 'static,
    ) -> Rc<dyn IObservable<BindingValue<TResult>>> {
        PropertyObservable::create(
            self,
            property,
            move |o| BindingValue::new(converter(o.get_value_untyped(property))),
            |a, b| a == b,
            false,
        )
    }

    fn get_property_changed_observable(&self, property: &'static FerroProperty) -> ObjectPropertyChangedObservable {
        ObjectPropertyChangedObservable { target: self.to_weak(), property }
    }

    fn bind_typed<P: TypedProperty>(
        &self,
        property: &'static P,
        source: Rc<dyn IObservable<P::Value>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        property.typed_bind(self, source, priority)
    }

    fn bind_typed_value<P: TypedProperty>(
        &self,
        property: &'static P,
        source: Rc<dyn IObservable<BindingValue<P::Value>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        property.typed_bind_value(self, source, priority)
    }

    fn get_typed_value<P: TypedProperty>(&self, property: &'static P) -> P::Value {
        property.typed_get_value(self)
    }

    fn get_typed_base_value<P: TypedProperty>(&self, property: &'static P) -> Option<P::Value> {
        property.typed_get_base_value(self)
    }

    fn get_base_value_untyped(&self, property: &'static FerroProperty) -> BoxedValue {
        property.routes().route_get_base_value(self)
    }
}

/// The change notifications of one property on one object.
///
/// Returned by
/// [`get_property_changed_observable`](FerroObjectExtensions::get_property_changed_observable).
/// Change notifications borrow their values, so handlers are closures rather
/// than observers. The object is held weakly.
#[derive(Clone)]
pub struct ObjectPropertyChangedObservable {
    target: WeakRef<FerroObject>,
    property: &'static FerroProperty,
}

impl ObjectPropertyChangedObservable {
    /// Subscribes to changes of the property on the object. Disposing the
    /// returned handle unsubscribes.
    pub fn subscribe(&self, handler: impl Fn(&FerroPropertyChangedEventArgs<'_>) + 'static) -> Rc<dyn IDisposable> {
        let property = self.property;
        match self.target.upgrade() {
            Some(target) => target.property_changed(move |e| {
                if e.property() == property {
                    handler(e);
                }
            }),
            None => Disposable::empty(),
        }
    }

    /// Subscribes a handler that is only invoked when the object is of class
    /// `TTarget` (or a class derived from it).
    pub fn add_class_handler<TTarget: ObjectType>(
        &self,
        handler: impl Fn(&TTarget, &FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe(move |e| {
            if let Some(target) = e.sender().downcast_ref::<TTarget>() {
                handler(target, e);
            }
        })
    }
}

/// An observable over the value of a property on an object.
///
/// Listens to the object's change notifications while it has observers, and
/// replays the current value to each new observer.
struct PropertyObservable<TResult: Clone + 'static> {
    this: Weak<PropertyObservable<TResult>>,
    target: WeakRef<FerroObject>,
    property: &'static FerroProperty,
    read: Box<dyn Fn(&FerroObject) -> TResult>,
    same: fn(&TResult, &TResult) -> bool,
    /// Whether the cached value is dropped when the last observer leaves.
    clear_value_on_deinitialize: bool,
    value: RefCell<Option<TResult>>,
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<TResult>>)>>,
    next_id: Cell<u64>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl<TResult: Clone + 'static> PropertyObservable<TResult> {
    fn create(
        target: &FerroObject,
        property: &'static FerroProperty,
        read: impl Fn(&FerroObject) -> TResult + 'static,
        same: fn(&TResult, &TResult) -> bool,
        clear_value_on_deinitialize: bool,
    ) -> Rc<dyn IObservable<TResult>> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            target: target.to_weak(),
            property,
            read: Box::new(read),
            same,
            clear_value_on_deinitialize,
            value: RefCell::new(None),
            observers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
            subscription: RefCell::new(None),
        })
    }

    fn initialize(&self) {
        let Some(target) = self.target.upgrade() else { return };
        let value = (self.read)(&target);
        *self.value.borrow_mut() = Some(value);
        // The handler keeps the observable alive for as long as it has
        // observers, even if the caller dropped its own reference.
        let Some(this) = self.this.upgrade() else { return };
        let subscription = target.property_changed(move |e| this.property_changed(e));
        *self.subscription.borrow_mut() = Some(subscription);
    }

    fn deinitialize(&self) {
        let subscription = self.subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        if self.clear_value_on_deinitialize {
            *self.value.borrow_mut() = None;
        }
    }

    fn property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() != self.property {
            return;
        }
        let new_value = (self.read)(e.sender());
        let changed = match &*self.value.borrow() {
            Some(current) => !(self.same)(&new_value, current),
            None => true,
        };
        if changed {
            *self.value.borrow_mut() = Some(new_value.clone());
            let observers: Vec<Rc<dyn IObserver<TResult>>> =
                self.observers.borrow().iter().map(|(_, o)| o.clone()).collect();
            match observers.len() {
                0 => {}
                1 => observers[0].on_next(new_value),
                _ => {
                    for observer in observers {
                        observer.on_next(new_value.clone());
                    }
                }
            }
        }
    }

    fn remove(&self, id: u64) {
        let now_empty = {
            let mut observers = self.observers.borrow_mut();
            let before = observers.len();
            observers.retain(|(i, _)| *i != id);
            before != observers.len() && observers.is_empty()
        };
        if now_empty {
            self.deinitialize();
        }
    }
}

impl<TResult: Clone + 'static> IObservable<TResult> for PropertyObservable<TResult> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TResult>>) -> Rc<dyn IDisposable> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let first = {
            let mut observers = self.observers.borrow_mut();
            observers.push((id, observer.clone()));
            observers.len() == 1
        };
        if first {
            self.initialize();
        }
        let current = self.value.borrow().clone();
        if let Some(value) = current {
            observer.on_next(value);
        }
        match self.this.upgrade() {
            Some(this) => Disposable::create(move || this.remove(id)),
            None => Disposable::empty(),
        }
    }
}
