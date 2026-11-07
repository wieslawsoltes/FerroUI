use super::binding_entry::{invalid_value_error, BindingEntryState};
use super::{IValueEntry, ValueFrame};
use crate::data::{BindingError, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::styled_property::resolve_boxed_binding_value;
use crate::{BoxedValue, FerroObject, FerroProperty};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A value entry in a value frame whose value is produced by an observable of
/// untyped values: a binding of a style setter or of markup.
///
/// The entry is not typed: it keeps its value in a box of its own, which holds
/// exactly the property's value type and is never handed out. The operations
/// that name the type (unpacking a typed binding value, validation, the
/// default value, copying the value) are the routes of the property, so the
/// entry and its observer are compiled once instead of once per value type.
/// Delivery, state transitions and the copies made of a value are those of the
/// typed [`BindingEntry`](super::BindingEntry).
pub(crate) struct BoxedBindingEntry {
    // Drop the subscription before any of the objects its callbacks can use.
    state: BindingEntryState,
    this: Weak<BoxedBindingEntry>,
    property: &'static FerroProperty,
    source: Rc<dyn IObservable<BoxedValue>>,
    value: RefCell<Option<BoxedValue>>,
    default_value: RefCell<Option<BoxedValue>>,
}

impl BoxedBindingEntry {
    /// Creates the entry for `frame`. Panics for a direct property, which has
    /// no value entries.
    pub(crate) fn new(
        target: &FerroObject,
        frame: Weak<dyn ValueFrame>,
        property: &'static FerroProperty,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<Self> {
        assert!(!property.is_direct(), "direct properties have no value entries");
        let has_data_validation = property.routes().route_enable_data_validation(target) == Some(true);
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            state: BindingEntryState::new(frame, has_data_validation),
            property,
            source,
            value: RefCell::new(None),
            default_value: RefCell::new(None),
        })
    }

    /// Subscribes to the source if not already subscribed, publishing the
    /// values it produces.
    pub(crate) fn start(&self) {
        self.start_core(true)
    }

    #[inline]
    fn start_core(&self, produce_value: bool) {
        if self.state.begin(produce_value) {
            self.subscribe();
        }
    }

    // Active entries only read the phase when the store asks for a value.
    #[inline(never)]
    fn subscribe(&self) {
        // The observer holds the entry weakly (DEVIATIONS.md, Property
        // system): the entry holds its source, and the source its observers.
        let observer = Rc::new(BoxedBindingObserver(self.this.clone()));
        let subscription = self.source.subscribe(observer);
        self.state.finish(subscription);
    }

    /// A copy of `value`, which holds exactly the property's value type, in a
    /// box of the entry's own.
    fn snapshot(&self, value: &BoxedValue) -> BoxedValue {
        self.property.routes().route_box_value(value.as_any()).expect("the value holds the property's value type")
    }

    /// Interprets the value, then validates it: a value that fails validation
    /// becomes a binding error, as in the typed entry.
    fn receive(self: &Rc<Self>, input: &BoxedValue) {
        let mut value = resolve_boxed_binding_value(self.property, input);
        // The entry keeps a snapshot, as the typed entry keeps a clone: never
        // the box it was given, which the source may share and whose value
        // may change. The other boxes of the resolution are copies already.
        if value.has_value() && Rc::ptr_eq(value.value(), input) {
            value = BindingValue::new(self.snapshot(input));
        }
        let value = if value.has_value() && !self.property.is_valid_value(value.value().as_any()) {
            BindingValue::binding_error(invalid_value_error(self.property))
        } else {
            value
        };
        self.set_value(value);
    }

    fn set_value(self: &Rc<Self>, mut value: BindingValue<BoxedValue>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(bound) = self.state.bound_frame() else { return };

        if !value.has_value() && value.value_type() != BindingValueType::DATA_VALIDATION_ERROR {
            let default_value = boxed_binding_default(self.property, &bound.owner, &self.default_value);
            value = value.with_value(self.snapshot(&default_value));
        }

        self.state.set_validation(value.value_type(), value.error());

        if value.has_value() {
            let changed = !matches!(&*self.value.borrow(), Some(current) if **current == **value.value());
            if changed {
                // `replace` releases the borrow before the previous value is
                // dropped: dropping a value may run code that reads the property.
                self.value.replace(value.into_option());
                if self.state.publishes() {
                    bound.publish(self.clone());
                }
            }
        }
    }

    fn binding_completed(&self) {
        self.state.completed(self.property);
    }
}

/// The fallback value of an untyped binding: the default value of the property
/// for the type of `owner`. A binding keeps the first fallback it observes for
/// its owner, even if the property's metadata is subsequently overridden (the
/// boxed form of `StyledProperty::binding_default`).
pub(super) fn boxed_binding_default(
    property: &'static FerroProperty,
    owner: &FerroObject,
    cache: &RefCell<Option<BoxedValue>>,
) -> BoxedValue {
    if let Some(value) = &*cache.borrow() {
        return value.clone();
    }
    let value = property.routes().route_get_default_value(owner.get_type());
    *cache.borrow_mut() = Some(value.clone());
    value
}

impl IValueEntry for BoxedBindingEntry {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        self.start_core(false);
        self.value.borrow().is_some()
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        self.start_core(false);
        match &*self.value.borrow() {
            Some(value) => self.property.routes().route_copy_value(value.as_any(), out),
            None => false,
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        self.start_core(false);
        self.value.borrow().as_ref().map(|value| self.snapshot(value))
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        self.state.validation()
    }

    fn unsubscribe(&self) {
        self.state.unsubscribe();
    }
}

impl IDisposable for BoxedBindingEntry {
    fn dispose(&self) {
        self.unsubscribe();
        self.binding_completed();
    }
}

struct BoxedBindingObserver(Weak<BoxedBindingEntry>);

impl IObserver<BoxedValue> for BoxedBindingObserver {
    fn on_next(&self, value: BoxedValue) {
        let Some(entry) = self.0.upgrade() else { return };
        entry.receive(&value);
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
