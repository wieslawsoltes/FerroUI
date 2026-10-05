use super::{AccessorListener, IPropertyAccessor, InpcPropertyAccessor, PropertyAccessorFactory};
use crate::collections::NotifyCollectionChangedAction;
use crate::data::core::{IPropertyInfo, ValueType, WeakValue};
use crate::data::model::{CollectionChange, INotifyPropertyChanged, ModelTypes};
use crate::data::{BindingError, BindingPriority};
use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The accessor factories used by compiled binding paths for plain
/// properties.
pub struct PropertyInfoAccessorFactory;

impl PropertyInfoAccessorFactory {
    /// An accessor for a property of owner type `O`, which raises property
    /// change notifications.
    pub fn create_inpc_property_accessor<O: INotifyPropertyChanged + 'static>() -> PropertyAccessorFactory {
        Rc::new(|reference, property| InpcPropertyAccessor::for_notifying::<O>(reference.clone(), property))
    }

    /// An accessor for a property of an owner that does not notify: the
    /// value is read once per source.
    pub fn create_plain_property_accessor() -> PropertyAccessorFactory {
        Rc::new(|reference, property| InpcPropertyAccessor::new(reference.clone(), property, None))
    }

    /// An accessor for an indexer with one integer argument: besides the
    /// property change notifications of the owner it follows its collection
    /// change notifications, re-reading the value when a change affects the
    /// position `argument`.
    ///
    /// Whether the owner notifies is decided when the accessor is created,
    /// from what the owner's type declared (see [`ModelTypes`]).
    pub fn create_indexer_property_accessor(argument: i32) -> PropertyAccessorFactory {
        Rc::new(move |reference, property| IndexerAccessor::new(reference.clone(), property, argument))
    }
}

/// The accessor of an indexer with one integer argument. See
/// [`PropertyInfoAccessorFactory::create_indexer_property_accessor`].
pub(crate) struct IndexerAccessor {
    this: Weak<IndexerAccessor>,
    inner: Rc<InpcPropertyAccessor>,
    index: i32,
    token: Cell<Option<u64>>,
}

impl IndexerAccessor {
    pub(crate) fn new(
        target: WeakValue,
        base_property_info: Rc<dyn IPropertyInfo>,
        argument: i32,
    ) -> Rc<dyn IPropertyAccessor> {
        let inner =
            InpcPropertyAccessor::create(target, base_property_info, Some(InpcPropertyAccessor::runtime_notifier()));
        Rc::new_cyclic(|this| IndexerAccessor { this: this.clone(), inner, index: argument, token: Cell::new(None) })
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
        if let Some(token) = self.token.take() {
            if let Some(o) = self.inner.reference().upgrade() {
                if let Some(incc) = ModelTypes::find(&*o).and_then(|m| m.as_notify_collection_changed(&*o)) {
                    incc.collection_changed().remove(token);
                }
            }
        }
    }
}

impl IDisposable for IndexerAccessor {
    fn dispose(&self) {
        if self.inner.is_subscribed() {
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
        if let Some(o) = self.inner.reference().upgrade() {
            if let Some(incc) = ModelTypes::find(&*o).and_then(|m| m.as_notify_collection_changed(&*o)) {
                let weak = self.this.clone();
                let token = incc.collection_changed().add(Rc::new(move |e: &CollectionChange| {
                    if let Some(this) = weak.upgrade() {
                        if this.should_notify_listeners(e) {
                            this.inner.send_current_value();
                        }
                    }
                }));
                self.token.set(Some(token));
            }
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
