use super::{markup_members, AccessorListener, IPropertyAccessor, IPropertyAccessorPlugin, PropertyAccessorBase, PropertyError};
use crate::data::core::{IPropertyInfo, ValueType, ValueTypes, WeakValue};
use crate::data::model::{INotifyPropertyChanged, ModelTypes};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::reactive::IDisposable;
use crate::{AnyValue, BoxedValue};
use std::cell::Cell;
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
        if let Some(notifier) = markup.and_then(|m| m.notify_property_changed).and_then(|view| view(value)) {
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
            Some(Rc::new(|v: &dyn AnyValue| v.downcast_ref::<O>().map(|o| o as &dyn INotifyPropertyChanged))),
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
                )))
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
mod tests {
    use super::*;
    use crate::data::core::{ClrPropertyInfo, Value};
    use std::cell::RefCell;

    struct Source {
        text: RefCell<String>,
    }

    impl PartialEq for Source {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    fn text_property() -> Rc<dyn IPropertyInfo> {
        Rc::new(ClrPropertyInfo::read_write::<Source, Value<String>>(
            "Text",
            |s| s.text.borrow().clone(),
            |s, v| *s.text.borrow_mut() = v,
        ))
    }

    fn source() -> (Rc<Source>, BoxedValue) {
        ValueTypes::register_reference::<Source>();
        let source = Rc::new(Source { text: RefCell::new("a".to_string()) });
        let boxed: BoxedValue = source.clone();
        (source, boxed)
    }

    #[test]
    fn set_value_converts_before_it_writes() {
        let (source, boxed) = source();
        let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), text_property(), None);

        // A value of another type is converted to the type of the property.
        let number: BoxedValue = Rc::new(5i32);
        assert_eq!(accessor.set_value(Some(&number), BindingPriority::LocalValue), Ok(true));
        assert_eq!(*source.text.borrow(), "5");

        // A value that cannot be converted is an error, and nothing is written.
        let accessor = InpcPropertyAccessor::new(
            WeakValue::new(&boxed),
            Rc::new(ClrPropertyInfo::read_write::<Source, Value<i32>>("Length", |s| s.text.borrow().len() as i32, |_, _| {})),
            None,
        );
        let text: BoxedValue = Rc::new("not a number".to_string());
        let error = accessor.set_value(Some(&text), BindingPriority::LocalValue).unwrap_err();
        assert!(error.to_string().contains("cannot be converted to type"), "{error}");
    }

    #[test]
    fn set_value_on_a_source_that_no_longer_exists_is_an_error() {
        let (source, boxed) = source();
        let reference = WeakValue::new(&boxed);
        let accessor = InpcPropertyAccessor::new(reference, text_property(), None);
        drop(boxed);
        drop(source);

        assert!(accessor.value().is_none());
        let value: BoxedValue = Rc::new("b".to_string());
        let error = accessor.set_value(Some(&value), BindingPriority::LocalValue).unwrap_err();
        assert_eq!(error.to_string(), "Non-static method requires a target.");
        // The conversion comes first: its error wins over the missing target.
        let read_only = InpcPropertyAccessor::new(
            WeakValue::Value(Rc::new(1i32)),
            Rc::new(ClrPropertyInfo::read_only::<Source, Value<String>>("Text", |s| s.text.borrow().clone())),
            None,
        );
        assert_eq!(read_only.set_value(Some(&value), BindingPriority::LocalValue), Ok(false));
    }
}
