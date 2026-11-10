//! A copy of a property value kept on the object, for the properties layout and rendering read
//! many times a frame (FerroUI deviation, docs/porting/desktop-performance.md, section 9.9).
//!
//! The copy is valid while the object's value store has the generation it was read at and no
//! property metadata has changed since: every change of an effective value of the store (a value
//! set, coerced, added or removed) advances the store's generation, and every override or removal
//! of metadata advances the metadata generation of the process. A read is then the value of
//! `get_value`, without looking the property up. Only for properties that do not inherit: an
//! inherited value changes with an ancestor's store. (A property that inherits may be kept while
//! no store holds a value of it, when its value is the default of the class: see
//! `Layoutable::use_layout_rounding`.)
//!
//! A class that keeps one value keeps a [`HotValue`]; a class that keeps several keeps them
//! beside one [`HotStamp`], which costs the group the eight bytes of the generations once
//! (section 9.36).

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

/// The generations the kept copies of several property values of one object were read at, and
/// which of them are kept.
///
/// The values live in cells of the class beside the stamp, one bit of the stamp for each: a
/// group of copies costs the generations once, where a [`HotValue`] for each copy costs them
/// for every one. Public for the same reason as [`HotValue`]; not part of the documented API.
#[doc(hidden)]
#[derive(Default)]
#[repr(C)]
pub struct HotStamp {
    stamp: Cell<u64>,
    kept: Cell<u16>,
}

impl HotStamp {
    /// A stamp of a group that keeps nothing yet.
    pub const fn new() -> Self {
        Self { stamp: Cell::new(0), kept: Cell::new(0) }
    }

    /// As [`Self::get`], for a property of `object` that does not inherit.
    #[inline]
    pub fn get_for<T: Copy>(&self, object: &crate::FerroObject, bit: u16, value: &Cell<T>, read: impl FnOnce() -> T) -> T {
        self.get(object.values().value_generation(), bit, value, read)
    }

    /// The value kept in `value`, the cell of the group `bit` stands for (one bit, a different
    /// one for each cell), when it is still the value of the property, else `read`'s, kept.
    #[inline]
    pub(crate) fn get<T: Copy>(&self, store_generation: u32, bit: u16, value: &Cell<T>, read: impl FnOnce() -> T) -> T {
        let stamp = (u64::from(store_generation) << 32) | u64::from(metadata_generation());
        if self.stamp.get() == stamp {
            if self.kept.get() & bit != 0 {
                return value.get();
            }
        } else {
            // Another generation: none of the copies is the value of its property any more.
            self.stamp.set(stamp);
            self.kept.set(0);
        }
        let read_value = read();
        // `read` may have read another copy of the group at a later generation: this one is
        // kept for the generations it was read at only.
        if self.stamp.get() == stamp {
            value.set(read_value);
            self.kept.set(self.kept.get() | bit);
        }
        read_value
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn a_group_keeps_each_value_until_the_generation_changes() {
        let stamp = HotStamp::new();
        let (first, second) = (Cell::new(0), Cell::new(0));
        let reads = Cell::new(0);
        let read = |value: i32| {
            reads.set(reads.get() + 1);
            value
        };

        assert_eq!(1, stamp.get(7, 1, &first, || read(1)));
        assert_eq!(1, stamp.get(7, 1, &first, || read(10)));
        assert_eq!(1, reads.get());

        // The other value of the group is read on its own first request.
        assert_eq!(2, stamp.get(7, 2, &second, || read(2)));
        assert_eq!(2, stamp.get(7, 2, &second, || read(20)));
        assert_eq!(2, reads.get());

        // A change of the store makes both read again.
        assert_eq!(3, stamp.get(8, 1, &first, || read(3)));
        assert_eq!(4, stamp.get(8, 2, &second, || read(4)));
        assert_eq!(4, reads.get());
        assert_eq!(3, stamp.get(8, 1, &first, || read(30)));
        assert_eq!(4, reads.get());
    }

    #[test]
    fn a_change_of_metadata_makes_a_group_read_again() {
        let stamp = HotStamp::new();
        let value = Cell::new(0);
        assert_eq!(1, stamp.get(7, 1, &value, || 1));
        advance_metadata_generation();
        assert_eq!(2, stamp.get(7, 1, &value, || 2));
        assert_eq!(2, stamp.get(7, 1, &value, || 3));
    }

    #[test]
    fn a_value_read_while_another_generation_was_stamped_is_not_kept() {
        let stamp = HotStamp::new();
        let (first, second) = (Cell::new(0), Cell::new(0));
        // The read of the first value reads the second at a later generation.
        let read = stamp.get(7, 1, &first, || {
            assert_eq!(5, stamp.get(8, 2, &second, || 5));
            1
        });
        assert_eq!(1, read);
        // The first value was read at generation 7 and the group is at 8: it is read again.
        assert_eq!(9, stamp.get(8, 1, &first, || 9));
        assert_eq!(5, stamp.get(8, 2, &second, || 50));
    }
}
