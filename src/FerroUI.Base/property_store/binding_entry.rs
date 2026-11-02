use super::{IValueEntry, ValueFrame};
use crate::data::{BindingError, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, Ref, StyledProperty};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// A deferred frame entry retains the observable and the adapter chosen by the
// binding overload. A boxed-only property can resubscribe without retaining
// its typed and BindingValue observer implementations in the executable.
type EntryObserver<T, U> = fn(Weak<BindingEntry<T>>) -> Rc<dyn IObserver<U>>;

pub(crate) enum BindingSource<T: PropertyValue> {
    Typed(Rc<dyn IObservable<T>>, EntryObserver<T, T>),
    Value(Rc<dyn IObservable<BindingValue<T>>>, EntryObserver<T, BindingValue<T>>),
    Untyped(Rc<dyn IObservable<BoxedValue>>, EntryObserver<T, BoxedValue>),
}

impl<T: PropertyValue> BindingSource<T> {
    pub(crate) fn typed(source: Rc<dyn IObservable<T>>) -> Self {
        Self::Typed(source, |entry| Rc::new(TypedObserver(entry)))
    }

    pub(crate) fn value(source: Rc<dyn IObservable<BindingValue<T>>>) -> Self {
        Self::Value(source, |entry| Rc::new(ValueObserver(entry)))
    }

    pub(crate) fn untyped(source: Rc<dyn IObservable<BoxedValue>>) -> Self {
        Self::Untyped(source, |entry| Rc::new(UntypedObserver(entry)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubscriptionPhase {
    Idle,
    /// Subscribing; values produced during subscription are published.
    Creating,
    /// Subscribing; values produced during subscription are only stored.
    CreatingQuiet,
    Active,
}

/// Subscription and validation belong to the binding, regardless of its
/// property's Rust value type. Keep their reentrant transitions in one place.
struct BindingEntryState {
    // The phase is readable while subscribing or disposing synchronously.
    phase: Cell<SubscriptionPhase>,
    subscription: Cell<Option<Rc<dyn IDisposable>>>,
    frame: Weak<dyn ValueFrame>,
    has_data_validation: bool,
    data_validation_state: Cell<BindingValueType>,
    data_validation_error: RefCell<Option<BindingError>>,
}

struct BoundFrame {
    owner: Ref<FerroObject>,
    frame: Rc<dyn ValueFrame>,
}

impl BoundFrame {
    fn publish(&self, entry: Rc<dyn IValueEntry>) {
        self.owner.values().on_binding_value_changed(&self.owner, &entry, self.frame.base().priority());
    }
}

impl BindingEntryState {
    fn new(frame: Weak<dyn ValueFrame>, has_data_validation: bool) -> Self {
        Self {
            phase: Cell::new(SubscriptionPhase::Idle),
            subscription: Cell::new(None),
            frame,
            has_data_validation,
            data_validation_state: Cell::new(BindingValueType::VALUE),
            data_validation_error: RefCell::new(None),
        }
    }

    fn bound_frame(&self) -> Option<BoundFrame> {
        let frame = self.frame.upgrade()?;
        Some(BoundFrame { owner: frame.base().owner()?, frame })
    }

    fn begin(&self, publish: bool) -> bool {
        if self.phase.get() != SubscriptionPhase::Idle {
            return false;
        }
        self.phase.set(if publish { SubscriptionPhase::Creating } else { SubscriptionPhase::CreatingQuiet });
        true
    }

    fn finish(&self, subscription: Rc<dyn IDisposable>) {
        // A synchronous source may have completed during subscribe.
        if self.phase.get() == SubscriptionPhase::Idle {
            subscription.dispose();
        } else {
            self.phase.set(SubscriptionPhase::Active);
            self.subscription.set(Some(subscription));
        }
    }

    fn publishes(&self) -> bool {
        matches!(self.phase.get(), SubscriptionPhase::Creating | SubscriptionPhase::Active)
    }

    fn set_validation(&self, state: BindingValueType, error: Option<&BindingError>) {
        if self.has_data_validation {
            self.data_validation_state.set(state);
            *self.data_validation_error.borrow_mut() = error.cloned();
        }
    }

    fn validation(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        self.has_data_validation
            .then(|| (self.data_validation_state.get(), self.data_validation_error.borrow().clone()))
    }

    fn unsubscribe(&self) {
        self.phase.set(SubscriptionPhase::Idle);
        if let Some(s) = self.subscription.take() {
            s.dispose();
        }
    }

    fn completed(&self, property: &'static FerroProperty) {
        self.phase.set(SubscriptionPhase::Idle);
        drop(self.subscription.take());
        if let Some(frame) = self.frame.upgrade() {
            frame.on_binding_completed(property);
        }
    }
}

impl Drop for BindingEntryState {
    fn drop(&mut self) {
        if let Some(s) = self.subscription.get_mut().take() {
            s.dispose();
        }
    }
}

/// Construction and insertion into a frame can be shared for boxed observable
/// sources. The entry still subscribes and receives values with its typed
/// observer, including after synchronous completion or resubscription.
pub(crate) trait BoxedFrameBinding: IValueEntry + IDisposable {
    fn start_publishing(&self);
}

/// A value entry in a value frame whose value is produced by an observable.
pub(crate) struct BindingEntry<T: PropertyValue> {
    // Drop the subscription before any of the objects its callbacks can use.
    state: BindingEntryState,
    this: Weak<BindingEntry<T>>,
    property: &'static StyledProperty<T>,
    source: BindingSource<T>,
    value: RefCell<Option<T>>,
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
            state: BindingEntryState::new(frame, has_data_validation),
            property,
            source,
            value: RefCell::new(None),
            default_value: RefCell::new(None),
        })
    }

    /// Subscribes to the source if not already subscribed, publishing the
    /// values it produces.
    pub fn start(&self) {
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
        // The observers hold the entry weakly (DEVIATIONS.md, Property
        // system): the entry holds its source, and the source its observers.
        let this = self.this.clone();
        let subscription = match &self.source {
            BindingSource::Typed(source, observer) => source.subscribe(observer(this)),
            BindingSource::Value(source, observer) => source.subscribe(observer(this)),
            BindingSource::Untyped(source, observer) => source.subscribe(observer(this)),
        };
        self.state.finish(subscription);
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

    fn set_value(self: &Rc<Self>, mut value: BindingValue<T>) {
        if value.value_type() == BindingValueType::DO_NOTHING {
            return;
        }
        let Some(bound) = self.state.bound_frame() else { return };

        if !value.has_value() && value.value_type() != BindingValueType::DATA_VALIDATION_ERROR {
            value = value.with_value(self.property.binding_default(&bound.owner, &self.default_value));
        }

        self.state.set_validation(value.value_type(), value.error());

        if value.has_value() {
            let changed = self.value.borrow().as_ref() != Some(value.value());
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
        self.state.validation()
    }

    fn unsubscribe(&self) {
        self.state.unsubscribe();
    }
}

impl<T: PropertyValue> BoxedFrameBinding for BindingEntry<T> {
    fn start_publishing(&self) {
        self.start();
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
