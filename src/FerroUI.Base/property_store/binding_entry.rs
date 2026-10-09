use super::{IValueEntry, ValueFrame};
use crate::data::{BindingError, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledProperty};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The observable a binding entry or binding observer reads its values from.
pub(crate) enum BindingSource<T> {
    Typed(Rc<dyn IObservable<T>>),
    Value(Rc<dyn IObservable<BindingValue<T>>>),
    Untyped(Rc<dyn IObservable<BoxedValue>>),
}

enum Subscription {
    None,
    /// Subscribing; values produced during subscription are published.
    Creating,
    /// Subscribing; values produced during subscription are only stored.
    CreatingQuiet,
    Active(Rc<dyn IDisposable>),
}

/// A value entry in a value frame whose value is produced by an observable.
pub(crate) struct BindingEntry<T: PropertyValue> {
    this: Weak<BindingEntry<T>>,
    frame: Weak<dyn ValueFrame>,
    property: &'static StyledProperty<T>,
    source: BindingSource<T>,
    subscription: RefCell<Subscription>,
    value: RefCell<Option<T>>,
    has_data_validation: bool,
    data_validation_state: Cell<BindingValueType>,
    data_validation_error: RefCell<Option<BindingError>>,
    default_value: RefCell<Option<T>>,
}

impl<T: PropertyValue> BindingEntry<T> {
    pub fn new(
        target: &FerroObject,
        frame: Weak<dyn ValueFrame>,
        property: &'static StyledProperty<T>,
        source: BindingSource<T>,
    ) -> Rc<Self> {
        let has_data_validation = property.get_metadata_for(target).enable_data_validation() == Some(true);
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            frame,
            property,
            source,
            subscription: RefCell::new(Subscription::None),
            value: RefCell::new(None),
            has_data_validation,
            data_validation_state: Cell::new(BindingValueType::VALUE),
            data_validation_error: RefCell::new(None),
            default_value: RefCell::new(None),
        })
    }

    pub fn is_subscribed(&self) -> bool {
        !matches!(*self.subscription.borrow(), Subscription::None)
    }

    /// Subscribes to the source if not already subscribed, publishing the
    /// values it produces.
    pub fn start(&self) {
        self.start_core(true)
    }

    fn start_core(&self, produce_value: bool) {
        if self.is_subscribed() {
            return;
        }
        *self.subscription.borrow_mut() =
            if produce_value { Subscription::Creating } else { Subscription::CreatingQuiet };
        // The observers hold the entry weakly (DEVIATIONS.md, Property
        // system): the entry holds its source, and the source its observers.
        let this = self.this.clone();
        let subscription = match &self.source {
            BindingSource::Typed(source) => source.subscribe(Rc::new(TypedObserver(this))),
            BindingSource::Value(source) => source.subscribe(Rc::new(ValueObserver(this))),
            BindingSource::Untyped(source) => source.subscribe(Rc::new(UntypedObserver(this))),
        };
        let mut current = self.subscription.borrow_mut();
        // The source may have completed during subscription, in which case the
        // entry is already detached.
        if matches!(*current, Subscription::None) {
            drop(current);
            subscription.dispose();
        } else {
            *current = Subscription::Active(subscription);
        }
    }

    /// A value that fails validation becomes a binding error; the entry then
    /// falls back to the default value and reports the error to data
    /// validation.
    fn validate(&self, value: T) -> BindingValue<T> {
        if self.property.is_valid(&value) {
            BindingValue::new(value)
        } else {
            self.invalid_value()
        }
    }

    fn convert_and_validate(&self, value: BindingValue<T>) -> BindingValue<T> {
        if value.has_value() && !self.property.is_valid(value.value()) {
            return self.invalid_value();
        }
        value
    }

    fn invalid_value(&self) -> BindingValue<T> {
        BindingValue::binding_error(invalid_value_error(self.property))
    }

    fn cached_default_value(&self) -> T {
        if let Some(v) = &*self.default_value.borrow() {
            return v.clone();
        }
        let owner = self.frame.upgrade().and_then(|f| f.base().owner());
        let value = match owner {
            Some(owner) => self.property.get_default_value_for(&owner),
            None => self.property.get_default_value(self.property.owner_type()),
        };
        *self.default_value.borrow_mut() = Some(value.clone());
        value
    }

    fn set_value(&self, mut value: BindingValue<T>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(frame) = self.frame.upgrade() else { return };
        let Some(owner) = frame.base().owner() else { return };

        if !value.has_value() && value.value_type() != BindingValueType::DATA_VALIDATION_ERROR {
            value = value.with_value(self.cached_default_value());
        }

        if self.has_data_validation {
            self.data_validation_state.set(value.value_type());
            *self.data_validation_error.borrow_mut() = value.error().cloned();
        }

        if value.has_value() {
            let changed = self.value.borrow().as_ref() != Some(value.value());
            if changed {
                // `replace` releases the borrow before the previous value is
                // dropped: dropping a value may run code that reads the property.
                self.value.replace(value.into_option());
                let publish = matches!(
                    *self.subscription.borrow(),
                    Subscription::Creating | Subscription::Active(_)
                );
                if publish {
                    let entry: Rc<dyn IValueEntry> = self.this.upgrade().expect("binding entry is alive");
                    owner.values().on_binding_value_changed(&owner, &entry, frame.base().priority());
                }
            }
        }
    }

    fn binding_completed(&self) {
        *self.subscription.borrow_mut() = Subscription::None;
        if let Some(frame) = self.frame.upgrade() {
            frame.on_binding_completed(self.property);
        }
    }
}

impl<T: PropertyValue> IValueEntry for BindingEntry<T> {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        self.start_core(false);
        self.value.borrow().is_some()
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        self.start_core(false);
        match (out.downcast_mut::<Option<T>>(), &*self.value.borrow()) {
            (Some(out), Some(value)) => {
                *out = Some(value.clone());
                true
            }
            _ => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        self.start_core(false);
        self.value.borrow().as_ref().map(|v| Rc::new(v.clone()) as BoxedValue)
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        self.has_data_validation
            .then(|| (self.data_validation_state.get(), self.data_validation_error.borrow().clone()))
    }

    fn unsubscribe(&self) {
        let subscription = std::mem::replace(&mut *self.subscription.borrow_mut(), Subscription::None);
        if let Subscription::Active(s) = subscription {
            s.dispose();
        }
    }
}

impl<T: PropertyValue> IDisposable for BindingEntry<T> {
    fn dispose(&self) {
        self.unsubscribe();
        self.binding_completed();
    }
}

/// The error of a value that fails the validation of the property (compiled
/// once, not per value type).
#[cold]
fn invalid_value_error(property: &FerroProperty) -> BindingError {
    BindingError::message(format!("The value is not valid for property '{}'.", property.name()))
}

/// An entry that is dropped while it is subscribed (its frame was dropped
/// with the value store of an object that is gone) leaves its source: the
/// collector does this for the managed original.
impl<T: PropertyValue> Drop for BindingEntry<T> {
    fn drop(&mut self) {
        if let Subscription::Active(subscription) = std::mem::replace(self.subscription.get_mut(), Subscription::None) {
            subscription.dispose();
        }
    }
}

struct TypedObserver<T: PropertyValue>(Weak<BindingEntry<T>>);

impl<T: PropertyValue> IObserver<T> for TypedObserver<T> {
    fn on_next(&self, value: T) {
        let Some(entry) = self.0.upgrade() else { return };
        let value = entry.validate(value);
        entry.set_value(value);
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
    fn on_completed(&self) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
}

struct ValueObserver<T: PropertyValue>(Weak<BindingEntry<T>>);

impl<T: PropertyValue> IObserver<BindingValue<T>> for ValueObserver<T> {
    fn on_next(&self, value: BindingValue<T>) {
        let Some(entry) = self.0.upgrade() else { return };
        let value = entry.convert_and_validate(value);
        entry.set_value(value);
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
    fn on_completed(&self) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
}

struct UntypedObserver<T: PropertyValue>(Weak<BindingEntry<T>>);

impl<T: PropertyValue> IObserver<BoxedValue> for UntypedObserver<T> {
    fn on_next(&self, value: BoxedValue) {
        let Some(entry) = self.0.upgrade() else { return };
        let value = entry.property.from_untyped(value.as_any());
        let value = entry.convert_and_validate(value);
        entry.set_value(value);
    }
    fn on_error(&self, _error: ObservableError) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
    fn on_completed(&self) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding_completed()
        }
    }
}
