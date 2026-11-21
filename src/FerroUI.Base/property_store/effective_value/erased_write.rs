//! The write of an effective value and its change notification, compiled once.
//!
//! [`EffectiveValue<T>`] keeps its typed layout: a read is still the typed read of a field. What
//! the write does between the typed operations (the priorities, the base value, the order of the
//! notifications) depends on no value type, so it is one function here, and each storage type
//! contributes only the small typed leaves of [`WriteLeaves`].
//!
//! # Soundness
//!
//! The values of a write stay in the frame of the typed caller, in `Option<T>` slots: a value is
//! moved by `Option::take`, so it is moved at most once, and whatever is left in a slot is dropped
//! by the caller, on unwinding as well. The erased functions only pass the addresses of the slots
//! back to the leaves of the same `T`.
//!
//! That pairing is not an invariant of the callers: [`ErasedWrite`] and [`ErasedNotify`] have
//! private fields and one generic constructor each, which takes the effective value and its slots
//! as `&EffectiveValue<T>` and `&mut Option<T>` of one `T`. The type of the leaves and the type
//! of the slots are therefore the same by the signature of the constructor, and no value of
//! either struct can be made any other way.

use super::{value_equals, with_values, EffectiveValue, EffectiveValueState};
use crate::data::BindingPriority;
use crate::{FerroObject, StyledValue};
use std::any::Any;
use std::marker::PhantomData;
use std::ptr;
use std::rc::Rc;

/// The typed operations of a write.
///
/// # Safety
///
/// Every `*const ()` and `*mut ()` parameter must be the address of a live `Option<T>`, where `T`
/// is the storage type of the implementing effective value, valid for the access for the length
/// of the call (`old` of [`with_new_value`](Self::with_new_value) and
/// [`with_new_value_again`](Self::with_new_value_again): a live `T`). Only [`ErasedWrite`] and
/// [`ErasedNotify`] call these functions, with the slots they were constructed with.
unsafe trait WriteLeaves {
    /// Whether the value in the slot `new` differs from the current value.
    unsafe fn value_differs(&self, new: *const ()) -> bool;
    /// Moves the value of the slot `new` in as the current value, and the previous one to `old`.
    unsafe fn replace_value(&self, new: *mut (), old: *mut ());
    /// Whether the base value differs from the current value.
    fn base_differs_from_value(&self) -> bool;
    /// Whether the base value differs from the value in the slot `new`.
    unsafe fn base_differs(&self, new: *const ()) -> bool;
    /// Makes the value the base value as well.
    fn set_base_to_value(&self);
    /// Moves the value of the slot `new` to the base value.
    unsafe fn move_new_to_base(&self, new: *mut ());
    /// Moves the previous value of the slot `old` to the base value, leaving a copy in the slot
    /// if `keep_copy`.
    unsafe fn move_old_to_base(&self, old: *mut (), keep_copy: bool);
    /// Raises the change of the value, with the previous value of the slot `old`.
    unsafe fn notify_changed(&self, owner: &FerroObject, old: *mut ());
    /// Raises the change of the base value.
    fn notify_base_changed(&self, owner: &FerroObject);
    /// Calls `f` with the old value and a copy of the current value, the copy kept in the slot
    /// `keep`, so that it lives until the caller of the notification returns.
    unsafe fn with_new_value(
        &self,
        old: *const (),
        keep: *mut (),
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    );
    /// Calls `f` with the old value and a copy of the current value that lasts for the call.
    unsafe fn with_new_value_again(&self, old: *const (), f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any));
}

// SAFETY: every function reads its parameters as `Option<T>` (or `T`, for `old` of the two
// `with_new_value` functions), the type the contract of the trait states for this `T`.
unsafe impl<T: StyledValue> WriteLeaves for EffectiveValue<T> {
    unsafe fn value_differs(&self, new: *const ()) -> bool {
        // SAFETY: the contract of the trait.
        let new = unsafe { &*new.cast::<Option<T>>() };
        !value_equals(&*self.value.borrow(), new.as_ref().expect("the new value of a write"))
    }

    unsafe fn replace_value(&self, new: *mut (), old: *mut ()) {
        // SAFETY: the contract of the trait; the two slots are distinct locals of the caller.
        let (new, old) = unsafe { (&mut *new.cast::<Option<T>>(), &mut *old.cast::<Option<T>>()) };
        // `replace` releases the borrow before anything else runs: dropping a value may run code
        // that reads the property, and the previous value is only dropped by the typed caller.
        *old = Some(self.value.replace(new.take().expect("the new value of a write")));
    }

    fn base_differs_from_value(&self) -> bool {
        let value = self.value.borrow();
        let differs = !self.with_base_value(|base| base.is_some_and(|base| value_equals(base, &value)));
        drop(value);
        differs
    }

    unsafe fn base_differs(&self, new: *const ()) -> bool {
        // SAFETY: the contract of the trait.
        let new = unsafe { &*new.cast::<Option<T>>() };
        let new = new.as_ref().expect("the new value of a write");
        !self.with_base_value(|base| base.is_some_and(|base| value_equals(base, new)))
    }

    fn set_base_to_value(&self) {
        EffectiveValue::set_base_to_value(self);
    }

    unsafe fn move_new_to_base(&self, new: *mut ()) {
        // SAFETY: the contract of the trait.
        let new = unsafe { &mut *new.cast::<Option<T>>() };
        self.set_base_value(new.take().expect("the new value of a write"));
    }

    unsafe fn move_old_to_base(&self, old: *mut (), keep_copy: bool) {
        // SAFETY: the contract of the trait.
        let old = unsafe { &mut *old.cast::<Option<T>>() };
        let previous = old.take().expect("the previous value of a write");
        // The copy first: a clone may read the property, which still has its base value.
        *old = keep_copy.then(|| previous.clone());
        self.base_value.set(Some(Rc::new(previous)));
    }

    unsafe fn notify_changed(&self, owner: &FerroObject, old: *mut ()) {
        // SAFETY: the contract of the trait.
        let old = unsafe { &mut *old.cast::<Option<T>>() };
        self.notify_value_changed(owner, old.take().expect("the old value of a changed value is kept"));
    }

    fn notify_base_changed(&self, owner: &FerroObject) {
        self.notify_base_value_changed(owner);
    }

    unsafe fn with_new_value(
        &self,
        old: *const (),
        keep: *mut (),
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    ) {
        // SAFETY: the contract of the trait: `old` is a live `T`, `keep` a live `Option<T>`.
        let (old, keep) = unsafe { (&*old.cast::<T>(), &mut *keep.cast::<Option<T>>()) };
        let new = keep.insert(self.value());
        with_values(self.state.property, Some(old), new, |old, new| f(old, new));
    }

    unsafe fn with_new_value_again(&self, old: *const (), f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any)) {
        // SAFETY: the contract of the trait: `old` is a live `T`.
        let old = unsafe { &*old.cast::<T>() };
        let new = self.value();
        with_values(self.state.property, Some(old), &new, |old, new| f(old, new));
    }
}

/// A write in progress: an effective value and the two slots of the typed frame that holds the
/// values of the write. See the module documentation for why the pairing is sound.
pub(super) struct ErasedWrite<'a> {
    leaves: &'a dyn WriteLeaves,
    new: *mut (),
    old: *mut (),
    slots: PhantomData<&'a mut ()>,
}

impl<'a> ErasedWrite<'a> {
    /// The write of the value in `new` to `target`, whose previous value goes to `old`.
    #[inline(always)]
    pub(super) fn new<T: StyledValue>(
        target: &'a EffectiveValue<T>,
        new: &'a mut Option<T>,
        old: &'a mut Option<T>,
    ) -> Self {
        Self {
            leaves: target,
            new: ptr::from_mut(new).cast(),
            old: ptr::from_mut(old).cast(),
            slots: PhantomData,
        }
    }

    #[inline]
    fn value_differs(&self) -> bool {
        // SAFETY: `leaves` and `new` were made together by `Self::new` for one `T`.
        unsafe { self.leaves.value_differs(self.new) }
    }

    #[inline]
    fn replace_value(&self) {
        // SAFETY: as above, for both slots.
        unsafe { self.leaves.replace_value(self.new, self.old) }
    }

    #[inline]
    fn base_differs(&self) -> bool {
        // SAFETY: as above.
        unsafe { self.leaves.base_differs(self.new) }
    }

    #[inline]
    fn move_new_to_base(&self) {
        // SAFETY: as above.
        unsafe { self.leaves.move_new_to_base(self.new) }
    }

    #[inline]
    fn move_old_to_base(&self, keep_copy: bool) {
        // SAFETY: as above.
        unsafe { self.leaves.move_old_to_base(self.old, keep_copy) }
    }

    #[inline]
    fn notify_changed(&self, owner: &FerroObject) {
        // SAFETY: as above.
        unsafe { self.leaves.notify_changed(owner, self.old) }
    }
}

/// The write of a value of a property without a coercion callback, for every storage type.
#[inline(never)]
pub(super) fn write_value(
    state: &EffectiveValueState,
    write: ErasedWrite<'_>,
    owner: &FerroObject,
    priority: BindingPriority,
    is_overridden_current_value: bool,
    is_coerced_default_value: bool,
) {
    state.is_overriden_current_value.set(is_overridden_current_value);
    state.is_coerced_default_value.set(is_coerced_default_value);

    let sets_base = priority <= state.base_priority.get() && priority >= BindingPriority::LocalValue;
    let mut value_changed = false;
    let mut base_value_changed = false;

    if priority <= state.priority.get() {
        value_changed = write.value_differs();
        // Before the write: dropping the previous value may read the property.
        owner.values().advance_value_generation();
        write.replace_value();
        if sets_base {
            // The value and the base value at once, which is every set without an animation over
            // the value: the value is kept once, and a base value that was the value changed if
            // the value did.
            if state.base_is_value.get() {
                base_value_changed = value_changed;
            } else {
                base_value_changed = write.leaves.base_differs_from_value();
                write.leaves.set_base_to_value();
            }
            state.priority.set(priority);
            state.base_priority.set(priority);
        } else {
            // The value alone. A base value that was the value stays what it was: the value that
            // is replaced becomes it.
            if state.base_is_value.get() {
                write.move_old_to_base(value_changed);
                state.base_is_value.set(false);
            }
            state.priority.set(priority);
        }
    } else if sets_base {
        base_value_changed = write.base_differs();
        write.move_new_to_base();
        state.base_priority.set(priority);
    }

    if value_changed {
        write.notify_changed(owner);
    } else if base_value_changed {
        write.leaves.notify_base_changed(owner);
    }
}

/// A change notification in progress: an effective value, the old value and the slot that keeps
/// the copy of the new value until the typed caller returns.
pub(super) struct ErasedNotify<'a> {
    leaves: &'a dyn WriteLeaves,
    old: *const (),
    keep: *mut (),
    slots: PhantomData<(&'a (), &'a mut ())>,
}

impl<'a> ErasedNotify<'a> {
    /// The notification of a change of `target` from `old`; `keep` is an empty slot of the caller.
    #[inline(always)]
    pub(super) fn new<T: StyledValue>(target: &'a EffectiveValue<T>, old: &'a T, keep: &'a mut Option<T>) -> Self {
        Self { leaves: target, old: ptr::from_ref(old).cast(), keep: ptr::from_mut(keep).cast(), slots: PhantomData }
    }
}

/// The notification of a change of a value, for every storage type.
#[inline(never)]
pub(super) fn raise_value_changed(state: &EffectiveValueState, notify: ErasedNotify<'_>, owner: &FerroObject) {
    let property = state.property;
    let notifying = property.notifying();
    if let Some(notifying) = notifying {
        notifying(owner, true);
    }
    // SAFETY: `leaves`, `old` and `keep` were made together by `ErasedNotify::new` for one `T`.
    unsafe {
        notify.leaves.with_new_value(notify.old, notify.keep, &mut |old, new| {
            // Read after the copy of the new value was made.
            let priority = state.priority.get();
            owner.raise_property_changed_values(property, old, new, priority, true);
        });
    }
    if property.inherits() {
        // A change handler may have changed the value again: inheritance children are told the
        // value as it is now, not the value that was announced to the handlers of this object.
        // SAFETY: as above.
        unsafe {
            notify.leaves.with_new_value_again(notify.old, &mut |old, new| {
                owner.values().on_inherited_effective_value_changed(
                    owner,
                    property,
                    old.expect("the old value"),
                    new,
                );
            });
        }
    }
    if let Some(notifying) = notifying {
        notifying(owner, false);
    }
}

#[cfg(test)]
mod tests {
    use crate::data::BindingPriority;
    use crate::{FerroObject, FerroProperty, StyledProperty, StyledPropertyOptions};
    use std::cell::Cell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A value that counts how many of it are alive, and whose comparison can panic.
    struct Tracked {
        number: i32,
        alive: Rc<Cell<isize>>,
        panic_on_compare: Rc<Cell<bool>>,
    }

    crate::styled_value!(Tracked);

    impl Tracked {
        fn new(number: i32, like: (&Rc<Cell<isize>>, &Rc<Cell<bool>>)) -> Self {
            like.0.set(like.0.get() + 1);
            Self { number, alive: like.0.clone(), panic_on_compare: like.1.clone() }
        }
    }

    impl Clone for Tracked {
        fn clone(&self) -> Self {
            Self::new(self.number, (&self.alive, &self.panic_on_compare))
        }
    }

    impl PartialEq for Tracked {
        fn eq(&self, other: &Self) -> bool {
            assert!(!self.panic_on_compare.get(), "the comparison panics");
            self.number == other.number
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.alive.set(self.alive.get() - 1);
        }
    }

    type Fixture = (crate::Ref<FerroObject>, &'static StyledProperty<Tracked>, Rc<Cell<isize>>, Rc<Cell<bool>>);

    fn fixture() -> Fixture {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let (alive, panics) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(false)));
        let name = format!("ErasedWrite{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let default = Tracked::new(0, (&alive, &panics));
        let property = FerroProperty::register_with::<FerroObject, _>(&name, StyledPropertyOptions::new(default));
        (FerroObject::new(), property, alive, panics)
    }

    #[test]
    fn a_written_value_is_moved_once_and_the_previous_one_dropped_once() {
        let (owner, property, alive, panics) = fixture();
        let before = alive.get();
        for number in [1, 2, 2, 3] {
            owner.set_value(property, Tracked::new(number, (&alive, &panics)));
            assert_eq!(alive.get(), before + 1);
            assert_eq!(owner.get_value(property).number, number);
        }
    }

    #[test]
    fn a_value_under_an_animation_is_moved_to_the_base_value_once() {
        let (owner, property, alive, panics) = fixture();
        let before = alive.get();
        owner.set_value(property, Tracked::new(1, (&alive, &panics)));
        let animation =
            owner.set_value_with_priority(property, Tracked::new(5, (&alive, &panics)), BindingPriority::Animation);
        // The base value, the value, and the one the frame of the animation keeps.
        assert_eq!(alive.get(), before + 3);
        owner.set_value(property, Tracked::new(2, (&alive, &panics)));
        assert_eq!(alive.get(), before + 3);
        assert_eq!(owner.get_value(property).number, 5);
        drop(animation);
    }

    #[test]
    fn a_comparison_that_panics_leaves_every_value_dropped_once() {
        let (owner, property, alive, panics) = fixture();
        owner.set_value(property, Tracked::new(1, (&alive, &panics)));
        let before = alive.get();
        panics.set(true);
        let value = Tracked::new(2, (&alive, &panics));
        let result = catch_unwind(AssertUnwindSafe(|| owner.set_value(property, value)));
        panics.set(false);
        assert!(result.is_err());
        // The new value was still in its slot of the typed frame, which dropped it.
        assert_eq!(alive.get(), before);
        assert_eq!(owner.get_value(property).number, 1);
    }
}
