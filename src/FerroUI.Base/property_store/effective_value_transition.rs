//! Shared effective-value control flow over borrowed, typed stack frames.
//!
//! The engine never owns a value, callback, metadata handle, or entry. Typed
//! callers keep their original owning scopes and lend only the individual
//! slots needed by each operation. This also keeps partially initialized
//! values in the caller when Clone, coercion, equality, or Drop unwinds.

use super::*;

#[derive(Clone, Copy)]
pub(super) enum WriteOperation {
    Prepare { is_coerced_default_value: bool },
    ReplaceEffective,
    StoreUncoercedEffective,
    ReplaceBase,
    StoreUncoercedBase,
    NotifyEffective,
    NotifyBase,
}

pub(super) trait WriteValues {
    /// Only ReplaceEffective and ReplaceBase return a change flag.
    fn apply(&mut self, owner: &FerroObject, operation: WriteOperation) -> bool;
}

/// All fields are borrows. In particular, putting the owned values in this
/// struct would change Rust's local-versus-field destruction order.
pub(super) struct WriteFrame<'slots, 'target, T> {
    pub(super) target: &'target EffectiveValue<T>,
    pub(super) input: &'slots mut Option<T>,
    pub(super) old_value: &'slots mut Option<T>,
    pub(super) value: &'slots mut Option<T>,
    pub(super) uncoerced: &'slots mut Option<(&'target UncommonFields<T>, T)>,
}

impl<T: PropertyValue> WriteValues for WriteFrame<'_, '_, T> {
    fn apply(&mut self, owner: &FerroObject, operation: WriteOperation) -> bool {
        match operation {
            WriteOperation::Prepare { is_coerced_default_value } => {
                match &self.target.uncommon {
                    Some(uncommon) if !is_coerced_default_value => {
                        // Borrow the original input through the callback. Moving
                        // it into this helper before cloning/coercing would drop
                        // it before the caller's old value on panic.
                        let value = (uncommon.coerce)(owner, self.input.as_ref().expect("write input is present").clone());
                        *self.value = Some(value);
                        *self.uncoerced = Some((uncommon, self.input.take().expect("write input is present")));
                    }
                    _ => *self.value = self.input.take(),
                }
                false
            }
            WriteOperation::ReplaceEffective => {
                let value = self.value.as_ref().expect("prepared write value is present");
                let changed = !value_equals(&*self.target.value.borrow(), value);
                // Keep the complete statement here: replace releases its
                // RefCell borrow and drops the old T before the next step.
                self.target.value.replace(value.clone());
                changed
            }
            WriteOperation::StoreUncoercedEffective => {
                if let Some((uncommon, value)) = &*self.uncoerced {
                    uncommon.uncoerced_value.replace(value.clone());
                }
                false
            }
            WriteOperation::ReplaceBase => {
                let value = self.value.as_ref().expect("prepared write value is present");
                let changed = !self.target.base_value.borrow().as_ref().is_some_and(|base| value_equals(base, value));
                self.target.base_value.replace(Some(self.value.take().expect("prepared write value is present")));
                changed
            }
            WriteOperation::StoreUncoercedBase => {
                if let Some((uncommon, value)) = self.uncoerced.take() {
                    uncommon.uncoerced_base_value.replace(value);
                }
                false
            }
            WriteOperation::NotifyEffective => {
                self.target.notify_value_changed(owner, self.old_value.take().expect("old write value is present"));
                false
            }
            WriteOperation::NotifyBase => {
                self.target.notify_base_value_changed(owner);
                false
            }
        }
    }
}

/// Keep one common branch/priority/notification engine. The typed operation
/// method supplies value semantics without allocating or changing storage.
#[inline(never)]
pub(super) fn write(
    state: &EffectiveValueState,
    owner: &FerroObject,
    priority: BindingPriority,
    is_overridden_current_value: bool,
    is_coerced_default_value: bool,
    values: &mut dyn WriteValues,
) {
    let mut value_changed = false;
    let mut base_value_changed = false;

    state.is_overriden_current_value.set(is_overridden_current_value);
    state.is_coerced_default_value.set(is_coerced_default_value);
    values.apply(owner, WriteOperation::Prepare { is_coerced_default_value });

    // Coercion, equality, Clone, and discarded old values can reenter. Each
    // priority is read at the original point, never planned ahead of time.
    if priority <= state.priority.get() {
        value_changed = values.apply(owner, WriteOperation::ReplaceEffective);
        state.priority.set(priority);
        values.apply(owner, WriteOperation::StoreUncoercedEffective);
    }

    if priority <= state.base_priority.get() && priority >= BindingPriority::LocalValue {
        base_value_changed = values.apply(owner, WriteOperation::ReplaceBase);
        state.base_priority.set(priority);
        values.apply(owner, WriteOperation::StoreUncoercedBase);
    }

    if value_changed {
        values.apply(owner, WriteOperation::NotifyEffective);
    } else if base_value_changed {
        values.apply(owner, WriteOperation::NotifyBase);
    }
}

#[derive(Clone, Copy)]
pub(super) enum NotificationOperation {
    CaptureCurrent,
    CaptureInherited,
    ReleaseInherited,
}

pub(super) trait NotificationValues {
    fn apply(&mut self, operation: NotificationOperation) -> Option<&dyn Any>;
}

pub(super) struct NotificationFrame<'slots, 'target, T> {
    pub(super) target: &'target EffectiveValue<T>,
    pub(super) current: &'slots mut Option<T>,
    pub(super) inherited: &'slots mut Option<T>,
}

impl<T: PropertyValue> NotificationValues for NotificationFrame<'_, '_, T> {
    fn apply(&mut self, operation: NotificationOperation) -> Option<&dyn Any> {
        let slot = match operation {
            NotificationOperation::CaptureCurrent => &mut *self.current,
            NotificationOperation::CaptureInherited => &mut *self.inherited,
            NotificationOperation::ReleaseInherited => {
                // The inherited snapshot's scope ends before notifying(false),
                // including when its destructor itself panics.
                drop(self.inherited.take());
                return None;
            }
        };
        *slot = Some(self.target.value());
        slot.as_ref().map(|value| value as &dyn Any)
    }
}

#[inline(never)]
pub(super) fn notify(
    state: &EffectiveValueState,
    owner: &FerroObject,
    old_value: &dyn Any,
    values: &mut dyn NotificationValues,
) {
    let notifying = state.property.notifying();
    if let Some(notifying) = notifying {
        notifying(owner, true);
    }
    let new_value = values.apply(NotificationOperation::CaptureCurrent).expect("current notification value is present");
    owner.raise_property_changed_values(state.property, Some(old_value), new_value, state.priority.get(), true);
    if state.property.inherits() {
        // Handlers can have changed the value again. Capture it only now and
        // leave the first snapshot in its caller-owned slot until normal exit.
        let new_value = values.apply(NotificationOperation::CaptureInherited).expect("inherited notification value is present");
        owner.values().on_inherited_effective_value_changed(owner, state.property, old_value, new_value);
        values.apply(NotificationOperation::ReleaseInherited);
    }
    if let Some(notifying) = notifying {
        // Deliberately not an unwind guard: the original callback was skipped
        // whenever cloning, an event handler, inheritance, or Drop panicked.
        notifying(owner, false);
    }
}
