use super::{entry_ptr_eq, IValueEntry};
use crate::data::{BindingPriority, BindingValueType};
use crate::ferro_property_metadata::CoerceValueCallback;
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledProperty, StyledPropertyMetadata};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The untyped interface of an [`EffectiveValue`].
pub(crate) trait EffectiveValueDyn: Any {
    fn property(&self) -> &'static FerroProperty;
    fn priority(&self) -> BindingPriority;
    fn base_priority(&self) -> BindingPriority;
    fn value_entry(&self) -> Option<Rc<dyn IValueEntry>>;
    fn is_overriden_current_value(&self) -> bool;
    fn set_is_overriden_current_value(&self, value: bool);
    fn boxed_value(&self) -> BoxedValue;
    /// A reference to the current value as `&dyn Any`, passed to `f`.
    fn with_value(&self, f: &mut dyn FnMut(&dyn Any));

    fn begin_reevaluation(&self, clear_local_value: bool);
    fn end_reevaluation(&self, owner: &FerroObject);
    fn can_remove(&self) -> bool;
    fn unsubscribe_if_necessary(&self);

    /// Sets the value from a value entry and raises change notifications.
    fn set_and_raise(&self, owner: &FerroObject, value: &Rc<dyn IValueEntry>, priority: BindingPriority);
    /// Sets an untyped local value and raises change notifications.
    fn set_local_value_and_raise_untyped(&self, owner: &FerroObject, value: &dyn Any);
    /// Sets a local value from a value entry and raises change notifications.
    fn set_local_value_and_raise_entry(&self, owner: &FerroObject, entry: &dyn IValueEntry);
    /// Raises a change notification on `owner` for a change of the value it
    /// inherits, from `old_value` to `new_value` (either may be absent,
    /// meaning the property's default value).
    fn raise_inherited_value_changed(
        &self,
        owner: &FerroObject,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    );
    fn remove_animation_and_raise(&self, owner: &FerroObject);
    fn coerce_value(&self, owner: &FerroObject);
    fn dispose_and_raise_unset(&self, owner: &FerroObject);
    fn as_any(&self) -> &dyn Any;
}

struct UncommonFields<T> {
    coerce: CoerceValueCallback<T>,
    uncoerced_value: RefCell<T>,
    uncoerced_base_value: RefCell<T>,
}

/// The effective (resolved) value of a styled property on an object: the
/// highest priority value and the highest priority non-animation ("base")
/// value.
pub(crate) struct EffectiveValue<T: PropertyValue> {
    property: &'static StyledProperty<T>,
    metadata: Rc<StyledPropertyMetadata<T>>,
    priority: Cell<BindingPriority>,
    base_priority: Cell<BindingPriority>,
    value_entry: RefCell<Option<Rc<dyn IValueEntry>>>,
    base_value_entry: RefCell<Option<Rc<dyn IValueEntry>>>,
    is_overriden_current_value: Cell<bool>,
    is_coerced_default_value: Cell<bool>,
    value: RefCell<T>,
    base_value: RefCell<Option<T>>,
    uncommon: Option<UncommonFields<T>>,
}

impl<T: PropertyValue> EffectiveValue<T> {
    pub fn new(owner: &FerroObject, property: &'static StyledProperty<T>, inherited: Option<&EffectiveValue<T>>) -> Self {
        let metadata = property.get_metadata_for(owner);
        let value = match inherited {
            Some(inherited) => inherited.value(),
            None => metadata.default_value().clone(),
        };
        let uncommon = metadata.coerce_value().map(|coerce| UncommonFields {
            coerce: coerce.clone(),
            uncoerced_value: RefCell::new(value.clone()),
            uncoerced_base_value: RefCell::new(value.clone()),
        });
        Self {
            property,
            metadata,
            priority: Cell::new(BindingPriority::Unset),
            base_priority: Cell::new(BindingPriority::Unset),
            value_entry: RefCell::new(None),
            base_value_entry: RefCell::new(None),
            is_overriden_current_value: Cell::new(false),
            is_coerced_default_value: Cell::new(false),
            value: RefCell::new(value),
            base_value: RefCell::new(None),
            uncommon,
        }
    }

    /// The current value.
    #[inline]
    pub fn value(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn set_local_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, BindingPriority::LocalValue, false, false);
    }

    pub fn set_current_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, self.priority.get(), true, false);
    }

    pub fn set_coerced_default_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, self.priority.get(), false, true);
    }

    /// The base value, if there is one.
    pub fn try_get_base_value(&self) -> Option<T> {
        if self.base_priority.get() != BindingPriority::Unset {
            self.base_value.borrow().clone()
        } else {
            None
        }
    }

    fn get_entry_value(&self, entry: &dyn IValueEntry) -> T {
        let mut out: Option<T> = None;
        if entry.has_value() {
            if entry.try_get_value(&mut out) {
                if let Some(v) = out {
                    return v;
                }
            } else if let Some(boxed) = entry.get_value_boxed() {
                // An untyped entry (a binding expression): its value is
                // already converted to the property type, or is the unset
                // marker.
                if let Some(v) = boxed.downcast_ref::<T>() {
                    return v.clone();
                }
            }
        }
        self.metadata.default_value().clone()
    }

    fn update_value_entry(&self, entry: Option<&Rc<dyn IValueEntry>>, priority: BindingPriority) {
        debug_assert!(priority != BindingPriority::LocalValue);

        fn same(a: &Option<Rc<dyn IValueEntry>>, b: Option<&Rc<dyn IValueEntry>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => entry_ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }

        fn replace(slot: &RefCell<Option<Rc<dyn IValueEntry>>>, entry: Option<&Rc<dyn IValueEntry>>) {
            if !same(&slot.borrow(), entry) {
                let old = slot.replace(entry.cloned());
                if let Some(old) = old {
                    old.unsubscribe();
                }
            }
        }

        let current = self.priority.get();
        if priority <= BindingPriority::Animation {
            // If we've received an animation value and the current value is a
            // non-animation value, then the current entry becomes our base entry.
            if current > BindingPriority::LocalValue && current < BindingPriority::Inherited {
                let entry = self.value_entry.take();
                debug_assert!(entry.is_some());
                *self.base_value_entry.borrow_mut() = entry;
            }
            replace(&self.value_entry, entry);
        } else if current <= BindingPriority::Animation {
            // We've received a non-animation value and have an active animation
            // value, so the new entry becomes our base entry.
            replace(&self.base_value_entry, entry);
        } else {
            // Both the current value and the new value are non-animation values,
            // so the new entry replaces the existing entry.
            replace(&self.value_entry, entry);
        }
    }

    fn set_and_raise_core(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        is_overridden_current_value: bool,
        is_coerced_default_value: bool,
    ) {
        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;

        self.is_overriden_current_value.set(is_overridden_current_value);
        self.is_coerced_default_value.set(is_coerced_default_value);

        let v = match &self.uncommon {
            Some(uncommon) if !is_coerced_default_value => (uncommon.coerce)(owner, value.clone()),
            _ => value.clone(),
        };

        if priority <= self.priority.get() {
            value_changed = *self.value.borrow() != v;
            // `replace` releases the borrow before the previous value is
            // dropped: dropping a value may run code that reads the property.
            self.value.replace(v.clone());
            self.priority.set(priority);
            if let (false, Some(uncommon)) = (is_coerced_default_value, &self.uncommon) {
                uncommon.uncoerced_value.replace(value.clone());
            }
        }

        if priority <= self.base_priority.get() && priority >= BindingPriority::LocalValue {
            base_value_changed = self.base_value.borrow().as_ref() != Some(&v);
            self.base_value.replace(Some(v));
            self.base_priority.set(priority);
            if let (false, Some(uncommon)) = (is_coerced_default_value, &self.uncommon) {
                uncommon.uncoerced_base_value.replace(value);
            }
        }

        if value_changed {
            self.notify_value_changed(owner, old_value);
        } else if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }

    fn set_and_raise_core_with_base(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        base_value: T,
        base_priority: BindingPriority,
    ) {
        debug_assert!(base_priority > BindingPriority::Animation);
        debug_assert!(priority <= base_priority);

        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;
        let mut v = value.clone();
        let mut bv = base_value.clone();

        if let Some(uncommon) = &self.uncommon {
            v = (uncommon.coerce)(owner, value.clone());
            if priority != base_priority {
                bv = (uncommon.coerce)(owner, base_value.clone());
            }
        }

        if *self.value.borrow() != v {
            self.value.replace(v.clone());
            value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(value);
            }
        }

        if self.base_value.borrow().as_ref() != Some(&bv) {
            self.base_value.replace(Some(v));
            base_value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(base_value);
            }
        }

        self.priority.set(priority);
        self.base_priority.set(base_priority);

        if value_changed {
            self.notify_value_changed(owner, old_value);
        }
        if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }

    fn notify_value_changed(&self, owner: &FerroObject, old_value: T) {
        let notifying = self.property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }
        let new_value = self.value();
        owner.raise_property_changed(self.property, Some(&old_value), &new_value, self.priority.get(), true);
        if self.property.inherits() {
            // A change handler may have changed the value again: inheritance
            // children are told the value as it is now, not the value that
            // was announced to the handlers of this object.
            let new_value = self.value();
            owner.values().on_inherited_effective_value_changed(owner, self.property, &old_value, &new_value);
        }
        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }

    fn notify_base_value_changed(&self, owner: &FerroObject) {
        let base = self.base_value.borrow().clone();
        if let Some(base) = base {
            owner.raise_property_changed(self.property, None, &base, self.base_priority.get(), false);
        }
    }

    fn data_validation_enabled(&self) -> bool {
        let has = |slot: &RefCell<Option<Rc<dyn IValueEntry>>>| {
            slot.borrow().as_ref().map(|e| e.get_data_validation_state().is_some())
        };
        has(&self.value_entry).or_else(|| has(&self.base_value_entry)).unwrap_or(false)
    }
}

/// Panics for an untyped local value that is not of the value type of the
/// property (compiled once, not per value type).
#[cold]
#[track_caller]
fn invalid_value_type(property: &FerroProperty) -> ! {
    panic!("invalid value type for property '{}'", property.name())
}

/// Downcasts an untyped effective value to its typed form.
#[inline]
pub(crate) fn cast_effective_value<T: PropertyValue>(value: &dyn EffectiveValueDyn) -> &EffectiveValue<T> {
    debug_assert!(value.as_any().is::<EffectiveValue<T>>());
    // SAFETY: effective values are created exclusively by
    // `StyledProperty<T>::create_effective_value` and stored under that
    // property's ID, and a property ID is never shared between properties of
    // different value types. A value looked up by a `StyledProperty<T>` is
    // therefore always an `EffectiveValue<T>`. Skipping the checked downcast
    // matters because reading a property value is the hottest path of the
    // property system.
    unsafe { &*(value as *const dyn EffectiveValueDyn as *const EffectiveValue<T>) }
}

impl<T: PropertyValue> EffectiveValueDyn for EffectiveValue<T> {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn priority(&self) -> BindingPriority {
        self.priority.get()
    }

    fn base_priority(&self) -> BindingPriority {
        self.base_priority.get()
    }

    fn value_entry(&self) -> Option<Rc<dyn IValueEntry>> {
        self.value_entry.borrow().clone()
    }

    fn is_overriden_current_value(&self) -> bool {
        self.is_overriden_current_value.get()
    }

    fn set_is_overriden_current_value(&self, value: bool) {
        self.is_overriden_current_value.set(value)
    }

    fn boxed_value(&self) -> BoxedValue {
        Rc::new(self.value())
    }

    fn with_value(&self, f: &mut dyn FnMut(&dyn Any)) {
        let value = self.value();
        f(&value)
    }

    fn begin_reevaluation(&self, clear_local_value: bool) {
        let overridden = self.is_overriden_current_value.get();
        if clear_local_value || (self.priority.get() != BindingPriority::LocalValue && !overridden) {
            self.priority.set(BindingPriority::Unset);
        }
        if clear_local_value || (self.base_priority.get() != BindingPriority::LocalValue && !overridden) {
            self.base_priority.set(BindingPriority::Unset);
        }
    }

    fn end_reevaluation(&self, owner: &FerroObject) {
        if self.priority.get() == BindingPriority::Unset {
            if let Some(uncommon) = &self.uncommon {
                // Coerce the default value and raise if it differs.
                let default = self.metadata.default_value().clone();
                let coerced = (uncommon.coerce)(owner, default.clone());
                if default != coerced {
                    self.set_coerced_default_value_and_raise(owner, coerced);
                }
            }
        }
    }

    fn can_remove(&self) -> bool {
        self.priority.get() == BindingPriority::Unset
            && !self.is_overriden_current_value.get()
            && !self.is_coerced_default_value.get()
    }

    fn unsubscribe_if_necessary(&self) {
        if self.priority.get() == BindingPriority::Unset {
            if let Some(entry) = self.value_entry.take() {
                entry.unsubscribe();
            }
        }
        if self.base_priority.get() == BindingPriority::Unset {
            if let Some(entry) = self.base_value_entry.take() {
                entry.unsubscribe();
            }
        }
    }

    fn set_and_raise(&self, owner: &FerroObject, value: &Rc<dyn IValueEntry>, priority: BindingPriority) {
        debug_assert!(priority != BindingPriority::LocalValue);
        self.update_value_entry(Some(value), priority);
        let v = self.get_entry_value(&**value);
        self.set_and_raise_core(owner, v, priority, false, false);

        if priority > BindingPriority::LocalValue {
            if let Some((state, error)) = value.get_data_validation_state() {
                owner.on_update_data_validation(self.property, state, error);
            }
        }
    }

    fn set_local_value_and_raise_untyped(&self, owner: &FerroObject, value: &dyn Any) {
        let value = value
            .downcast_ref::<T>()
            .unwrap_or_else(|| invalid_value_type(self.property))
            .clone();
        self.set_local_value_and_raise(owner, value);
    }

    fn set_local_value_and_raise_entry(&self, owner: &FerroObject, entry: &dyn IValueEntry) {
        let value = self.get_entry_value(entry);
        self.set_local_value_and_raise(owner, value);
    }

    fn raise_inherited_value_changed(
        &self,
        owner: &FerroObject,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    ) {
        debug_assert!(old_value.is_some() || new_value.is_some());
        let default = || self.metadata.default_value().clone();
        let o = old_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let n = new_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let priority = if new_value.is_some() { BindingPriority::Inherited } else { BindingPriority::Unset };
        if o != n {
            owner.raise_property_changed(self.property, Some(&o), &n, priority, true);
        }
    }

    fn remove_animation_and_raise(&self, owner: &FerroObject) {
        debug_assert!(self.priority.get() != BindingPriority::Animation);
        debug_assert!(self.base_priority.get() != BindingPriority::Unset);
        self.update_value_entry(None, BindingPriority::Animation);
        let base = self.base_value.borrow().clone().expect("base value is set");
        self.set_and_raise_core(owner, base, self.base_priority.get(), false, false);
    }

    fn coerce_value(&self, owner: &FerroObject) {
        let Some(uncommon) = &self.uncommon else { return };
        let value = uncommon.uncoerced_value.borrow().clone();
        let base_value = uncommon.uncoerced_base_value.borrow().clone();
        self.set_and_raise_core_with_base(owner, value, self.priority.get(), base_value, self.base_priority.get());
    }

    fn dispose_and_raise_unset(&self, owner: &FerroObject) {
        let clear_data_validation = self.data_validation_enabled();

        if let Some(entry) = self.value_entry.take() {
            entry.unsubscribe();
        }
        if let Some(entry) = self.base_value_entry.take() {
            entry.unsubscribe();
        }

        let inherited = if self.property.inherits() {
            owner.values().try_get_inherited_value(self.property)
        } else {
            None
        };
        let (new_value, priority) = match &inherited {
            Some(i) => (cast_effective_value::<T>(&**i).value(), BindingPriority::Inherited),
            None => (self.metadata.default_value().clone(), BindingPriority::Unset),
        };

        let old_value = self.value();
        if new_value != old_value {
            owner.raise_property_changed(self.property, Some(&old_value), &new_value, priority, true);
            if self.property.inherits() {
                owner.values().on_inherited_effective_value_disposed(owner, self.property, &old_value, &new_value);
            }
        }

        if clear_data_validation {
            owner.on_update_data_validation(self.property, BindingValueType::UNSET_VALUE, None);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
