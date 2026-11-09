//! A copy of a property value kept on the object, for the properties layout and rendering read
//! many times a frame (FerroUI deviation, docs/porting/desktop-performance.md, section 9.9).
//!
//! The copy is valid while the object's value store has the generation it was read at and no
//! property metadata has changed since: every change of an effective value of the store (a value
//! set, coerced, added or removed) advances the store's generation, and every override or removal
//! of metadata advances the metadata generation. A read is then the value of `get_value`, without
//! looking the property up. Only for properties that do not inherit: an inherited value changes
//! with an ancestor's store.

use std::cell::Cell;

thread_local! {
    static METADATA_GENERATION: Cell<u32> = const { Cell::new(1) };
}

/// The generation of the property metadata of this thread.
pub(crate) fn metadata_generation() -> u32 {
    METADATA_GENERATION.with(Cell::get)
}

/// Makes every kept copy of a property value read its property again: metadata changed.
pub(crate) fn advance_metadata_generation() {
    METADATA_GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1).max(1)));
}

/// A kept copy of a property value: the value and the generations it was read at.
pub(crate) struct HotValue<T: Copy> {
    stamp: Cell<u64>,
    value: Cell<T>,
}

impl<T: Copy + Default> Default for HotValue<T> {
    fn default() -> Self {
        Self { stamp: Cell::new(0), value: Cell::new(T::default()) }
    }
}

impl<T: Copy> HotValue<T> {
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
