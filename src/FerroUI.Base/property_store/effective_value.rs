use super::{entry_ptr_eq, IValueEntry};
use crate::data::{BindingPriority, BindingValueType};
use crate::ferro_property::value_equals;
use crate::ferro_property_metadata::{CoerceValueCallback, CoercedWrite, CoercionOperations, DefaultCoercionOperations};
use crate::{BoxedValue, FerroObject, FerroProperty, PropertyValue, StyledProperty, StyledPropertyMetadata};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The untyped interface of an [`EffectiveValue`].
pub(crate) trait EffectiveValueDyn: Any {
    // A single projection gives the common store its property identity,
    // priorities and subscriptions for every payload type.
    fn state(&self) -> &EffectiveValueState;
    fn boxed_value(&self) -> BoxedValue;
    fn end_reevaluation(&self, owner: &FerroObject);
    /// Reads the entry as this property's Rust type, then applies it.
    fn apply_entry(&self, owner: &FerroObject, entry: &dyn IValueEntry, priority: BindingPriority);
    fn remove_animation_and_raise(&self, owner: &FerroObject);
    fn coerce_value(&self, owner: &FerroObject);
    fn raise_unset(
        &self,
        owner: &FerroObject,
        inherited: Option<&dyn EffectiveValueDyn>,
        clear_data_validation: bool,
    );
}

pub(crate) struct UncommonFields<T> {
    coerce: CoerceValueCallback<T>,
    uncoerced_value: RefCell<T>,
    uncoerced_base_value: RefCell<T>,
}

impl<T: Clone> UncommonFields<T> {
    pub(crate) fn new(coerce: &CoerceValueCallback<T>, value: &T) -> Self {
        // Preserve the original map closure's initialization and partial-drop
        // order. In particular, the callback is cloned before either T clone.
        Self {
            coerce: coerce.clone(),
            uncoerced_value: RefCell::new(value.clone()),
            uncoerced_base_value: RefCell::new(value.clone()),
        }
    }
}

/// Entry selection and subscription lifetime are identical for every styled
/// property. Only the resolved values and their coercion callback depend on T.
pub(crate) struct EffectiveValueState {
    property: &'static FerroProperty,
    priority: Cell<BindingPriority>,
    base_priority: Cell<BindingPriority>,
    value_entry: RefCell<Option<Rc<dyn IValueEntry>>>,
    base_value_entry: RefCell<Option<Rc<dyn IValueEntry>>>,
    is_overriden_current_value: Cell<bool>,
    is_coerced_default_value: Cell<bool>,
}

impl EffectiveValueState {
    fn new(property: &'static FerroProperty) -> Self {
        Self {
            property,
            priority: Cell::new(BindingPriority::Unset),
            base_priority: Cell::new(BindingPriority::Unset),
            value_entry: RefCell::new(None),
            base_value_entry: RefCell::new(None),
            is_overriden_current_value: Cell::new(false),
            is_coerced_default_value: Cell::new(false),
        }
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

    fn begin_reevaluation(&self, clear_local_value: bool) {
        let overridden = self.is_overriden_current_value.get();
        if clear_local_value || (self.priority.get() != BindingPriority::LocalValue && !overridden) {
            self.priority.set(BindingPriority::Unset);
        }
        if clear_local_value || (self.base_priority.get() != BindingPriority::LocalValue && !overridden) {
            self.base_priority.set(BindingPriority::Unset);
        }
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

    fn data_validation_enabled(&self) -> bool {
        let has = |slot: &RefCell<Option<Rc<dyn IValueEntry>>>| {
            slot.borrow().as_ref().map(|e| e.get_data_validation_state().is_some())
        };
        has(&self.value_entry).or_else(|| has(&self.base_value_entry)).unwrap_or(false)
    }

    fn finish_unset(
        &self,
        owner: &FerroObject,
        old: &dyn Any,
        new: &dyn Any,
        priority: BindingPriority,
        changed: bool,
        clear_data_validation: bool,
    ) {
        if changed {
            owner.raise_property_changed_values(self.property, Some(old), new, priority, true);
            if self.property.inherits() {
                owner.values().on_inherited_effective_value_changed(owner, self.property, old, new);
            }
        }
        if clear_data_validation {
            owner.on_update_data_validation(self.property, BindingValueType::UNSET_VALUE, None);
        }
    }

    /// Raises the change of the value of an effective value from
    /// `old_value`: the notifying callback of the property, the change on the
    /// owner and the change of the inheritance children. Compiled once; the
    /// value type matters only to copy the new value, which the table of the
    /// value type does (#576). `value` is the cell of the effective value, and
    /// `new_value` and `inherited_value` are empty `Option`s of the value type
    /// in the frame of the typed caller, which receive the copy that is
    /// announced and the copy that the inheritance children are told; they
    /// drop there after this returns, as the copies of the typed notification
    /// did.
    #[inline(never)]
    fn notify_value_changed(
        &self,
        owner: &FerroObject,
        value: &RefCell<dyn Any>,
        old_value: &dyn Any,
        new_value: &mut dyn Any,
        inherited_value: &mut dyn Any,
    ) {
        let property = self.property;
        let notifying = property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }
        let clone_into = property.value_table().clone_into;
        // SAFETY: the cell holds the property's value type, and the copies are
        // `Option`s of it (the typed caller).
        let new_value = unsafe { clone_into(&*value.borrow(), new_value) };
        owner.raise_property_changed_values(property, Some(old_value), new_value, self.priority.get(), true);
        if property.inherits() {
            // A change handler may have changed the value again: inheritance
            // children are told the value as it is now, not the value that
            // was announced to the handlers of this object.
            // SAFETY: as above.
            let new_value = unsafe { clone_into(&*value.borrow(), inherited_value) };
            owner.values().on_inherited_effective_value_changed(owner, property, old_value, new_value);
        }
        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }
}

// The store already holds these values through a trait object. Only methods
// that read or write a typed payload need a distinct vtable slot per T.
impl dyn EffectiveValueDyn {
    pub(crate) fn property(&self) -> &'static FerroProperty {
        self.state().property
    }

    /// Uses the capability of the already-selected property, avoiding another
    /// state projection and a typed vtable slot for non-inheriting properties.
    pub(crate) fn raise_inherited_value_changed(
        &self,
        property: &'static FerroProperty,
        owner: &FerroObject,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    ) {
        (property.inherited_value_changed())(self, owner, old_value, new_value);
    }

    pub(crate) fn priority(&self) -> BindingPriority {
        self.state().priority.get()
    }

    pub(crate) fn base_priority(&self) -> BindingPriority {
        self.state().base_priority.get()
    }

    pub(crate) fn value_entry(&self) -> Option<Rc<dyn IValueEntry>> {
        self.state().value_entry.borrow().clone()
    }

    pub(crate) fn is_overriden_current_value(&self) -> bool {
        self.state().is_overriden_current_value.get()
    }

    pub(crate) fn set_is_overriden_current_value(&self, value: bool) {
        self.state().is_overriden_current_value.set(value);
    }

    pub(crate) fn begin_reevaluation(&self, clear_local_value: bool) {
        self.state().begin_reevaluation(clear_local_value);
    }

    pub(crate) fn can_remove(&self) -> bool {
        let state = self.state();
        state.priority.get() == BindingPriority::Unset
            && !state.is_overriden_current_value.get()
            && !state.is_coerced_default_value.get()
    }

    pub(crate) fn unsubscribe_if_necessary(&self) {
        self.state().unsubscribe_if_necessary();
    }

    pub(crate) fn set_and_raise(&self, owner: &FerroObject, entry: &Rc<dyn IValueEntry>, priority: BindingPriority) {
        debug_assert!(priority != BindingPriority::LocalValue);
        let state = self.state();
        state.update_value_entry(Some(entry), priority);
        self.apply_entry(owner, &**entry, priority);
        if priority > BindingPriority::LocalValue {
            if let Some((kind, error)) = entry.get_data_validation_state() {
                owner.on_update_data_validation(state.property, kind, error);
            }
        }
    }

    pub(crate) fn set_local_value_and_raise_entry(&self, owner: &FerroObject, entry: &dyn IValueEntry) {
        self.apply_entry(owner, entry, BindingPriority::LocalValue);
    }

    pub(crate) fn dispose_and_raise_unset(&self, owner: &FerroObject) {
        let state = self.state();
        let clear_data_validation = state.data_validation_enabled();
        if let Some(entry) = state.value_entry.take() {
            entry.unsubscribe();
        }
        if let Some(entry) = state.base_value_entry.take() {
            entry.unsubscribe();
        }
        let inherited = if state.property.inherits() { owner.values().try_get_inherited_value(state.property) } else { None };
        self.raise_unset(owner, inherited.as_deref(), clear_data_validation);
    }
}

/// The effective (resolved) value of a styled property on an object: the
/// highest priority value and the highest priority non-animation ("base")
/// value.
// Keep the representation unbounded so Clone-only public metadata can name
// its cold algorithm. Construction, store access and notification impls retain
// their PropertyValue bounds; none of the typed fields is erased or recast.
pub(crate) struct EffectiveValue<T> {
    metadata: Rc<StyledPropertyMetadata<T>>,
    state: EffectiveValueState,
    value: RefCell<T>,
    base_value: RefCell<Option<T>>,
    uncommon: Option<UncommonFields<T>>,
}

impl<T: Clone + 'static> EffectiveValue<T> {
    /// The current value.
    #[inline]
    pub fn value(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn with_value<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.value.borrow())
    }

    // Preserve the two owned-value frames and their argument evaluation:
    // clone the uncoerced value, then the base, then read priorities in the
    // original call. Clone, equality, coercion and Drop can reenter the store.
    pub(crate) fn apply_coercion(&self, owner: &FerroObject, operations: CoercionOperations<T>) {
        let Some(uncommon) = &self.uncommon else { return };
        let value = uncommon.uncoerced_value.borrow().clone();
        let base_value = uncommon.uncoerced_base_value.borrow().clone();
        self.set_and_raise_core_with_base(owner, value, self.state.priority.get(), base_value, self.state.base_priority.get(), operations);
    }

    pub(crate) fn apply_coercion_at_end(&self, owner: &FerroObject, operations: DefaultCoercionOperations<T>) {
        if let Some(uncommon) = &self.uncommon {
            // Coerce the default value and raise if it differs. The default
            // stays alive through the typed setter, including its unwind.
            let default = self.metadata.default_value().clone();
            let coerced = (uncommon.coerce)(owner, default.clone());
            if !(operations.equals)(&default, &coerced) {
                (operations.set_default)(self, owner, coerced);
            }
        }
    }

    fn set_and_raise_core_with_base(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        base_value: T,
        base_priority: BindingPriority,
        operations: CoercionOperations<T>,
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

        if !(operations.equals)(&*self.value.borrow(), &v) {
            self.value.replace(v.clone());
            value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(value);
            }
        }

        if !self.base_value.borrow().as_ref().is_some_and(|base| (operations.equals)(base, &bv)) {
            self.base_value.replace(Some(v));
            base_value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(base_value);
            }
        }

        self.state.priority.set(priority);
        self.state.base_priority.set(base_priority);

        if value_changed {
            (operations.notify_value_changed)(self, owner, old_value);
        }
        if base_value_changed {
            (operations.notify_base_value_changed)(self, owner);
        }
    }

    // The coercing branch of `set_and_raise_core`, reached only through the
    // dispatch of a registered coercer: a value type whose properties never
    // coerce does not retain it (#576). The steps, their order and the
    // owned-value frame are those of the original: the value given is a local
    // declared before the old value, as a parameter drops after the locals.
    pub(crate) fn set_and_raise_coerced(&self, owner: &FerroObject, write: &mut CoercedWrite<T>) {
        let value = write.value.take().expect("a coerced write has a value");
        let priority = write.priority;
        let equals = write.operations.equals;
        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;

        self.state.is_overriden_current_value.set(write.is_overridden_current_value);
        self.state.is_coerced_default_value.set(false);

        let (v, uncoerced) = match &self.uncommon {
            Some(uncommon) => ((uncommon.coerce)(owner, value.clone()), Some((uncommon, value))),
            None => (value, None),
        };

        if priority <= self.state.priority.get() {
            value_changed = !equals(&*self.value.borrow(), &v);
            // `replace` releases the borrow before the previous value is
            // dropped: dropping a value may run code that reads the property.
            self.value.replace(v.clone());
            self.state.priority.set(priority);
            if let Some((uncommon, value)) = &uncoerced {
                uncommon.uncoerced_value.replace(value.clone());
            }
        }

        if priority <= self.state.base_priority.get() && priority >= BindingPriority::LocalValue {
            base_value_changed = !self.base_value.borrow().as_ref().is_some_and(|base| equals(base, &v));
            self.base_value.replace(Some(v));
            self.state.base_priority.set(priority);
            if let Some((uncommon, value)) = uncoerced {
                uncommon.uncoerced_base_value.replace(value);
            }
        }

        if value_changed {
            (write.operations.notify_value_changed)(self, owner, old_value);
        } else if base_value_changed {
            (write.operations.notify_base_value_changed)(self, owner);
        }
    }
}

impl<T: PropertyValue> EffectiveValue<T> {
    // Its address is taken only by an explicit inheritance registration.
    // Keep the original owned-value frame: the selected source's metadata
    // supplies defaults, then old/new clones, equality and notification run
    // in their original order. The checked projection adds no owning handle.
    pub(crate) fn notify_inherited_value_changed(
        source: &dyn EffectiveValueDyn,
        owner: &FerroObject,
        old_value: Option<&dyn EffectiveValueDyn>,
        new_value: Option<&dyn EffectiveValueDyn>,
    ) {
        let source = (source as &dyn Any).downcast_ref::<Self>().expect("inherited value has the property's Rust type");
        debug_assert!(old_value.is_some() || new_value.is_some());
        let default = || source.metadata.default_value().clone();
        let o = old_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let n = new_value.map(|v| cast_effective_value::<T>(v).value()).unwrap_or_else(default);
        let priority = if new_value.is_some() { BindingPriority::Inherited } else { BindingPriority::Unset };
        if !value_equals(&o, &n) {
            owner.raise_inherited_property_changed(source.state.property, &o, &n, priority);
        }
    }

    pub fn new(owner: &FerroObject, property: &'static StyledProperty<T>, inherited: Option<&EffectiveValue<T>>) -> Self {
        let metadata = property.get_metadata_for(owner);
        let value = match inherited {
            Some(inherited) => inherited.value(),
            None => metadata.default_value().clone(),
        };
        let uncommon = metadata.coercion().map(|coercion| coercion.initialize_uncommon(&value));
        Self {
            metadata,
            state: EffectiveValueState::new(property),
            value: RefCell::new(value),
            base_value: RefCell::new(None),
            uncommon,
        }
    }

    pub fn set_local_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, BindingPriority::LocalValue, false, false);
    }

    pub fn set_current_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, self.state.priority.get(), true, false);
    }

    pub fn set_coerced_default_value_and_raise(&self, owner: &FerroObject, value: T) {
        self.set_and_raise_core(owner, value, self.state.priority.get(), false, true);
    }

    /// The base value, if there is one.
    pub fn try_get_base_value(&self) -> Option<T> {
        if self.state.base_priority.get() != BindingPriority::Unset {
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

    fn set_and_raise_core(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        is_overridden_current_value: bool,
        is_coerced_default_value: bool,
    ) {
        // A value is coerced when the metadata has a coercer (whose fields
        // are the uncommon fields) and the value is not the coerced default.
        // That branch is a function the coercer registered: only value types
        // whose properties coerce compile it (#576).
        if let (Some(_), Some(coercion), false) = (&self.uncommon, self.metadata.coercion(), is_coerced_default_value) {
            let mut write = CoercedWrite {
                value: Some(value),
                priority,
                is_overridden_current_value,
                operations: CoercionOperations {
                    equals: value_equals::<T>,
                    notify_value_changed: Self::notify_value_changed,
                    notify_base_value_changed: Self::notify_base_value_changed,
                },
            };
            coercion.set_and_raise(self, owner, &mut write);
        } else {
            self.set_and_raise_uncoerced(owner, value, priority, is_overridden_current_value, is_coerced_default_value);
        }
    }

    // `set_and_raise_core` of a value that is not coerced: `v` is the value
    // itself, not a copy of it, and there is no uncoerced value to keep.
    fn set_and_raise_uncoerced(
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

        self.state.is_overriden_current_value.set(is_overridden_current_value);
        self.state.is_coerced_default_value.set(is_coerced_default_value);

        let v = value;

        if priority <= self.state.priority.get() {
            value_changed = !value_equals(&*self.value.borrow(), &v);
            // `replace` releases the borrow before the previous value is
            // dropped: dropping a value may run code that reads the property.
            self.value.replace(v.clone());
            self.state.priority.set(priority);
        }

        if priority <= self.state.base_priority.get() && priority >= BindingPriority::LocalValue {
            base_value_changed = !self.base_value.borrow().as_ref().is_some_and(|base| value_equals(base, &v));
            self.base_value.replace(Some(v));
            self.state.base_priority.set(priority);
        }

        if value_changed {
            self.notify_value_changed(owner, old_value);
        } else if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }

    // The typed end of the shared notification (`EffectiveValueState::
    // notify_value_changed`): the copies of the new value live in this frame,
    // so they drop as they did, the copy for inheritance first, then the
    // announced copy, then the old value.
    fn notify_value_changed(&self, owner: &FerroObject, old_value: T) {
        let mut new_value = None::<T>;
        let mut inherited_value = None::<T>;
        self.state.notify_value_changed(owner, &self.value, &old_value, &mut new_value, &mut inherited_value);
    }

    fn notify_base_value_changed(&self, owner: &FerroObject) {
        let base = self.base_value.borrow().clone();
        if let Some(base) = base {
            owner.raise_property_changed(self.state.property, None, &base, self.state.base_priority.get(), false);
        }
    }
}

/// Downcasts an untyped effective value to its typed form.
#[inline]
pub(crate) fn cast_effective_value<T: PropertyValue>(value: &dyn EffectiveValueDyn) -> &EffectiveValue<T> {
    debug_assert!((value as &dyn Any).is::<EffectiveValue<T>>());
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
    fn state(&self) -> &EffectiveValueState {
        &self.state
    }

    fn boxed_value(&self) -> BoxedValue {
        Rc::new(self.value())
    }

    fn end_reevaluation(&self, owner: &FerroObject) {
        if self.state.priority.get() == BindingPriority::Unset {
            // Keep the priority short circuit before even reading metadata.
            if let Some(coercion) = self.metadata.coercion() {
                coercion.end_reevaluation(self, owner, DefaultCoercionOperations {
                    equals: value_equals::<T>,
                    set_default: Self::set_coerced_default_value_and_raise,
                });
            }
        }
    }

    fn apply_entry(&self, owner: &FerroObject, entry: &dyn IValueEntry, priority: BindingPriority) {
        let value = self.get_entry_value(entry);
        self.set_and_raise_core(owner, value, priority, false, false);
    }

    fn remove_animation_and_raise(&self, owner: &FerroObject) {
        debug_assert!(self.state.priority.get() != BindingPriority::Animation);
        debug_assert!(self.state.base_priority.get() != BindingPriority::Unset);
        self.state.update_value_entry(None, BindingPriority::Animation);
        let base = self.base_value.borrow().clone().expect("base value is set");
        self.set_and_raise_core(owner, base, self.state.base_priority.get(), false, false);
    }

    fn coerce_value(&self, owner: &FerroObject) {
        let Some(coercion) = self.metadata.coercion() else { return };
        // These typed leaves are the only PropertyValue-specific operations
        // of explicit recoercion. The algorithm address comes from the actual
        // coercer registration, never from this all-value-types vtable entry.
        coercion.apply(self, owner, CoercionOperations {
            equals: value_equals::<T>,
            notify_value_changed: Self::notify_value_changed,
            notify_base_value_changed: Self::notify_base_value_changed,
        });
    }

    fn raise_unset(
        &self,
        owner: &FerroObject,
        inherited: Option<&dyn EffectiveValueDyn>,
        clear_data_validation: bool,
    ) {
        let (new_value, priority) = match inherited {
            Some(i) => (cast_effective_value::<T>(i).value(), BindingPriority::Inherited),
            None => (self.metadata.default_value().clone(), BindingPriority::Unset),
        };

        let old_value = self.value();
        let changed = !value_equals(&new_value, &old_value);
        self.state.finish_unset(owner, &old_value, &new_value, priority, changed, clear_data_validation);
    }
}

#[cfg(test)]
#[path = "effective_value_coercion_tests.rs"]
mod coercion_tests;

#[cfg(test)]
#[path = "effective_value_inheritance_tests.rs"]
mod inheritance_tests;

#[cfg(test)]
#[path = "effective_value_construction_tests.rs"]
mod construction_tests;
