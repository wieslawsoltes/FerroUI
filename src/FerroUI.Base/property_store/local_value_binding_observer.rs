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
        let this = self.this.upgrade().expect("observer is alive");
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(Typed(this))),
            BindingSource::Value(source) => source.subscribe(Rc::new(Value(this))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(Untyped(this))),
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

struct Typed<T: PropertyValue>(Rc<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<T> for Typed<T> {
    fn on_next(&self, value: T) {
        self.0.next(value)
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed()
    }
    fn on_completed(&self) {
        self.0.completed()
    }
}

struct Value<T: PropertyValue>(Rc<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BindingValue<T>> for Value<T> {
    fn on_next(&self, value: BindingValue<T>) {
        self.0.next_value(value)
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed()
    }
    fn on_completed(&self) {
        self.0.completed()
    }
}

struct Untyped<T: PropertyValue>(Rc<LocalValueBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BoxedValue> for Untyped<T> {
    fn on_next(&self, value: BoxedValue) {
        self.0.next_value(self.0.property.from_untyped(value.as_any()))
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed()
    }
    fn on_completed(&self) {
        self.0.completed()
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
        let this = self.this.upgrade().expect("observer is alive");
        self.is_untyped.set(matches!(source, BindingSource::Untyped(_)));
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(DirectTyped(this))),
            BindingSource::Value(source) => source.subscribe(Rc::new(DirectValue(this))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(DirectUntyped(this))),
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

struct DirectTyped<T: PropertyValue>(Rc<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<T> for DirectTyped<T> {
    fn on_next(&self, value: T) {
        self.0.next_value(BindingValue::new(value))
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed(false)
    }
    fn on_completed(&self) {
        self.0.completed(false)
    }
}

struct DirectValue<T: PropertyValue>(Rc<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BindingValue<T>> for DirectValue<T> {
    fn on_next(&self, value: BindingValue<T>) {
        self.0.next_value(value)
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed(false)
    }
    fn on_completed(&self) {
        self.0.completed(false)
    }
}

struct DirectUntyped<T: PropertyValue>(Rc<DirectBindingObserver<T>>);
impl<T: PropertyValue> IObserver<BoxedValue> for DirectUntyped<T> {
    fn on_next(&self, value: BoxedValue) {
        use crate::{DoNothingType, UnsetValueType};
        let value = if let Some(v) = value.downcast_ref::<T>() {
            BindingValue::new(v.clone())
        } else if value.is::<UnsetValueType>() {
            BindingValue::unset()
        } else if value.is::<DoNothingType>() {
            BindingValue::do_nothing()
        } else if let Some(v) = value.downcast_ref::<BindingValue<T>>() {
            v.clone()
        } else if let Some(n) = value.downcast_ref::<crate::data::BindingNotification>() {
            n.to_binding_value(self.0.property.property_type_name())
        } else {
            BindingValue::binding_error(crate::data::BindingError::message("Invalid value type."))
        };
        self.0.next_value(value)
    }
    fn on_error(&self, _error: ObservableError) {
        self.0.completed(false)
    }
    fn on_completed(&self) {
        self.0.completed(false)
    }
}
