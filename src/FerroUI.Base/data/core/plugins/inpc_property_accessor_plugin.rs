use super::{
    AccessorListener, IPropertyAccessor, IPropertyAccessorPlugin, PropertyAccessorBase, PropertyError, markup_members,
};
use crate::data::core::{IPropertyInfo, ValueType, ValueTypes, WeakValue};
use crate::data::model::{INotifyPropertyChanged, ModelTypes};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::reactive::IDisposable;
use crate::{AnyValue, BoxedValue};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Views an untyped value as a property change notifier.
pub type AsNotifyPropertyChanged = fn(&dyn AnyValue) -> Option<&dyn INotifyPropertyChanged>;

/// Reads a declared property from a model object that optionally raises
/// property change notifications.
///
/// Properties are found by name in the metadata the object's type declared:
/// its binding metadata ([`ModelTypes`]) or its markup metadata
/// ([`MarkupType`](crate::metadata::MarkupType), walking the base types as
/// reflection walks a class hierarchy); a type without metadata has no
/// properties here.
pub struct InpcPropertyAccessorPlugin;

impl InpcPropertyAccessorPlugin {
    fn find_model_property(obj: &dyn AnyValue, property_name: &str) -> Option<Rc<dyn IPropertyInfo>> {
        ModelTypes::find(obj).and_then(|m| m.find_property(property_name))
    }
}

impl IPropertyAccessorPlugin for InpcPropertyAccessorPlugin {
    fn match_(&self, obj: &dyn AnyValue, property_name: &str) -> bool {
        Self::find_model_property(obj, property_name).is_some()
            || markup_members::find_property(obj, property_name).is_some()
    }

    fn start(&self, reference: &WeakValue, property_name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        let instance = reference.upgrade()?;
        if let Some(p) = Self::find_model_property(&*instance, property_name) {
            return Some(InpcPropertyAccessor::with_runtime_notifier(reference.clone(), p));
        }
        match markup_members::find_property(&*instance, property_name) {
            Some(p) => {
                // The members of a reference type are invoked on the shared
                // object, which is held weakly and asked for its
                // notifications.
                let instance = markup_members::instance_of(&instance);
                Some(InpcPropertyAccessor::with_runtime_notifier(
                    WeakValue::new(&instance),
                    Rc::new(markup_members::MarkupPropertyInfo(p)),
                ))
            }
            None => {
                let message = format!(
                    "Could not find CLR property '{}' on '{}'",
                    property_name,
                    ValueTypes::to_display_string(Some(&instance))
                );
                Some(Rc::new(PropertyError::new(BindingNotification::with_error(
                    BindingError::message(message),
                    BindingErrorType::Error,
                ))))
            }
        }
    }
}

/// Finds the property change notifications of an untyped value, if it
/// raises any.
pub type InpcLookup = Rc<dyn Fn(&dyn AnyValue) -> Option<&dyn INotifyPropertyChanged>>;

/// A property accessor over an [`IPropertyInfo`] that listens to the owner's
/// property change notifications.
pub struct InpcPropertyAccessor {
    this: Weak<InpcPropertyAccessor>,
    base: PropertyAccessorBase,
    reference: WeakValue,
    property: Rc<dyn IPropertyInfo>,
    inpc: Option<InpcLookup>,
    token: Cell<Option<u64>>,
    native_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    event_raised: Cell<bool>,
}

impl InpcPropertyAccessor {
    /// Creates an accessor for `property` of the object behind
    /// `reference`. `inpc` views the object as a property change notifier;
    /// `None` for an object that raises no notifications.
    pub fn new(
        reference: WeakValue,
        property: Rc<dyn IPropertyInfo>,
        inpc: Option<InpcLookup>,
    ) -> Rc<dyn IPropertyAccessor> {
        Self::create(reference, property, inpc)
    }

    /// Creates an accessor for `property` of the object behind
    /// `reference`, whose property change notifications are looked up at
    /// run time: when the accessor subscribes, the object is asked (through
    /// the metadata its type declared, see [`ModelTypes`]) whether it raises
    /// property change notifications, as the cast to the notification
    /// interface does in the managed original.
    pub fn with_runtime_notifier(reference: WeakValue, property: Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor> {
        Self::new(reference, property, Some(Self::runtime_notifier()))
    }

    /// The run-time lookup of the property change notifications of an
    /// object: through the metadata its type declared.
    pub fn runtime_notifier() -> InpcLookup {
        Rc::new(Self::find_notifier)
    }

    /// The property change notifications of an object, if the metadata its
    /// type declared says it raises them. This is the one place where the
    /// accessors of the string-path bindings decide whether an owner
    /// notifies.
    pub fn find_notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
        // Markup metadata first: a view model that states
        // `notify_property_changed:` needs no other registration.
        let markup = crate::metadata::MarkupType::find_by_handle(value.value_type_id());
        if let Some(notifier) = markup
            .and_then(|m| m.notify_property_changed)
            .and_then(|view| view(value))
        {
            return Some(notifier);
        }
        let model = ModelTypes::find(value)?;
        model.as_notify_property_changed(value)
    }

    pub(crate) fn create(
        reference: WeakValue,
        property: Rc<dyn IPropertyInfo>,
        inpc: Option<InpcLookup>,
    ) -> Rc<InpcPropertyAccessor> {
        Rc::new_cyclic(|this| InpcPropertyAccessor {
            this: this.clone(),
            base: PropertyAccessorBase::new(),
            reference,
            property,
            inpc,
            token: Cell::new(None),
            native_subscription: RefCell::new(None),
            event_raised: Cell::new(false),
        })
    }

    pub(crate) fn reference(&self) -> &WeakValue {
        &self.reference
    }

    pub(crate) fn is_subscribed(&self) -> bool {
        self.base.is_subscribed()
    }

    /// Creates an accessor for a property of owner type `O`, which raises
    /// property change notifications.
    pub fn for_notifying<O: INotifyPropertyChanged + 'static>(
        reference: WeakValue,
        property: Rc<dyn IPropertyInfo>,
    ) -> Rc<dyn IPropertyAccessor> {
        Self::new(
            reference,
            property,
            Some(Rc::new(|v: &dyn AnyValue| {
                v.downcast_ref::<O>().map(|o| o as &dyn INotifyPropertyChanged)
            })),
        )
    }

    pub(crate) fn send_current_value(&self) {
        // A failing getter is published as a binding error.
        let value = match self.reference.upgrade() {
            Some(o) => match self.property.try_get_boxed(&o) {
                Ok(value) => value,
                Err(e) => Some(Rc::new(BindingNotification::with_error(e, BindingErrorType::Error)) as BoxedValue),
            },
            None => None,
        };
        self.base.publish_value(value);
    }

    fn remove_handler(&self) {
        if let Some(subscription) = self.native_subscription.take() {
            subscription.dispose();
        }
        if let Some(token) = self.token.take() {
            if let (Some(target), Some(inpc)) = (self.reference.upgrade(), &self.inpc) {
                if let Some(inpc) = inpc(&*target) {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }
}

impl IDisposable for InpcPropertyAccessor {
    fn dispose(&self) {
        if self.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for InpcPropertyAccessor {
    fn property_type(&self) -> Option<ValueType> {
        Some(self.property.property_type())
    }

    fn value(&self) -> Option<BoxedValue> {
        let o = self.reference.upgrade()?;
        self.property.get_boxed(&o)
    }

    fn set_value(&self, value: Option<&BoxedValue>, _priority: BindingPriority) -> Result<bool, BindingError> {
        if !self.property.can_set() {
            return Ok(false);
        }
        let target_type = self.property.property_type();
        let converted = match ValueTypes::try_convert(value, target_type) {
            Some(v) => v,
            None if target_type.is_object() => value.cloned(),
            None => {
                return Err(BindingError::message(format!(
                    "Object of type '{}' cannot be converted to type '{}'.",
                    value.map_or("null", |v| (**v).type_name()),
                    target_type
                )));
            }
        };
        self.event_raised.set(false);
        // A source that no longer exists cannot be written: the setter of
        // the managed original is invoked without a target, which throws.
        let Some(target) = self.reference.upgrade() else {
            return Err(BindingError::message("Non-static method requires a target."));
        };
        self.property.set_boxed(&target, converted.as_ref())?;
        if !self.event_raised.get() {
            self.send_current_value();
        }
        Ok(true)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);
        if let (Some(target), Some(inpc)) = (self.reference.upgrade(), &self.inpc) {
            if let Some(inpc) = inpc(&*target) {
                let weak = self.this.clone();
                let token = inpc.property_changed().add(Rc::new(move |name: &str| {
                    if let Some(this) = weak.upgrade() {
                        if name.is_empty() || name == this.property.name() {
                            this.event_raised.set(true);
                            this.send_current_value();
                        }
                    }
                }));
                self.token.set(Some(token));
            }
        }
        // A statically declared CLR/interface property can wrap a registered
        // property on the runtime native object. Follow that property's changes
        // while still reading/writing through the declared accessors: replacing
        // them with raw property access would bypass overrides and errors.
        // A real INPC subscription remains authoritative when present.
        if self.token.get().is_none() {
            if let Some(target) = self.reference.upgrade().and_then(|v| ValueTypes::as_object(&*v)) {
                if let Some(property) =
                    crate::FerroPropertyRegistry::instance().find_registered_for(&target, self.property.name())
                {
                    let id = property.id();
                    let weak = self.this.clone();
                    let subscription = target.property_changed(move |e| {
                        if e.property().id() == id {
                            if let Some(this) = weak.upgrade() {
                                this.event_raised.set(true);
                                this.send_current_value();
                            }
                        }
                    });
                    self.native_subscription.replace(Some(subscription));
                }
            }
        }
        self.send_current_value();
    }

    fn unsubscribe(&self) {
        self.remove_handler();
        self.base.clear_listener();
    }
}

impl Drop for InpcPropertyAccessor {
    fn drop(&mut self) {
        self.remove_handler();
    }
}

#[cfg(test)]
#[path = "inpc_property_accessor_plugin_tests.rs"]
mod tests;
