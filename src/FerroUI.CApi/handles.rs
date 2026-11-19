//! The handle table of the UI thread: a [`FerroHandle`] is the index of a slot plus one in its
//! low 32 bits and the generation of the slot in its high 32 bits. A slot holds a strong reference
//! to its object and a count of the references the caller holds; when the count reaches zero the
//! slot is freed and its generation moves on, so the old handle no longer matches it. One object
//! has one handle at a time: asking for the handle of an object that has one counts one more
//! reference to that handle.

use crate::error::{guard, status, Error, Result};
use crate::{thread, FerroHandle, FerroStatus, FERRO_BAD_HANDLE, FERRO_NOT_FOUND, FERRO_NOT_SUPPORTED};
use ferroui_base::{FerroObject, Ref};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

struct Slot {
    generation: u32,
    count: u32,
    object: Option<Ref<FerroObject>>,
}

#[derive(Default)]
struct Table {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The handle of each object of the table, by the address of the object.
    by_object: HashMap<usize, FerroHandle, BuildHasherDefault<AddressHasher>>,
}

/// A hasher for addresses: they are distinct and aligned, so a multiplication spreads them well
/// enough, and costs far less than the default hasher.
#[derive(Default)]
struct AddressHasher(u64);

impl Hasher for AddressHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn write_usize(&mut self, value: usize) {
        self.0 = (value as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(29);
    }
}

ferroui_base::ferro_thread_local! {
    static TABLE: RefCell<Table> = RefCell::new(Table::default());
}

/// The address that identifies an object in the table.
#[inline]
pub(crate) fn address(object: &FerroObject) -> usize {
    object as *const FerroObject as usize
}

#[inline]
fn split(handle: FerroHandle) -> Option<(usize, u32)> {
    let index = (handle & 0xFFFF_FFFF) as usize;
    (index != 0).then(|| (index - 1, (handle >> 32) as u32))
}

#[cold]
fn bad_handle(handle: FerroHandle) -> Error {
    if handle == 0 {
        Error::new(FERRO_BAD_HANDLE, "the handle is 0 (no object)")
    } else {
        Error::new(FERRO_BAD_HANDLE, format!("the handle {handle:#x} was released or never issued"))
    }
}

/// The object of `handle`. The caller has checked the thread.
#[inline]
pub(crate) fn get(handle: FerroHandle) -> Result<Ref<FerroObject>> {
    TABLE.with_borrow(|table| {
        let (index, generation) = split(handle)?;
        let slot = table.slots.get(index)?;
        if slot.generation != generation {
            return None;
        }
        slot.object.clone()
    })
    .ok_or_else(|| bad_handle(handle))
}

/// Whether `handle` is the handle of the object at `address` (a cheap check of a handle remembered
/// for an object).
#[inline]
pub(crate) fn is_handle_of(handle: FerroHandle, address: usize) -> bool {
    TABLE.with_borrow(|table| {
        split(handle)
            .and_then(|(index, generation)| table.slots.get(index).filter(|slot| slot.generation == generation))
            .and_then(|slot| slot.object.as_ref())
            .is_some_and(|object| self::address(object) == address)
    })
}

/// The handle the object at `address` has, if it has one; no reference is counted.
pub(crate) fn find(address: usize) -> Option<FerroHandle> {
    TABLE.with_borrow(|table| table.by_object.get(&address).copied())
}

/// A handle for `object` with one more reference counted: its handle if it has one, else a new
/// one.
pub(crate) fn insert(object: Ref<FerroObject>) -> FerroHandle {
    TABLE.with_borrow_mut(|table| {
        let key = address(&object);
        if let Some(&handle) = table.by_object.get(&key) {
            if let Some((index, _)) = split(handle) {
                table.slots[index].count += 1;
            }
            return handle;
        }
        let index = match table.free.pop() {
            Some(index) => index as usize,
            None => {
                table.slots.push(Slot { generation: 1, count: 0, object: None });
                table.slots.len() - 1
            }
        };
        let slot = &mut table.slots[index];
        slot.count = 1;
        slot.object = Some(object);
        let handle = (u64::from(slot.generation) << 32) | (index as u64 + 1);
        table.by_object.insert(key, handle);
        handle
    })
}

/// Counts one more reference to `handle`.
pub(crate) fn retain(handle: FerroHandle) -> Result<()> {
    TABLE.with_borrow_mut(|table| {
        let (index, generation) = split(handle)?;
        let slot = table.slots.get_mut(index).filter(|slot| slot.generation == generation && slot.object.is_some())?;
        slot.count = slot.count.checked_add(1)?;
        Some(())
    })
    .ok_or_else(|| bad_handle(handle))
}

/// Counts one reference less to `handle`; the last one frees the slot. The object is dropped
/// after the table is released (dropping it may run code that uses the table).
pub(crate) fn release(handle: FerroHandle) -> Result<()> {
    let freed = TABLE.with_borrow_mut(|table| {
        let (index, generation) = split(handle)?;
        let slot = table.slots.get_mut(index).filter(|slot| slot.generation == generation && slot.object.is_some())?;
        slot.count -= 1;
        if slot.count > 0 {
            return Some(None);
        }
        let object = slot.object.take();
        slot.generation = slot.generation.wrapping_add(1);
        table.free.push(index as u32);
        if let Some(object) = &object {
            table.by_object.remove(&address(object));
        }
        Some(object)
    });
    match freed {
        Some(object) => {
            drop(object);
            Ok(())
        }
        None => Err(bad_handle(handle)),
    }
}

/// Creates an object of the type `type_id` ([`ferro_type_id`](crate::ferro_type_id)) with its
/// default constructor and returns its handle, with one reference counted. `0` on a failure
/// ([`ferro_last_error_code`](crate::ferro_last_error_code): [`FERRO_NOT_FOUND`] for an unknown
/// ID, [`FERRO_NOT_SUPPORTED`] for a type without a default constructor, such as an abstract
/// class). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_create(type_id: i64) -> FerroHandle {
    guard(0, || {
        let created = (|| -> Result<FerroHandle> {
            thread::check()?;
            let type_ = crate::ids::type_info(type_id)?;
            let object = type_.create_instance().ok_or_else(|| {
                Error::new(FERRO_NOT_SUPPORTED, format!("the type {} has no default constructor", type_.name()))
            })?;
            Ok(insert(object))
        })();
        created.unwrap_or_else(|error: Error| {
            error.record();
            0
        })
    })
}

/// Counts one more reference to `object`, which one more [`ferro_release`] gives back. UI thread
/// only.
#[no_mangle]
pub extern "C" fn ferro_retain(object: FerroHandle) -> FerroStatus {
    status(|| {
        thread::check()?;
        retain(object)
    })
}

/// Gives back one reference to `object`. When the last one is given back the handle is no longer
/// valid; the object itself lives on while the framework holds it (a child in a panel, an open
/// window). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_release(object: FerroHandle) -> FerroStatus {
    status(|| {
        thread::check()?;
        release(object)
    })
}

/// The type ID of the class of `object` (as [`ferro_type_id`](crate::ferro_type_id) gives it), or
/// a negative status. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_type_of(object: FerroHandle) -> i64 {
    crate::error::status_or_value(|| {
        thread::check()?;
        let object = get(object)?;
        crate::ids::intern_type(object.get_type())
            .ok_or_else(|| Error::new(FERRO_NOT_FOUND, "the type of the object has no ID"))
    })
}
