//! Port of `MarkupExtensions/CompiledBindings/PropertyInfoAccessorFactory.cs`.

use crate::XamlLoadException;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::core::plugins::{
    AccessorListener, FerroPropertyAccessor, IPropertyAccessor, InpcPropertyAccessor as BaseInpcPropertyAccessor,
    PropertyAccessorBase,
};
use ferroui_base::data::core::{IPropertyInfo, ValueType, ValueTypes, WeakValue};
use ferroui_base::data::model::{CollectionChange, ModelTypes};
use ferroui_base::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The accessor factories compiled binding paths use for their property
/// elements.
pub struct PropertyInfoAccessorFactory;

impl PropertyInfoAccessorFactory {
    /// An accessor for a plain property that follows the property change
    /// notifications of the object, if it raises them.
    pub fn create_inpc_property_accessor(target: &WeakValue, property: Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor> {
        InpcPropertyAccessor::new(target.clone(), property)
    }

    /// An accessor for a registered property, given through the property
    /// description contract.
    ///
    /// # Panics
    /// Panics if `property` is not a registered property (the invalid cast
    /// of the managed original). Use
    /// [`try_create_ferro_property_accessor`](Self::try_create_ferro_property_accessor)
    /// to handle it.
    pub fn create_ferro_property_accessor(target: &WeakValue, property: Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor> {
        crate::throw(Self::try_create_ferro_property_accessor(target, property))
    }

    /// [`create_ferro_property_accessor`](Self::create_ferro_property_accessor)
    /// without the panic: a property that is not a registered property is
    /// an error.
    pub fn try_create_ferro_property_accessor(
        target: &WeakValue,
        property: Rc<dyn IPropertyInfo>,
    ) -> Result<Rc<dyn IPropertyAccessor>, XamlLoadException> {
        let Some(ferro_property) = property.as_ferro_property() else {
            return Err(XamlLoadException::with_message(format!(
                "Unable to cast the property '{}' to type 'FerroProperty'.",
                property.name()
            )));
        };
        // A target that is gone, or is not an object, is a reference to
        // nothing.
        let object = target.upgrade().and_then(|o| ValueTypes::as_object(&*o));
        Ok(FerroPropertyAccessor::new(object.map(|o| o.downgrade()), ferro_property))
    }

    /// An accessor for an indexer with one integer argument: besides the
    /// property change notifications it follows the collection change
    /// notifications of the object, re-reading the value when a change
    /// affects the position `argument`.
    pub fn create_indexer_property_accessor(
        target: &WeakValue,
        property: Rc<dyn IPropertyInfo>,
        argument: i32,
    ) -> Rc<dyn IPropertyAccessor> {
        IndexerAccessor::new(target.clone(), property, argument)
    }
}

// The accessor of a registered property (the first internal class of the
// managed file) is the one of the base library, `FerroPropertyAccessor`.

/// The accessor of a plain property, as the managed file declares it. It
/// differs from the accessor of the string-path bindings of the base
/// library: a value is written as it is (no conversion), writing to a
/// source that no longer exists does nothing and reports `false`, and the
/// current value is always published after a write.
pub(crate) struct InpcPropertyAccessor {
    this: Weak<InpcPropertyAccessor>,
    base: PropertyAccessorBase,
    reference: WeakValue,
    property: Rc<dyn IPropertyInfo>,
    token: Cell<Option<u64>>,
}

impl InpcPropertyAccessor {
    pub(crate) fn new(reference: WeakValue, property: Rc<dyn IPropertyInfo>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: PropertyAccessorBase::new(),
            reference,
            property,
            token: Cell::new(None),
        })
    }

    fn send_current_value(&self) {
        let value = match self.reference.upgrade() {
            Some(o) => match self.property.try_get_boxed(&o) {
                Ok(value) => value,
                Err(e) => Some(Rc::new(BindingNotification::with_error(e, BindingErrorType::Error)) as BoxedValue),
            },
            None => None,
        };
        self.base.publish_value(value);
    }

    fn subscribe_to_changes(&self) {
        let Some(o) = self.reference.upgrade() else { return };
        // `o is INotifyPropertyChanged`: through the metadata of its type.
        if let Some(inpc) = BaseInpcPropertyAccessor::find_notifier(&*o) {
            let weak = self.this.clone();
            let token = inpc.property_changed().add(Rc::new(move |property_name: &str| {
                if let Some(this) = weak.upgrade() {
                    if property_name == this.property.name() || property_name.is_empty() {
                        this.send_current_value();
                    }
                }
            }));
            self.token.set(Some(token));
        }
    }

    fn unsubscribe_from_changes(&self) {
        let Some(token) = self.token.take() else { return };
        let Some(o) = self.reference.upgrade() else { return };
        if let Some(inpc) = BaseInpcPropertyAccessor::find_notifier(&*o) {
            inpc.property_changed().remove(token);
        }
    }

    fn subscribe_core(&self) {
        self.send_current_value();
        self.subscribe_to_changes();
    }

    fn unsubscribe_core(&self) {
        self.unsubscribe_from_changes();
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
        if self.property.can_set() {
            if let Some(o) = self.reference.upgrade() {
                self.property.set_boxed(&o, value)?;

                self.send_current_value();

                return Ok(true);
            }
        }

        Ok(false)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);
        self.subscribe_core();
    }

    fn unsubscribe(&self) {
        self.unsubscribe_core();
        self.base.clear_listener();
    }
}

impl Drop for InpcPropertyAccessor {
    fn drop(&mut self) {
        self.unsubscribe_from_changes();
    }
}

/// The accessor of an indexer with one integer argument. (The managed class
/// derives from the plain property accessor; here it holds one.)
pub(crate) struct IndexerAccessor {
    this: Weak<IndexerAccessor>,
    inner: Rc<InpcPropertyAccessor>,
    index: i32,
    token: Cell<Option<u64>>,
}

impl IndexerAccessor {
    pub(crate) fn new(target: WeakValue, base_property_info: Rc<dyn IPropertyInfo>, argument: i32) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            inner: InpcPropertyAccessor::new(target, base_property_info),
            index: argument,
            token: Cell::new(None),
        })
    }

    fn should_notify_listeners(&self, e: &CollectionChange) -> bool {
        let index = self.index;
        match e.action {
            NotifyCollectionChangedAction::Add => index >= e.new_starting_index,
            NotifyCollectionChangedAction::Remove => index >= e.old_starting_index,
            NotifyCollectionChangedAction::Replace => {
                index >= e.new_starting_index && index < e.new_starting_index + e.new_count as i32
            }
            NotifyCollectionChangedAction::Move => {
                (index >= e.new_starting_index && index < e.new_starting_index + e.new_count as i32)
                    || (index >= e.old_starting_index && index < e.old_starting_index + e.old_count as i32)
            }
            NotifyCollectionChangedAction::Reset => true,
        }
    }

    fn remove_handler(&self) {
        let Some(token) = self.token.take() else { return };
        let Some(o) = self.inner.reference.upgrade() else { return };
        let Some(model) = ModelTypes::find(&*o) else { return };
        if let Some(incc) = model.as_notify_collection_changed(&*o) {
            incc.collection_changed().remove(token);
        }
    }
}

impl IDisposable for IndexerAccessor {
    fn dispose(&self) {
        if self.inner.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for IndexerAccessor {
    fn property_type(&self) -> Option<ValueType> {
        self.inner.property_type()
    }

    fn value(&self) -> Option<BoxedValue> {
        self.inner.value()
    }

    fn set_value(&self, value: Option<&BoxedValue>, priority: BindingPriority) -> Result<bool, BindingError> {
        self.inner.set_value(value, priority)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.inner.subscribe(listener);
        let Some(o) = self.inner.reference.upgrade() else { return };
        let Some(model) = ModelTypes::find(&*o) else { return };
        if let Some(incc) = model.as_notify_collection_changed(&*o) {
            let weak = self.this.clone();
            let token = incc.collection_changed().add(Rc::new(move |args: &CollectionChange| {
                if let Some(this) = weak.upgrade() {
                    if this.should_notify_listeners(args) {
                        this.inner.send_current_value();
                    }
                }
            }));
            self.token.set(Some(token));
        }
    }

    fn unsubscribe(&self) {
        self.inner.unsubscribe();
        self.remove_handler();
    }
}

impl Drop for IndexerAccessor {
    fn drop(&mut self) {
        self.remove_handler();
    }
}

ferro_markup_type!(static PropertyInfoAccessorFactory {
    methods: [
        static fn CreateInpcPropertyAccessor(WeakValue, Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor> =>
            |target: WeakValue, property: Rc<dyn IPropertyInfo>| {
                PropertyInfoAccessorFactory::create_inpc_property_accessor(&target, property)
            },
        static try fn CreateFerroPropertyAccessor(WeakValue, Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor> =>
            |target: WeakValue, property: Rc<dyn IPropertyInfo>| {
                PropertyInfoAccessorFactory::try_create_ferro_property_accessor(&target, property)
            },
        static fn CreateIndexerPropertyAccessor(WeakValue, Rc<dyn IPropertyInfo>, i32) -> Rc<dyn IPropertyAccessor> =>
            |target: WeakValue, property: Rc<dyn IPropertyInfo>, argument: i32| {
                PropertyInfoAccessorFactory::create_indexer_property_accessor(&target, property, argument)
            },
    ],
});
