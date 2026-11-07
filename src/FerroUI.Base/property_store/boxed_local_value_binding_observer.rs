use super::boxed_binding_entry::boxed_binding_default;
use super::local_value_binding_observer::{ClearValidation, LocalBindingState};
use crate::data::BindingValueType;
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::styled_property::resolve_boxed_binding_value;
use crate::{BoxedValue, FerroObject, FerroProperty};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Observes a binding source of untyped values and writes its values to a
/// styled property with local value priority.
///
/// The observer is not typed: values stay boxed, and the operations that name
/// the property's value type (unpacking a typed binding value, validation, the
/// default value, the typed write of the local value) are the routes of the
/// property. Delivery is that of the typed
/// [`LocalValueBindingObserver`](super::LocalValueBindingObserver) of a source
/// of binding values.
pub(crate) struct BoxedLocalValueBindingObserver {
    // Dispose the subscription before releasing the cached fallback value.
    state: LocalBindingState,
    property: &'static FerroProperty,
    default_value: RefCell<Option<BoxedValue>>,
}

impl BoxedLocalValueBindingObserver {
    pub(crate) fn new(owner: &FerroObject, property: &'static FerroProperty) -> Rc<Self> {
        // Release the metadata before capturing the owner and allocating the
        // observer, as typed local installation does.
        let has_data_validation = property.routes().route_enable_data_validation(owner).unwrap_or(false);
        Rc::new(Self {
            state: LocalBindingState::new(owner, has_data_validation),
            property,
            default_value: RefCell::new(None),
        })
    }

    pub(crate) fn start(self: &Rc<Self>, source: Rc<dyn IObservable<BoxedValue>>) {
        // No temporary strong reference is held while subscribing: a source may
        // deliver and dispose the binding synchronously during subscribe.
        let observer = Rc::new(Observer(Rc::downgrade(self)));
        self.state.started(source.subscribe(observer));
    }

    fn completed(&self) {
        self.state.completed(self.property, self, ClearValidation::BeforeRemoval);
    }

    fn receive(&self, value: &BoxedValue) {
        // Conversion precedes the owner upgrade, as it can reenter user code
        // and release the owner.
        let value = resolve_boxed_binding_value(self.property, value);
        let kind = value.value_type();
        if kind == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(owner) = self.state.owner.upgrade() else { return };
        let (value, error) = value.into_value_and_error();
        let value = match value {
            Some(v) if self.property.is_valid_value(v.as_any()) => Some(v),
            None if kind == BindingValueType::DATA_VALIDATION_ERROR => None,
            _ => Some(boxed_binding_default(self.property, &owner, &self.default_value)),
        };
        if let Some(v) = value {
            self.property.routes().route_set_local_value(&owner, v);
        }
        self.state.validate(&owner, self.property, kind, error);
    }
}

impl IDisposable for BoxedLocalValueBindingObserver {
    fn dispose(&self) {
        self.state.detach();
        self.completed();
    }
}

struct Observer(Weak<BoxedLocalValueBindingObserver>);

impl IObserver<BoxedValue> for Observer {
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
            binding.completed();
        }
    }
}
