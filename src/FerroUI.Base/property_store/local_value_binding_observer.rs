use super::BindingSource;
use crate::data::{BindingError, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObserver, ObservableError, Subscription};
use crate::{BoxedValue, DirectPropertyBase, FerroObject, FerroProperty, PropertyValue, Ref, StyledProperty, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The source keeps its observer weakly. The value store holds the observer
/// for the binding lifetime, and its subscription guard detaches on drop.
struct LocalBindingState {
    // Release the source while the remaining observer fields are still alive.
    subscription: RefCell<Option<Subscription>>,
    owner: WeakRef<FerroObject>,
    has_data_validation: bool,
}

enum ClearValidation {
    BeforeRemoval,
    AfterRemoval,
    Keep,
}

impl LocalBindingState {
    fn new(owner: &FerroObject, has_data_validation: bool) -> Self {
        Self { subscription: RefCell::new(None), owner: owner.to_weak(), has_data_validation }
    }

    fn started(&self, subscription: Rc<dyn IDisposable>) {
        self.subscription.replace(Some(Subscription::new(subscription)));
    }

    fn detach(&self) {
        drop(self.subscription.take());
    }

    fn validate(
        &self,
        owner: &FerroObject,
        property: &'static FerroProperty,
        kind: BindingValueType,
        error: Option<BindingError>,
    ) {
        if self.has_data_validation {
            owner.on_update_data_validation(property, kind, error);
        }
    }

    fn completed(&self, property: &'static FerroProperty, this: &dyn IDisposable, clear: ClearValidation) {
        let Some(owner) = self.owner.upgrade() else { return };
        if matches!(clear, ClearValidation::BeforeRemoval) {
            self.validate(&owner, property, BindingValueType::UNSET_VALUE, None);
        }
        owner.values().on_local_value_binding_completed(&owner, property, this);
        if matches!(clear, ClearValidation::AfterRemoval) {
            self.validate(&owner, property, BindingValueType::UNSET_VALUE, None);
        }
    }
}

/// Observes a binding source and writes its values to a styled property with
/// local value priority.
pub(crate) struct LocalValueBindingObserver<T: PropertyValue> {
    state: LocalBindingState,
    property: &'static StyledProperty<T>,
    default_value: RefCell<Option<T>>,
}

impl<T: PropertyValue> LocalValueBindingObserver<T> {
    pub fn new(owner: &FerroObject, property: &'static StyledProperty<T>) -> Rc<Self> {
        let has_data_validation = property.get_metadata_for(owner).enable_data_validation().unwrap_or(false);
        Rc::new(Self {
            state: LocalBindingState::new(owner, has_data_validation),
            property,
            default_value: RefCell::new(None),
        })
    }

    pub fn start(self: &Rc<Self>, source: BindingSource<T>) {
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(Typed(Rc::downgrade(self)))),
            BindingSource::Value(source) => source.subscribe(Rc::new(Value(Rc::downgrade(self)))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(Untyped(Rc::downgrade(self)))),
        };
        self.state.started(subscription);
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
        self.state.completed(self.property, self, ClearValidation::BeforeRemoval);
    }

    fn next(&self, mut value: T) {
        let Some(owner) = self.state.owner.upgrade() else { return };
        if !self.property.is_valid(&value) {
            value = self.cached_default_value(&owner);
        }
        owner.values().set_local_value(&owner, self.property, value);
        self.state.validate(&owner, self.property, BindingValueType::VALUE, None);
    }

    fn next_value(&self, value: BindingValue<T>) {
        let kind = value.value_type();
        if kind == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(owner) = self.state.owner.upgrade() else { return };
        let (value, error) = value.into_value_and_error();
        let value = match value {
            Some(v) if self.property.is_valid(&v) => Some(v),
            None if kind == BindingValueType::DATA_VALIDATION_ERROR => None,
            _ => Some(self.cached_default_value(&owner)),
        };
        if let Some(v) = value {
            owner.values().set_local_value(&owner, self.property, v);
        }
        self.state.validate(&owner, self.property, kind, error);
    }
}

impl<T: PropertyValue> IDisposable for LocalValueBindingObserver<T> {
    fn dispose(&self) {
        self.state.detach();
        self.completed();
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
    state: LocalBindingState,
    property: &'static DirectPropertyBase<T>,
    /// Whether the source produces untyped values.
    is_untyped: Cell<bool>,
}

impl<T: PropertyValue> DirectBindingObserver<T> {
    pub fn new(owner: &FerroObject, property: &'static DirectPropertyBase<T>) -> Rc<Self> {
        let has_data_validation = property.get_metadata_for(owner).enable_data_validation().unwrap_or(false);
        Rc::new(Self {
            state: LocalBindingState::new(owner, has_data_validation),
            property,
            is_untyped: Cell::new(false),
        })
    }

    pub fn start(self: &Rc<Self>, source: BindingSource<T>) {
        self.is_untyped.set(matches!(source, BindingSource::Untyped(_)));
        let subscription = match source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(DirectTyped(Rc::downgrade(self)))),
            BindingSource::Value(source) => source.subscribe(Rc::new(DirectValue(Rc::downgrade(self)))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(DirectUntyped(Rc::downgrade(self)))),
        };
        self.state.started(subscription);
    }

    /// `disposing` is true when the binding is disposed, false when its
    /// source completes or fails.
    fn completed(&self, disposing: bool) {
        // Untyped direct bindings clear validation only on explicit disposal.
        let clear = if disposing || !self.is_untyped.get() {
            ClearValidation::AfterRemoval
        } else {
            ClearValidation::Keep
        };
        self.state.completed(self.property.as_property(), self, clear);
    }

    fn next_value(&self, value: BindingValue<T>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        if let Some(owner) = self.state.owner.upgrade() {
            owner.set_direct_value_unchecked(self.property, value);
        }
    }
}

impl<T: PropertyValue> IDisposable for DirectBindingObserver<T> {
    fn dispose(&self) {
        self.state.detach();
        self.completed(true);
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
