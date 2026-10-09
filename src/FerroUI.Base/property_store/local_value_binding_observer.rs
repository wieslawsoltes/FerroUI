use super::BindingSource;
use crate::data::{BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObserver, ObservableError};
use crate::{BoxedValue, DirectPropertyBase, FerroObject, PropertyValue, Ref, StyledProperty, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Observes a binding source and writes its values to a styled property with
/// local value priority.
pub(crate) struct LocalValueBindingObserver<T: PropertyValue> {
    this: Weak<LocalValueBindingObserver<T>>,
    owner: WeakRef<FerroObject>,
    property: &'static StyledProperty<T>,
    has_data_validation: bool,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    default_value: RefCell<Option<T>>,
}

impl<T: PropertyValue> LocalValueBindingObserver<T> {
    pub fn new(owner: &FerroObject, property: &'static StyledProperty<T>) -> Rc<Self> {
        let has_data_validation = property.get_metadata_for(owner).enable_data_validation().unwrap_or(false);
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner: owner.to_weak(),
            property,
            has_data_validation,
            subscription: RefCell::new(None),
            default_value: RefCell::new(None),
        })
    }

    pub fn start(&self, source: BindingSource<T>) {
        let this = &self.this;
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(Typed(this.clone()))),
            BindingSource::Value(source) => source.subscribe(Rc::new(Value(this.clone()))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(Untyped(this.clone()))),
        };
        *self.subscription.borrow_mut() = Some(subscription);
    }

    fn cached_default_value(&self, owner: &Ref<FerroObject>) -> T {
        if let Some(v) = &*self.default_value.borrow() {
            return v.clone();
        }
        let value = self.property.get_default_value_for(owner);
        *self.default_value.borrow_mut() = Some(value.clone());
        value
    }

    fn completed(&self) {
        let Some(owner) = self.owner.upgrade() else { return };
        if self.has_data_validation {
            owner.on_update_data_validation(self.property, BindingValueType::UNSET_VALUE, None);
        }
        let this: Rc<dyn IDisposable> = self.this.upgrade().expect("observer is alive");
        owner.values().on_local_value_binding_completed(&owner, self.property, &this);
    }

    fn next(&self, mut value: T) {
        let Some(owner) = self.owner.upgrade() else { return };
        if !self.property.is_valid(&value) {
            value = self.cached_default_value(&owner);
        }
        owner.values().set_local_value(&owner, self.property, value);
        if self.has_data_validation {
            owner.on_update_data_validation(self.property, BindingValueType::VALUE, None);
        }
    }

    fn next_value(&self, mut value: BindingValue<T>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(owner) = self.owner.upgrade() else { return };
        let original_type = value.value_type();
        // Revert to the default value if the binding value fails validation, or
        // if there was no value (though not if there was a data validation error).
        if (value.has_value() && !self.property.is_valid(value.value()))
            || (!value.has_value() && value.value_type() != BindingValueType::DATA_VALIDATION_ERROR)
        {
            value = value.with_value(self.cached_default_value(&owner));
        }
        let error = value.error().cloned();
        if let Some(v) = value.into_option() {
            owner.values().set_local_value(&owner, self.property, v);
        }
        if self.has_data_validation {
            owner.on_update_data_validation(self.property, original_type, error);
        }
    }
}

impl<T: PropertyValue> IDisposable for LocalValueBindingObserver<T> {
    fn dispose(&self) {
        if let Some(s) = self.subscription.take() {
            s.dispose();
        }
        self.completed();
    }
}

// The source holds its observers, and the observer its subscription to the
// source: in the managed original the observer is the object that subscribes
// itself, and the collector frees the two when the object of the value store
// is gone, the binding disposed or not. Here the observers the source holds
// are objects of their own that hold this one weakly, the value store holds
// it for as long as the binding lasts, and it leaves its source when it is
// dropped with the store (as `BindingEntry` does).
impl<T: PropertyValue> Drop for LocalValueBindingObserver<T> {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

struct Typed<T: PropertyValue>(Weak<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<T> for Typed<T> {
    fn on_next(&self, value: T) {
        if let Some(observer) = self.0.upgrade() {
            observer.next(value)
        }
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
}

struct Value<T: PropertyValue>(Weak<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BindingValue<T>> for Value<T> {
    fn on_next(&self, value: BindingValue<T>) {
        if let Some(observer) = self.0.upgrade() {
            observer.next_value(value)
        }
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
}

struct Untyped<T: PropertyValue>(Weak<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BoxedValue> for Untyped<T> {
    fn on_next(&self, value: BoxedValue) {
        if let Some(observer) = self.0.upgrade() {
            observer.next_value(observer.property.from_untyped(value.as_any()))
        }
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed()
        }
    }
}

/// Observes a binding source and writes its values to a direct property.
pub(crate) struct DirectBindingObserver<T: PropertyValue> {
    this: Weak<DirectBindingObserver<T>>,
    owner: WeakRef<FerroObject>,
    property: &'static DirectPropertyBase<T>,
    has_data_validation: bool,
    /// Whether the source produces untyped values.
    is_untyped: Cell<bool>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl<T: PropertyValue> DirectBindingObserver<T> {
    pub fn new(owner: &FerroObject, property: &'static DirectPropertyBase<T>) -> Rc<Self> {
        let has_data_validation = property.get_metadata_for(owner).enable_data_validation().unwrap_or(false);
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner: owner.to_weak(),
            property,
            has_data_validation,
            is_untyped: Cell::new(false),
            subscription: RefCell::new(None),
        })
    }

    pub fn start(&self, source: BindingSource<T>) {
        let this = &self.this;
        self.is_untyped.set(matches!(source, BindingSource::Untyped(_)));
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(DirectTyped(this.clone()))),
            BindingSource::Value(source) => source.subscribe(Rc::new(DirectValue(this.clone()))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(DirectUntyped(this.clone()))),
        };
        *self.subscription.borrow_mut() = Some(subscription);
    }

    /// `disposing` is true when the binding is disposed, false when its
    /// source completes or fails.
    fn completed(&self, disposing: bool) {
        let Some(owner) = self.owner.upgrade() else { return };
        let this: Rc<dyn IDisposable> = self.this.upgrade().expect("observer is alive");
        owner.values().on_local_value_binding_completed(&owner, self.property, &this);
        // As upstream: a binding to an untyped source only clears the data
        // validation state when it is disposed, not when its source completes.
        if self.has_data_validation && (disposing || !self.is_untyped.get()) {
            owner.on_update_data_validation(self.property.as_property(), BindingValueType::UNSET_VALUE, None);
        }
    }

    fn next_value(&self, value: BindingValue<T>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        if let Some(owner) = self.owner.upgrade() {
            owner.set_direct_value_unchecked(self.property, value);
        }
    }
}

impl<T: PropertyValue> IDisposable for DirectBindingObserver<T> {
    fn dispose(&self) {
        if let Some(s) = self.subscription.take() {
            s.dispose();
        }
        self.completed(true);
    }
}

// As `LocalValueBindingObserver`: the source holds this observer weakly, and
// it leaves its source when it is dropped with its value store.
impl<T: PropertyValue> Drop for DirectBindingObserver<T> {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

struct DirectTyped<T: PropertyValue>(Weak<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<T> for DirectTyped<T> {
    fn on_next(&self, value: T) {
        if let Some(observer) = self.0.upgrade() {
            observer.next_value(BindingValue::new(value))
        }
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
}

struct DirectValue<T: PropertyValue>(Weak<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BindingValue<T>> for DirectValue<T> {
    fn on_next(&self, value: BindingValue<T>) {
        if let Some(observer) = self.0.upgrade() {
            observer.next_value(value)
        }
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
}

struct DirectUntyped<T: PropertyValue>(Weak<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BoxedValue> for DirectUntyped<T> {
    fn on_next(&self, value: BoxedValue) {
        use crate::{DoNothingType, UnsetValueType};
        let Some(observer) = self.0.upgrade() else { return };
        let value = if let Some(v) = value.downcast_ref::<T>() {
            BindingValue::new(v.clone())
        } else if value.is::<UnsetValueType>() {
            BindingValue::unset()
        } else if value.is::<DoNothingType>() {
            BindingValue::do_nothing()
        } else if let Some(v) = value.downcast_ref::<BindingValue<T>>() {
            v.clone()
        } else if let Some(n) = value.downcast_ref::<crate::data::BindingNotification>() {
            n.to_binding_value(observer.property.property_type_name())
        } else {
            BindingValue::binding_error(crate::data::BindingError::message("Invalid value type."))
        };
        observer.next_value(value)
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
    fn on_completed(&self) {
        if let Some(observer) = self.0.upgrade() {
            observer.completed(false)
        }
    }
}
