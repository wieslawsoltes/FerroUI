use super::local_value_binding_observer::{ClearValidation, LocalBindingState};
use crate::data::BindingValueType;
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledProperty};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Only cold installation needs dynamic dispatch; each event upgrades a
/// typed weak reference and updates the original inline value cache.
pub(crate) trait BoxedLocalBindingValue: IDisposable {
    fn binding(&self) -> &BoxedLocalValueBindingObserver;
    fn observer(self: Rc<Self>) -> Rc<dyn IObserver<BoxedValue>>;
}

impl dyn BoxedLocalBindingValue {
    pub(crate) fn start(self: &Rc<Self>, source: Rc<dyn IObservable<BoxedValue>>) {
        let observer = self.clone().observer();
        self.binding().state.started(source.subscribe(observer));
    }
}

pub(crate) struct BoxedLocalValueBindingObserver {
    state: LocalBindingState,
}

impl BoxedLocalValueBindingObserver {
    pub(crate) fn new<T: PropertyValue>(
        owner: &FerroObject,
        property: &'static StyledProperty<T>,
    ) -> Rc<dyn BoxedLocalBindingValue> {
        Rc::new(Value { binding: Self::new_state(owner, property), property, default_value: RefCell::new(None) })
    }

    fn new_state(owner: &FerroObject, property: &'static FerroProperty) -> Self {
        let has_data_validation = property.get_metadata_for(owner).enable_data_validation().unwrap_or(false);
        Self { state: LocalBindingState::new(owner, has_data_validation) }
    }

    fn completed(&self, property: &'static FerroProperty, this: &dyn IDisposable) {
        self.state.completed(property, this, ClearValidation::BeforeRemoval);
    }

    fn dispose(&self, property: &'static FerroProperty, this: &dyn IDisposable) {
        self.state.detach();
        self.completed(property, this);
    }
}

struct Observer<T: PropertyValue>(Weak<Value<T>>);

impl<T: PropertyValue> IObserver<BoxedValue> for Observer<T> {
    fn on_next(&self, value: BoxedValue) {
        if let Some(binding) = self.0.upgrade() {
            // Retain the owned input until the upgraded binding drops. An
            // input destructor can reenter this observer, also on unwind.
            binding.receive(&value);
        }
    }

    fn on_error(&self, _error: ObservableError) {
        self.on_completed();
    }

    fn on_completed(&self) {
        if let Some(binding) = self.0.upgrade() {
            binding.binding.completed(binding.property, &*binding);
        }
    }
}

struct Value<T: PropertyValue> {
    // Dispose the subscription before releasing the cached fallback value.
    binding: BoxedLocalValueBindingObserver,
    property: &'static StyledProperty<T>,
    default_value: RefCell<Option<T>>,
}

impl<T: PropertyValue> BoxedLocalBindingValue for Value<T> {
    fn binding(&self) -> &BoxedLocalValueBindingObserver {
        &self.binding
    }

    fn observer(self: Rc<Self>) -> Rc<dyn IObserver<BoxedValue>> {
        let this = Rc::downgrade(&self);
        // Installation's temporary Rc is needed only to construct the typed
        // weak observer. Release it before allocating or subscribing: a source
        // may deliver and dispose the binding synchronously during subscribe.
        drop(self);
        Rc::new(Observer(this))
    }
}

impl<T: PropertyValue> Value<T> {
    fn receive(&self, value: &BoxedValue) {
        // Conversion precedes the owner upgrade, as it can reenter user code
        // and release the owner. No input box is retained by the typed cache.
        let value = self.property.from_untyped(value.as_any());
        let kind = value.value_type();
        if kind == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(owner) = self.binding.state.owner.upgrade() else { return };
        let (value, error) = value.into_value_and_error();
        let value = match value {
            Some(v) if self.property.is_valid(&v) => Some(v),
            None if kind == BindingValueType::DATA_VALIDATION_ERROR => None,
            _ => Some(self.property.binding_default(&owner, &self.default_value)),
        };
        if let Some(v) = value {
            owner.values().set_local_value(&owner, self.property, v);
        }
        self.binding.state.validate(&owner, self.property, kind, error);
    }
}

impl<T: PropertyValue> IDisposable for Value<T> {
    fn dispose(&self) {
        self.binding.dispose(self.property, self);
    }
}
