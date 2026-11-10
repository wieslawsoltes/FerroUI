//! A copy of a property value kept on the object, for the properties layout and rendering read
//! many times a frame (FerroUI deviation, docs/porting/desktop-performance.md, section 9.9).
//!
//! The copy is valid while the object's value store has the generation it was read at and no
//! property metadata has changed since: every change of an effective value of the store (a value
//! set, coerced, added or removed) advances the store's generation, and every override or removal
//! of metadata advances the metadata generation of the process. A read is then the value of
//! `get_value`, without looking the property up. Only for properties that do not inherit: an
//! inherited value changes with an ancestor's store.

use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

/// The generation of the property metadata. One for the process, not one a thread: a copy is
/// compared with it on every read, and a load of a static costs less than the address of a
/// thread-local value (section 9.28). A change of metadata on one thread makes the copies of
/// every thread read their property once more, which is all it costs.
static METADATA_GENERATION: AtomicU32 = AtomicU32::new(1);

/// The generation of the property metadata.
#[inline]
pub(crate) fn metadata_generation() -> u32 {
    METADATA_GENERATION.load(Ordering::Relaxed)
}

/// Makes every kept copy of a property value read its property again: metadata changed.
pub(crate) fn advance_metadata_generation() {
    // Never zero: the stamp of a copy that was never read is zero.
    let _ = METADATA_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |generation| Some(generation.wrapping_add(1).max(1)));
}

/// A kept copy of a property value: the value and the generations it was read at.
///
/// Public for the controls of the other crates of the framework, which keep the properties their
/// layout reads many times a frame the same way; not part of the documented API.
#[doc(hidden)]
pub struct HotValue<T: Copy> {
    stamp: Cell<u64>,
    value: Cell<T>,
}

impl<T: Copy + Default> Default for HotValue<T> {
    fn default() -> Self {
        Self { stamp: Cell::new(0), value: Cell::new(T::default()) }
    }
}

impl<T: Copy> HotValue<T> {
    /// A copy that holds nothing yet: `value` is only what the cell starts with.
    pub const fn new(value: T) -> Self {
        Self { stamp: Cell::new(0), value: Cell::new(value) }
    }

    /// As [`Self::get`], for a property of `object` that does not inherit: the kept value while
    /// the value store of `object` is unchanged, else `read`'s, kept.
    #[inline]
    pub fn get_for(&self, object: &crate::FerroObject, read: impl FnOnce() -> T) -> T {
        self.get(object.values().value_generation(), read)
    }

    /// The kept value when it is still the value of the property, else `read`'s, kept.
    #[inline]
    pub(crate) fn get(&self, store_generation: u32, read: impl FnOnce() -> T) -> T {
        let stamp = (u64::from(store_generation) << 32) | u64::from(metadata_generation());
        if self.stamp.get() == stamp {
            return self.value.get();
        }
        let value = read();
        self.value.set(value);
        self.stamp.set(stamp);
        value
    }
}
