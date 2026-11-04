use super::binding_entry::{invalid_value_error, BindingEntryState};
use super::{IValueEntry, ValueFrame};
use crate::data::{BindingError, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledProperty};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The typed cache behind the shared boxed-source installation path.
/// Observer construction is erased; delivery remains a typed weak upgrade.
pub(crate) trait BoxedBindingValue: IValueEntry + IDisposable {
    fn binding(&self) -> &BoxedBindingEntry;
    fn observer(&self) -> Rc<dyn IObserver<BoxedValue>>;
}

impl dyn BoxedBindingValue {
    pub(crate) fn start(&self) {
        self.binding().start_core(self, true);
    }
}

/// The subscription and source do not depend on the property's value type.
pub(crate) struct BoxedBindingEntry {
    // Dispose the subscription while its source and typed cache are alive.
    state: BindingEntryState,
    source: Rc<dyn IObservable<BoxedValue>>,
}

impl BoxedBindingEntry {
    pub(crate) fn new<T: PropertyValue>(
        target: &FerroObject,
        frame: Weak<dyn ValueFrame>,
        property: &'static StyledProperty<T>,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn BoxedBindingValue> {
        let has_data_validation = property.get_metadata_for(target).enable_data_validation() == Some(true);
        Rc::new_cyclic(|this| Value {
            binding: Self::new_state(frame, source, has_data_validation),
            this: this.clone(),
            property,
            value: RefCell::new(None),
            default_value: RefCell::new(None),
        })
    }

    fn new_state(
        frame: Weak<dyn ValueFrame>,
        source: Rc<dyn IObservable<BoxedValue>>,
        has_data_validation: bool,
    ) -> Self {
        Self { state: BindingEntryState::new(frame, has_data_validation), source }
    }

    #[inline]
    fn start_core(&self, entry: &dyn BoxedBindingValue, publish: bool) {
        if self.state.begin(publish) {
            self.subscribe(entry);
        }
    }

    #[inline(never)]
    fn subscribe(&self, entry: &dyn BoxedBindingValue) {
        let subscription = self.source.subscribe(entry.observer());
        self.state.finish(subscription);
    }

    fn completed(&self, property: &'static FerroProperty) {
        self.state.completed(property);
    }

    fn dispose(&self, property: &'static FerroProperty) {
        self.state.unsubscribe();
        self.completed(property);
    }
}

struct Observer<T: PropertyValue>(Weak<Value<T>>);

impl<T: PropertyValue> IObserver<BoxedValue> for Observer<T> {
    fn on_next(&self, value: BoxedValue) {
        if let Some(entry) = self.0.upgrade() {
            // Keep the owned input in this scope. Its destructor may reenter
            // a retained source observer after delivery disposes the binding.
            // The upgraded entry must drop first, on return and on unwind.
            entry.receive(&value);
        }
    }

    fn on_error(&self, _error: ObservableError) {
        self.on_completed();
    }

    fn on_completed(&self) {
        if let Some(entry) = self.0.upgrade() {
            entry.binding.completed(entry.property);
        }
    }
}

struct Value<T: PropertyValue> {
    binding: BoxedBindingEntry,
    this: Weak<Self>,
    property: &'static StyledProperty<T>,
    value: RefCell<Option<T>>,
    default_value: RefCell<Option<T>>,
}

impl<T: PropertyValue> BoxedBindingValue for Value<T> {
    fn binding(&self) -> &BoxedBindingEntry {
        &self.binding
    }

    fn observer(&self) -> Rc<dyn IObserver<BoxedValue>> {
        Rc::new(Observer(self.this.clone()))
    }
}

impl<T: PropertyValue> Value<T> {
    fn receive(self: &Rc<Self>, value: &BoxedValue) {
        let mut value = self.property.from_untyped(value.as_any());
        if value.has_value() && !self.property.is_valid(value.value()) {
            value = BindingValue::binding_error(invalid_value_error(self.property));
        }
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(bound) = self.binding.state.bound_frame() else { return };

        if !value.has_value() && value.value_type() != BindingValueType::DATA_VALIDATION_ERROR {
            value = value.with_value(self.property.binding_default(&bound.owner, &self.default_value));
        }

        self.binding.state.set_validation(value.value_type(), value.error());

        if value.has_value() {
            let changed = self.value.borrow().as_ref() != Some(value.value());
            if changed {
                // Release the borrow before dropping the previous T: its
                // destructor may read or update this property reentrantly.
                self.value.replace(value.into_option());
                if self.binding.state.publishes() {
                    bound.publish(self.clone());
                }
            }
        }
    }
}

impl<T: PropertyValue> IValueEntry for Value<T> {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        self.binding.start_core(self, false);
        self.value.borrow().is_some()
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        self.binding.start_core(self, false);
        match (out.downcast_mut::<Option<T>>(), &*self.value.borrow()) {
            (Some(out), Some(value)) => {
                *out = Some(value.clone());
                true
            }
            _ => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        self.binding.start_core(self, false);
        self.value.borrow().as_ref().map(|v| Rc::new(v.clone()) as BoxedValue)
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        self.binding.state.validation()
    }

    fn unsubscribe(&self) {
        self.binding.state.unsubscribe();
    }
}

impl<T: PropertyValue> IDisposable for Value<T> {
    fn dispose(&self) {
        self.binding.dispose(self.property);
    }
}
