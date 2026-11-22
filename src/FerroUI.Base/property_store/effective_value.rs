use super::{entry_ptr_eq, IValueEntry};
use crate::data::{BindingPriority, BindingValueType};
use crate::ferro_property::value_equals;
use crate::ferro_property_metadata::{CoerceValueCallback, CoercionOperations, DefaultCoercionOperations};
use crate::utilities::OptionCell;
use crate::{BoxedValue, EnumValue, FerroObject, FerroProperty, StyledPropertyMetadata, StyledValue};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The untyped interface of an [`EffectiveValue`].
pub(crate) trait EffectiveValueDyn: Any {
    // A single projection gives the common store its property identity,
    // priorities and subscriptions for every payload type.
    fn state(&self) -> &EffectiveValueState;
    fn boxed_value(&self) -> BoxedValue;
    /// The base value in a box of its own, if there is one.
    fn boxed_base_value(&self) -> Option<BoxedValue>;
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

/// The write of a value of a property with a coercion callback (`EffectiveValue::set_and_raise_coerced`).
type CoercedSet<T> = fn(&EffectiveValue<T>, &FerroObject, T, BindingPriority, bool, bool);

pub(crate) struct UncommonFields<T> {
    coerce: CoerceValueCallback<T>,
    uncoerced_value: RefCell<T>,
    uncoerced_base_value: RefCell<T>,
    /// The write of a value with the callback. Its address is taken here alone, where a coercion callback
    /// was registered, and not by the write of every value type.
    set: CoercedSet<T>,
    /// The typed leaves of that write (the comparison and the two notifications), named once where the
    /// value is made: named in the write of every value type, each copy of the write kept copies of them.
    operations: CoercionOperations<T>,
}

impl<T: Clone + 'static> UncommonFields<T> {
    pub(crate) fn new(coerce: &CoerceValueCallback<T>, value: &T, operations: CoercionOperations<T>) -> Self {
        // Preserve the original map closure's initialization and partial-drop
        // order. In particular, the callback is cloned before either T clone.
        Self {
            coerce: coerce.clone(),
            uncoerced_value: RefCell::new(value.clone()),
            uncoerced_base_value: RefCell::new(value.clone()),
            set: EffectiveValue::<T>::set_and_raise_coerced,
            operations,
        }
    }
}

/// Entry selection and subscription lifetime are identical for every styled
/// property. Only the resolved values and their coercion callback depend on T.
pub(crate) struct EffectiveValueState {
    property: &'static FerroProperty,
    priority: Cell<BindingPriority>,
    base_priority: Cell<BindingPriority>,
    value_entry: OptionCell<Rc<dyn IValueEntry>>,
    base_value_entry: OptionCell<Rc<dyn IValueEntry>>,
    is_overriden_current_value: Cell<bool>,
    is_coerced_default_value: Cell<bool>,
    /// The base value is the value (`EffectiveValue::base_value`): what a value without an animation over it
    /// has, and it is then kept once.
    base_is_value: Cell<bool>,
}

impl EffectiveValueState {
    fn new(property: &'static FerroProperty) -> Self {
        Self {
            property,
            priority: Cell::new(BindingPriority::Unset),
            base_priority: Cell::new(BindingPriority::Unset),
            value_entry: OptionCell::new(),
            base_value_entry: OptionCell::new(),
            is_overriden_current_value: Cell::new(false),
            is_coerced_default_value: Cell::new(false),
            base_is_value: Cell::new(false),
        }
    }

    fn update_value_entry(&self, entry: Option<&Rc<dyn IValueEntry>>, priority: BindingPriority) {
        debug_assert!(priority != BindingPriority::LocalValue);

        fn same(a: Option<&Rc<dyn IValueEntry>>, b: Option<&Rc<dyn IValueEntry>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => entry_ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }

        fn replace(slot: &OptionCell<Rc<dyn IValueEntry>>, entry: Option<&Rc<dyn IValueEntry>>) {
            if !slot.with(|current| same(current, entry)) {
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
                self.base_value_entry.set(entry);
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
        // The entry is read outside the cell: what it answers with may read this value.
        let has = |slot: &OptionCell<Rc<dyn IValueEntry>>| {
            slot.get().map(|e| e.get_data_validation_state().is_some())
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
        self.state().value_entry.get()
    }

    pub(crate) fn base_value_entry(&self) -> Option<Rc<dyn IValueEntry>> {
        self.state().base_value_entry.get()
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
///
/// `T` is the storage type of the property's value type
/// ([`StyledValue::Storage`]): the store is compiled once for each storage
/// type, not once for each value type (docs/binary-size.md, section 4.27).
// Keep the representation unbounded so Clone-only public metadata can name
// its cold algorithm. Construction, store access and notification impls require
// a StyledValue storage type; none of the typed fields is erased or recast.
pub(crate) struct EffectiveValue<T> {
    metadata: Rc<StyledPropertyMetadata<T>>,
    state: EffectiveValueState,
    value: RefCell<T>,
    /// The base value where it is not the value: under an animation, and while a value is evaluated again. Where
    /// it is the value (`EffectiveValueState::base_is_value`, nearly every value) this holds nothing and the
    /// value is kept once.
    base_value: OptionCell<Rc<T>>,
    /// What a property with a coercion callback keeps: boxed, so that a value
    /// of a property without one (nearly every value) is not the larger for
    /// it.
    uncommon: Option<Box<UncommonFields<T>>>,
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

    /// Calls `read` with the base value, whichever of the two it is kept as.
    pub(crate) fn with_base_value<R>(&self, read: impl FnOnce(Option<&T>) -> R) -> R {
        if self.state.base_is_value.get() {
            read(Some(&self.value.borrow()))
        } else {
            // Outside the cell: `read` compares values, and a comparison may read this one.
            let base = self.base_value.get();
            read(base.as_deref())
        }
    }

    /// A clone of the base value.
    fn base_value(&self) -> Option<T> {
        self.with_base_value(|base| base.cloned())
    }

    /// Replaces the value alone. A base value that was the value stays what it was: the value that is replaced
    /// becomes it.
    fn set_value_alone(&self, value: T) {
        // `replace` releases the borrow before the previous value is dropped: dropping a value may run code
        // that reads the property.
        let previous = self.value.replace(value);
        if self.state.base_is_value.get() {
            self.base_value.set(Some(Rc::new(previous)));
            self.state.base_is_value.set(false);
        }
    }

    /// Replaces the base value alone, with a value of its own.
    pub(crate) fn set_base_value(&self, base: T) {
        let previous = self.base_value.replace(Some(Rc::new(base)));
        self.state.base_is_value.set(false);
        drop(previous);
    }

    /// Makes the value the base value as well.
    fn set_base_to_value(&self) {
        self.state.base_is_value.set(true);
        self.base_value.set(None);
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

    /// The write of a value of a property with a coercion callback: what `set_and_raise_core` does for every
    /// other property, with the coercion and the two uncoerced values. It is reached through the uncommon
    /// fields, which only the registration of a coercion callback makes (docs/binary-size.md, section 4.17), so
    /// a value type without one does not carry it.
    pub(crate) fn set_and_raise_coerced(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        is_overridden_current_value: bool,
        is_coerced_default_value: bool,
    ) {
        let operations = &self.uncommon.as_ref().expect("a coerced write has the uncommon fields").operations;
        let old_value = self.value();
        let mut value_changed = false;
        let mut base_value_changed = false;

        self.state.is_overriden_current_value.set(is_overridden_current_value);
        self.state.is_coerced_default_value.set(is_coerced_default_value);

        // `v = value`, or the coerced value. The value as it was given is
        // only read again where it is stored uncoerced, so without a
        // coercion `v` is the value itself and not a copy of it.
        let (v, uncoerced) = match &self.uncommon {
            Some(uncommon) if !is_coerced_default_value => {
                ((uncommon.coerce)(owner, value.clone()), Some((uncommon, value)))
            }
            _ => (value, None),
        };

        let sets_base = |state: &EffectiveValueState| {
            priority <= state.base_priority.get() && priority >= BindingPriority::LocalValue
        };

        if priority <= self.state.priority.get() {
            value_changed = !(operations.equals)(&*self.value.borrow(), &v);
            // Before the write: dropping the previous value may read the property.
            owner.values().advance_value_generation();
            if sets_base(&self.state) {
                // The value and the base value at once, which is every set without an animation over the
                // value: `v` is kept once, and a base value that was the value changed if the value did.
                if self.state.base_is_value.get() {
                    // `replace` releases the borrow before the previous value is dropped: dropping a value
                    // may run code that reads the property.
                    drop(self.value.replace(v));
                    base_value_changed = value_changed;
                } else {
                    drop(self.value.replace(v));
                    let value = self.value.borrow();
                    base_value_changed =
                        !self.with_base_value(|base| base.is_some_and(|base| (operations.equals)(base, &value)));
                    drop(value);
                    self.set_base_to_value();
                }
                self.state.priority.set(priority);
                if let Some((uncommon, value)) = &uncoerced {
                    uncommon.uncoerced_value.replace(value.clone());
                }
                self.state.base_priority.set(priority);
                if let Some((uncommon, value)) = uncoerced {
                    uncommon.uncoerced_base_value.replace(value);
                }
            } else {
                self.set_value_alone(v);
                self.state.priority.set(priority);
                if let Some((uncommon, value)) = uncoerced {
                    uncommon.uncoerced_value.replace(value);
                }
            }
        } else if sets_base(&self.state) {
            base_value_changed = !self.with_base_value(|base| base.is_some_and(|base| (operations.equals)(base, &v)));
            self.set_base_value(v);
            self.state.base_priority.set(priority);
            if let Some((uncommon, value)) = uncoerced {
                uncommon.uncoerced_base_value.replace(value);
            }
        }

        if value_changed {
            (operations.notify_value_changed)(self, owner, old_value);
        } else if base_value_changed {
            (operations.notify_base_value_changed)(self, owner);
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
            owner.values().advance_value_generation();
            self.set_value_alone(v.clone());
            value_changed = true;
            if let Some(uncommon) = &self.uncommon {
                uncommon.uncoerced_value.replace(value);
            }
        }

        if !self.with_base_value(|base| base.is_some_and(|base| (operations.equals)(base, &bv))) {
            // The base value becomes `v`: the value itself where the value has just become it.
            if value_changed {
                self.set_base_to_value();
            } else {
                self.set_base_value(v);
            }
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
}

/// Calls `f` with `old` and `new`, values of the storage type `T`, as values of the value type
/// of `property`: as they are where `T` is the value type itself (decided when the function is
/// compiled), the static members for an enumeration, else converted by the property
/// ([`PropertyRoutes::route_with_values`]), one call that only a change of a value of another
/// shared storage type makes.
///
/// [`PropertyRoutes::route_with_values`]: crate::ferro_property::PropertyRoutes::route_with_values
#[inline]
pub(crate) fn with_values<T: StyledValue>(
    property: &'static FerroProperty,
    old: Option<&T>,
    new: &T,
    f: impl FnOnce(Option<&dyn Any>, &dyn Any),
) {
    if T::SHARED_STORAGE {
        // A member of an enumeration: the static member itself, found by its stored value, with
        // no call to the property.
        let members = property.members();
        if let Some(new) = new.member_of(members) {
            let old = old.map(|old| old.member_of(members).expect("a member of the same enumeration"));
            return f(old, new);
        }
        let mut f = Some(f);
        property.routes().route_with_values(old.map(|old| old as &dyn Any), new, &mut |old, new| {
            if let Some(f) = f.take() {
                f(old, new);
            }
        });
    } else {
        f(old.map(|old| old as &dyn Any), new);
    }
}

/// A stored value of `property` in a box of its own, holding the value type of the property.
pub(crate) fn box_stored<T: StyledValue>(property: &'static FerroProperty, value: &T) -> BoxedValue {
    if T::SHARED_STORAGE {
        let mut boxed = None;
        with_values(property, None, value, |_, value| boxed = property.styled_codec().box_value(value));
        boxed.expect("the property boxes a value of its own type")
    } else {
        Rc::new(value.clone())
    }
}

impl<T: StyledValue> EffectiveValue<T> {
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
            let property = source.state.property;
            with_values(property, Some(&o), &n, |o, n| {
                owner.raise_inherited_property_changed(property, o.expect("the old value"), n, priority);
            });
        }
    }

    /// The effective value of `property`, whose value type has the storage type `T`, with the
    /// metadata of the property for the type of its owner (as the store reads it,
    /// `StyledPropertyMetadata::for_storage`).
    pub fn new(
        property: &'static FerroProperty,
        metadata: Rc<StyledPropertyMetadata<T>>,
        inherited: Option<&EffectiveValue<T>>,
    ) -> Self {
        debug_assert!(
            T::SHARED_STORAGE || property.property_type() == std::any::TypeId::of::<T>(),
            "a storage type that other value types share states it (StyledValue::SHARED_STORAGE)"
        );
        let value = match inherited {
            Some(inherited) => inherited.value(),
            None => metadata.default_value().clone(),
        };
        let uncommon = metadata.coercion().map(|coercion| {
            Box::new(coercion.initialize_uncommon(
                &value,
                CoercionOperations {
                    equals: value_equals::<T>,
                    notify_value_changed: Self::notify_value_changed,
                    notify_base_value_changed: Self::notify_base_value_changed,
                },
            ))
        });
        Self {
            metadata,
            state: EffectiveValueState::new(property),
            value: RefCell::new(value),
            base_value: OptionCell::new(),
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
            self.base_value()
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
                // marker. A value of a type that is not its own storage type
                // is converted by the property.
                if T::SHARED_STORAGE {
                    if let Some(v) = self.stored_from_value(boxed.as_any()) {
                        return v;
                    }
                } else if let Some(v) = boxed.downcast_ref::<T>() {
                    return v.clone();
                }
            }
        }
        self.metadata.default_value().clone()
    }

    /// `value`, of the value type of the property, as this shared storage type.
    #[inline]
    fn stored_from_value(&self, value: &dyn Any) -> Option<T> {
        let codec = self.state.property.styled_codec();
        // Decided when the function is compiled: an enumeration is converted with one check of
        // its type.
        if TypeId::of::<T>() == TypeId::of::<EnumValue>() {
            let stored = codec.enum_storage(value)?;
            return (&stored as &dyn Any).downcast_ref::<T>().cloned();
        }
        let mut out = None;
        codec.copy_value(value, &mut out);
        out
    }

    fn set_and_raise_core(
        &self,
        owner: &FerroObject,
        value: T,
        priority: BindingPriority,
        is_overridden_current_value: bool,
        is_coerced_default_value: bool,
    ) {
        // A property with a coercion callback: its write is made with the callback (the uncommon fields carry
        // it), so that the write of every other value type is only what follows.
        if let Some(uncommon) = &self.uncommon {
            return (uncommon.set)(self, owner, value, priority, is_overridden_current_value, is_coerced_default_value);
        }

        self.state.is_overriden_current_value.set(is_overridden_current_value);
        self.state.is_coerced_default_value.set(is_coerced_default_value);

        let sets_base = priority <= self.state.base_priority.get() && priority >= BindingPriority::LocalValue;
        let mut value_changed = false;
        let mut base_value_changed = false;
        // The value that is replaced is the old value of the change. It is kept to the end, as the copy of it
        // was that this function began with: what dropping it does happens after the notification.
        let mut old_value = None;

        if priority <= self.state.priority.get() {
            value_changed = !value_equals(&*self.value.borrow(), &value);
            // Before the write: dropping the previous value may read the property.
            owner.values().advance_value_generation();
            let previous = self.value.replace(value);
            if sets_base {
                // The value and the base value at once, which is every set without an animation over the
                // value: the value is kept once, and a base value that was the value changed if the value did.
                if self.state.base_is_value.get() {
                    base_value_changed = value_changed;
                } else {
                    let value = self.value.borrow();
                    base_value_changed =
                        !self.with_base_value(|base| base.is_some_and(|base| value_equals(base, &value)));
                    drop(value);
                    self.set_base_to_value();
                }
                self.state.priority.set(priority);
                self.state.base_priority.set(priority);
                old_value = Some(previous);
            } else {
                // The value alone. A base value that was the value stays what it was: the value that is
                // replaced becomes it.
                if self.state.base_is_value.get() {
                    old_value = value_changed.then(|| previous.clone());
                    self.base_value.set(Some(Rc::new(previous)));
                    self.state.base_is_value.set(false);
                } else {
                    old_value = Some(previous);
                }
                self.state.priority.set(priority);
            }
        } else if sets_base {
            base_value_changed = !self.with_base_value(|base| base.is_some_and(|base| value_equals(base, &value)));
            self.set_base_value(value);
            self.state.base_priority.set(priority);
        }

        if value_changed {
            self.notify_value_changed(owner, old_value.expect("the old value of a changed value is kept"));
        } else if base_value_changed {
            self.notify_base_value_changed(owner);
        }
    }

    fn notify_value_changed(&self, owner: &FerroObject, old_value: T) {
        let notifying = self.state.property.notifying();
        if let Some(notifying) = notifying {
            notifying(owner, true);
        }
        let property = self.state.property;
        let new_value = self.value();
        let priority = self.state.priority.get();
        with_values(property, Some(&old_value), &new_value, |old, new| {
            owner.raise_property_changed_values(property, old, new, priority, true);
        });
        if property.inherits() {
            // A change handler may have changed the value again: inheritance
            // children are told the value as it is now, not the value that
            // was announced to the handlers of this object.
            let new_value = self.value();
            with_values(property, Some(&old_value), &new_value, |old, new| {
                owner.values().on_inherited_effective_value_changed(owner, property, old.expect("the old value"), new);
            });
        }
        if let Some(notifying) = notifying {
            notifying(owner, false);
        }
    }

    fn notify_base_value_changed(&self, owner: &FerroObject) {
        let base = self.base_value();
        if let Some(base) = base {
            let (property, priority) = (self.state.property, self.state.base_priority.get());
            with_values(property, None, &base, |_, base| {
                owner.raise_property_changed_values(property, None, base, priority, false);
            });
        }
    }
}

/// Downcasts an untyped effective value to its typed form, `T` the storage type of the
/// property's value type.
#[inline]
pub(crate) fn cast_effective_value<T: 'static>(value: &dyn EffectiveValueDyn) -> &EffectiveValue<T> {
    debug_assert!((value as &dyn Any).is::<EffectiveValue<T>>());
    // SAFETY: effective values are created exclusively by
    // `StyledProperty<V>::create_effective_value`, as an
    // `EffectiveValue<V::Storage>`, and stored under that property's ID, and a
    // property ID is never shared between properties of different value types.
    // A value looked up for a `StyledProperty<V>` is therefore always an
    // `EffectiveValue<V::Storage>`. Skipping the checked downcast
    // matters because reading a property value is the hottest path of the
    // property system.
    unsafe { &*(value as *const dyn EffectiveValueDyn as *const EffectiveValue<T>) }
}

impl<T: StyledValue> EffectiveValueDyn for EffectiveValue<T> {
    fn state(&self) -> &EffectiveValueState {
        &self.state
    }

    fn boxed_value(&self) -> BoxedValue {
        box_stored(self.state.property, &self.value())
    }

    fn boxed_base_value(&self) -> Option<BoxedValue> {
        self.try_get_base_value().map(|value| box_stored(self.state.property, &value))
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
        let base = self.base_value().expect("base value is set");
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
        if changed {
            with_values(self.state.property, Some(&old_value), &new_value, |old, new| {
                self.state.finish_unset(owner, old.expect("the old value"), new, priority, true, clear_data_validation);
            });
        } else {
            self.state.finish_unset(owner, &old_value, &new_value, priority, false, clear_data_validation);
        }
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

#[cfg(test)]
#[path = "effective_value_base_tests.rs"]
mod base_tests;
