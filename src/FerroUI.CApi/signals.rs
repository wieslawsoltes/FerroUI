//! Signals shared through memory (docs/c-abi.md, section 14): a table of slots that the library
//! allocates once and that both sides read and write directly, so a read or a write of a signal
//! from the language is a load or a store, not a call.
//!
//! - **Slots** hold one value of 8 bytes each (`values`): an `int64_t`, the bits of a `double`,
//!   a boolean as `0` or `1`, a handle. A text slot holds its text in the library (written with
//!   [`ferro_signal_set_str`], read with [`ferro_signal_get_cstr`]) and the length in `values`.
//!   `kinds` says what a slot holds (`0` is a free slot); only the library writes it.
//! - **Language to UI**: the language stores a value into `values[slot]` and then sets bit
//!   `slot % 64` of `dirty[slot / 64]` (an atomic OR with release order, or a plain store on the
//!   UI thread). The library applies the dirty slots on the UI thread at defined points
//!   ([`ferro_signals_apply`], the start of every frame commit, the end of every dispatch): it
//!   takes each dirty word (an atomic swap with acquire order), raises the version of each slot
//!   and writes the value to every property bound to the slot ([`ferro_signal_bind`], or a
//!   `bind` record of a frame). One apply for many writes: the cost of a write is a store.
//! - **UI to language**: a property watched into a slot ([`ferro_signal_watch`]: the text of a
//!   text box, the value of a slider, a selection) is written by the library into the slot when
//!   it changes, on the UI thread: the value (release order), then the version, then bit
//!   `slot % 64` of `changed[slot / 64]`. The language reads the value with a load whenever it
//!   wants, and the version or the changed bits (which it clears itself) tell it what changed.
//! - **Who writes what**: the language writes `values` of the slots it owns (not watched ones)
//!   and `dirty`, and clears `changed`; the library writes `versions`, `kinds`, `changed` and
//!   the `values` of watched slots. Values are written whole (8 aligned bytes), so a reader never
//!   sees half of one.
//!
//! **Safety.** A slot index is checked against the capacity in every call that takes one, and
//! the dirty bits of the last word beyond the capacity are ignored, so no index is out of bounds
//! by construction. Bindings keep a counted reference to their object; a handle is checked when
//! a binding is made, as everywhere in the ABI. Writes go through the setters of the ABI, so a
//! value is converted and validated as [`ferro_set_f64`](crate::ferro_set_f64) does.

use crate::error::{guard, status, status_or_value, Error, Result};
use crate::handles;
use crate::thread::{self, UiCell};
use crate::values;
use crate::{
    ferro_get_bool_or, ferro_get_f64_or, ferro_get_i64_or, ferro_release, ferro_retain, ferro_set_bool, ferro_set_f64,
    ferro_set_handle, ferro_set_i64, ferro_set_str, FerroHandle, FerroStatus, FERRO_INVALID_ARGUMENT,
    FERRO_INVALID_STATE, FERRO_NOT_FOUND, FERRO_TYPE_MISMATCH,
};
use std::ffi::c_char;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::OnceLock;

/// A free slot.
pub const FERRO_SIGNAL_FREE: i32 = 0;
/// A slot of an `int64_t` (an enumeration as its value).
pub const FERRO_SIGNAL_I64: i32 = 1;
/// A slot of a `double`, stored as its bits.
pub const FERRO_SIGNAL_F64: i32 = 2;
/// A slot of a boolean, stored as `0` or `1`.
pub const FERRO_SIGNAL_BOOL: i32 = 3;
/// A slot of a handle (`0` for null).
pub const FERRO_SIGNAL_HANDLE: i32 = 4;
/// A slot of a text: the text is in the library, the slot value is its length in bytes.
pub const FERRO_SIGNAL_TEXT: i32 = 5;

/// The version of the layout of [`FerroSignalTable`].
pub const FERRO_SIGNAL_LAYOUT: u32 = 1;

/// The signal table of the process (docs/c-abi.md, section 14): arrays the library allocates
/// once, which stay at their addresses until the process ends. `capacity` is a multiple of 64;
/// `dirty` and `changed` have `capacity / 64` words.
#[repr(C)]
pub struct FerroSignalTable {
    /// [`FERRO_SIGNAL_LAYOUT`].
    pub layout: u32,
    /// The number of slots.
    pub capacity: u32,
    /// The value of each slot, written whole.
    pub values: *mut u64,
    /// The version of each slot: raised by the library every time the value changes.
    pub versions: *mut u64,
    /// Language to UI: a bit per slot, set by the language after it wrote the value.
    pub dirty: *mut u64,
    /// UI to language: a bit per slot, set by the library after it wrote a watched value; the
    /// language clears it.
    pub changed: *mut u64,
    /// What each slot holds (`FERRO_SIGNAL_*`); written by the library only.
    pub kinds: *const u32,
}

/// The arrays of the table, as atomics (the same memory as the pointers of the table).
struct Shared {
    table: FerroSignalTable,
    values: &'static [AtomicU64],
    versions: &'static [AtomicU64],
    dirty: &'static [AtomicU64],
    changed: &'static [AtomicU64],
    kinds: &'static [AtomicU32],
}

// SAFETY: the arrays are atomics that live for the process; the raw pointers of `table` point at
// them and are only handed out.
unsafe impl Sync for Shared {}
unsafe impl Send for Shared {}

static SHARED: OnceLock<Shared> = OnceLock::new();

fn leak<T>(count: usize, make: impl Fn() -> T) -> &'static mut [T] {
    Box::leak((0..count).map(|_| make()).collect::<Vec<T>>().into_boxed_slice())
}

fn shared() -> Result<&'static Shared> {
    SHARED.get().ok_or_else(|| Error::new(FERRO_INVALID_STATE, "no signal table: call ferro_signal_table first"))
}

/// A binding of a slot to a property, or of a property to a slot.
struct Binding {
    id: i64,
    handle: FerroHandle,
    property: i64,
    /// The token of the property-changed handler of a watch, `None` for a bind.
    watch: Option<u64>,
}

#[derive(Default)]
struct Slot {
    text: String,
    binds: Vec<Binding>,
    watches: Vec<Binding>,
}

struct State {
    slots: Vec<Slot>,
    free: Vec<u32>,
    next_id: i64,
    /// The slot of each binding.
    bindings: Vec<(i64, u32)>,
    /// Whether an apply runs: what it writes is not watched back into the slot it came from.
    applying: Option<u32>,
}

static STATE: UiCell<State> =
    UiCell::new(State { slots: Vec::new(), free: Vec::new(), next_id: 0, bindings: Vec::new(), applying: None });

/// Creates the signal table of the process with room for `capacity` slots (rounded up to a
/// multiple of 64), or returns the one that exists (whose capacity then stays). The arrays stay
/// at their addresses until the process ends. `NULL` on a failure (a capacity that is not
/// positive or above 2^24; off the UI thread). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_table(capacity: i64) -> *const FerroSignalTable {
    guard(std::ptr::null(), || {
        let made = (|| -> Result<*const FerroSignalTable> {
            thread::check()?;
            if let Some(shared) = SHARED.get() {
                return Ok(&shared.table);
            }
            if !(1..=1 << 24).contains(&capacity) {
                return Err(Error::new(FERRO_INVALID_ARGUMENT, "the capacity is not between 1 and 2^24"));
            }
            let capacity = (capacity as usize).div_ceil(64) * 64;
            let values = leak(capacity, || AtomicU64::new(0));
            let versions = leak(capacity, || AtomicU64::new(0));
            let dirty = leak(capacity / 64, || AtomicU64::new(0));
            let changed = leak(capacity / 64, || AtomicU64::new(0));
            let kinds = leak(capacity, || AtomicU32::new(0));
            let table = FerroSignalTable {
                layout: FERRO_SIGNAL_LAYOUT,
                capacity: capacity as u32,
                values: values.as_mut_ptr().cast(),
                versions: versions.as_mut_ptr().cast(),
                dirty: dirty.as_mut_ptr().cast(),
                changed: changed.as_mut_ptr().cast(),
                kinds: kinds.as_ptr().cast(),
            };
            let _ = SHARED.set(Shared { table, values, versions, dirty, changed, kinds });
            STATE.with(|state| {
                state.slots.resize_with(capacity, Slot::default);
                state.free = (0..capacity as u32).rev().collect();
            })?;
            Ok(&shared()?.table)
        })();
        made.unwrap_or_else(|error| {
            error.record();
            std::ptr::null()
        })
    })
}

/// The slot `slot` of the table, checked.
#[inline]
fn slot_of(slot: i64) -> Result<(&'static Shared, usize)> {
    let shared = shared()?;
    match usize::try_from(slot) {
        Ok(index) if index < shared.values.len() => Ok((shared, index)),
        _ => Err(Error::new(FERRO_INVALID_ARGUMENT, format!("{slot} is not a slot of the signal table"))),
    }
}

/// A slot of the kind `kind`, checked.
#[inline]
fn typed_slot(slot: i64, kind: i32) -> Result<(&'static Shared, usize)> {
    let (shared, index) = slot_of(slot)?;
    let found = shared.kinds[index].load(Ordering::Relaxed) as i32;
    if found != kind {
        return Err(Error::new(FERRO_TYPE_MISMATCH, format!("the slot {slot} holds kind {found}, not {kind}")));
    }
    Ok((shared, index))
}

/// Takes a free slot of the kind `kind` (`FERRO_SIGNAL_I64` to `FERRO_SIGNAL_TEXT`), with the
/// value `0` (the empty text), and returns its index, or a negative status
/// ([`FERRO_INVALID_STATE`] when the table is full or not created). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_new(kind: i32) -> i64 {
    status_or_value(|| {
        if !(FERRO_SIGNAL_I64..=FERRO_SIGNAL_TEXT).contains(&kind) {
            return Err(Error::new(FERRO_INVALID_ARGUMENT, format!("{kind} is not a kind of signal")));
        }
        let shared = shared()?;
        let index = STATE
            .with(|state| state.free.pop())?
            .ok_or_else(|| Error::new(FERRO_INVALID_STATE, "the signal table is full"))? as usize;
        shared.values[index].store(0, Ordering::Relaxed);
        shared.kinds[index].store(kind as u32, Ordering::Release);
        Ok(index as i64)
    })
}

/// Frees the slot `slot`: its bindings are removed and its kind is `FERRO_SIGNAL_FREE` until
/// [`ferro_signal_new`] takes it again. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_free(slot: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        let (shared, index) = slot_of(slot)?;
        if shared.kinds[index].load(Ordering::Relaxed) == 0 {
            return Err(Error::new(FERRO_INVALID_STATE, format!("the slot {slot} is free")));
        }
        let ids: Vec<i64> = STATE.with(|state| {
            let slot = &state.slots[index];
            slot.binds.iter().chain(&slot.watches).map(|binding| binding.id).collect()
        })?;
        for id in ids {
            unbind(id)?;
        }
        shared.kinds[index].store(0, Ordering::Release);
        shared.dirty[index / 64].fetch_and(!(1 << (index % 64)), Ordering::AcqRel);
        STATE.with(|state| {
            state.slots[index].text.clear();
            state.free.push(index as u32);
        })?;
        Ok(())
    })
}

/// Stores `value` into a numeric slot and marks it dirty: what a language that cannot write
/// memory calls in place of the store. Any thread (atomics); applied on the UI thread.
#[inline]
fn set_word(slot: i64, kind: i32, value: u64) -> FerroStatus {
    status(|| {
        let (shared, index) = typed_slot(slot, kind)?;
        shared.values[index].store(value, Ordering::Relaxed);
        shared.dirty[index / 64].fetch_or(1 << (index % 64), Ordering::Release);
        Ok(())
    })
}

/// Writes an integer into the slot and marks it dirty (the store and the bit of section 14, for
/// a language that cannot write memory). Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_set_i64(slot: i64, value: i64) -> FerroStatus {
    set_word(slot, FERRO_SIGNAL_I64, value as u64)
}

/// Writes a number into the slot and marks it dirty. Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_set_f64(slot: i64, value: f64) -> FerroStatus {
    set_word(slot, FERRO_SIGNAL_F64, value.to_bits())
}

/// Writes a boolean into the slot and marks it dirty. Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_set_bool(slot: i64, value: bool) -> FerroStatus {
    set_word(slot, FERRO_SIGNAL_BOOL, u64::from(value))
}

/// Writes a handle (`0` for null) into the slot and marks it dirty. Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_set_handle(slot: i64, value: FerroHandle) -> FerroStatus {
    set_word(slot, FERRO_SIGNAL_HANDLE, value)
}

/// Writes UTF-8 text (`length` bytes, negative for NUL-terminated) into a text slot and marks
/// it dirty; the slot value becomes the length. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_signal_set_str(slot: i64, text: *const c_char, length: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        // SAFETY: the caller passes a string as `ferro_set_str` documents.
        let text = unsafe { crate::strings::borrowed(text, length) }?;
        let (shared, index) = typed_slot(slot, FERRO_SIGNAL_TEXT)?;
        STATE.with(|state| {
            let stored = &mut state.slots[index].text;
            stored.clear();
            stored.push_str(text);
        })?;
        shared.values[index].store(text.len() as u64, Ordering::Relaxed);
        shared.dirty[index / 64].fetch_or(1 << (index % 64), Ordering::Release);
        Ok(())
    })
}

/// The word of a slot, `fallback` when the slot is not of the kind (recorded).
fn get_word(slot: i64, kind: i32) -> Option<u64> {
    match typed_slot(slot, kind) {
        Ok((shared, index)) => Some(shared.values[index].load(Ordering::Acquire)),
        Err(error) => {
            error.record();
            None
        }
    }
}

/// The integer of the slot (the load of section 14, for a language that cannot read memory);
/// `0` when the slot is not an integer slot (recorded). Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_get_i64(slot: i64) -> i64 {
    get_word(slot, FERRO_SIGNAL_I64).map_or(0, |word| word as i64)
}

/// The number of the slot; `0` when the slot is not a number slot (recorded). Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_get_f64(slot: i64) -> f64 {
    get_word(slot, FERRO_SIGNAL_F64).map_or(0.0, f64::from_bits)
}

/// The boolean of the slot; `false` when the slot is not a boolean slot (recorded). Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_get_bool(slot: i64) -> bool {
    get_word(slot, FERRO_SIGNAL_BOOL).is_some_and(|word| word != 0)
}

/// The version of the slot (raised every time its value changes); `-1` for a slot that is not
/// in the table (recorded). Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_version(slot: i64) -> i64 {
    match slot_of(slot) {
        Ok((shared, index)) => shared.versions[index].load(Ordering::Acquire) as i64,
        Err(error) => {
            error.record();
            -1
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The text the last `ferro_signal_get_cstr` of the thread returned, with its NUL.
    static LAST_TEXT: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The text of a text slot as NUL-terminated UTF-8 owned by the library until the next call on
/// the thread; the empty text when the slot is not a text slot (recorded), never `NULL`. A
/// language reads it when the version of the slot changed. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_get_cstr(slot: i64) -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    guard(EMPTY.as_ptr().cast(), || {
        let text = typed_slot(slot, FERRO_SIGNAL_TEXT).and_then(|(_, index)| {
            STATE.with(|state| {
                LAST_TEXT.with_borrow_mut(|last| {
                    last.clear();
                    last.extend(state.slots[index].text.bytes().filter(|byte| *byte != 0));
                    last.push(0);
                    last.as_ptr().cast::<c_char>()
                })
            })
        });
        text.unwrap_or_else(|error| {
            error.record();
            EMPTY.as_ptr().cast()
        })
    })
}

/// Writes the value of slot `index` (of kind `kind`) to `property` of `handle`.
fn write(handle: FerroHandle, property: i64, kind: u32, word: u64, text: &str) -> FerroStatus {
    match kind as i32 {
        FERRO_SIGNAL_I64 => ferro_set_i64(handle, property, word as i64),
        FERRO_SIGNAL_F64 => ferro_set_f64(handle, property, f64::from_bits(word)),
        FERRO_SIGNAL_BOOL => ferro_set_bool(handle, property, word != 0),
        FERRO_SIGNAL_HANDLE => ferro_set_handle(handle, property, word),
        // SAFETY: a pointer and the length of a Rust string.
        _ => unsafe { ferro_set_str(handle, property, text.as_ptr().cast(), text.len() as i64) },
    }
}

/// Binds `property` of `object` to the slot: the property takes the value of the slot now and
/// every time an apply finds the slot dirty, with no frame and no call of the language. Returns a
/// positive binding ID for [`ferro_signal_unbind`], or a negative status. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_bind(slot: i64, object: FerroHandle, property: i64) -> i64 {
    status_or_value(|| bind(slot, object, property, false))
}

/// Watches `property` of `object` into the slot: the library writes the value of the property
/// into the slot now and every time it changes (the value, then the version, then the changed
/// bit), so the language reads it with a load and no call. A text property goes into a text slot
/// (read with [`ferro_signal_get_cstr`] when the version changed). Returns a positive binding ID
/// for [`ferro_signal_unbind`], or a negative status. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_watch(slot: i64, object: FerroHandle, property: i64) -> i64 {
    status_or_value(|| bind(slot, object, property, true))
}

pub(crate) fn bind(slot: i64, object: FerroHandle, property: i64, watch: bool) -> Result<i64> {
    thread::check()?;
    let (shared, index) = slot_of(slot)?;
    let kind = shared.kinds[index].load(Ordering::Relaxed);
    if kind == 0 {
        return Err(Error::new(FERRO_INVALID_STATE, format!("the slot {slot} is free")));
    }
    let target = handles::get(object)?;
    // The property must exist on the class (an error now, not at the first apply); a watched one
    // must notify its changes (a registered property).
    let registered = match crate::ids::property(target.get_type(), property)? {
        crate::ids::Property::Registered(registered) => Some(registered),
        crate::ids::Property::Plain(plain) if watch => {
            return Err(Error::new(
                FERRO_TYPE_MISMATCH,
                format!("the property {} does not notify its changes: it cannot be watched", plain.name),
            ))
        }
        crate::ids::Property::Plain(_) => None,
    };
    let id = STATE.with(|state| {
        state.next_id += 1;
        state.bindings.push((state.next_id, index as u32));
        state.next_id
    })?;
    let code = ferro_retain(object);
    if code < 0 {
        return Err(Error::new(code, "the object went away"));
    }
    let mut token = None;
    if let (true, Some(watched)) = (watch, registered) {
        token = Some(target.add_property_changed_handler(std::rc::Rc::new(move |change| {
            if std::ptr::eq(change.property(), watched) {
                watched_changed(index, object, property);
            }
        })));
        watched_changed(index, object, property);
    }
    let binding = Binding { id, handle: object, property, watch: token };
    let text = STATE.with(|state| {
        let slot = &mut state.slots[index];
        if watch {
            slot.watches.push(binding);
            None
        } else {
            slot.binds.push(binding);
            Some(slot.text.clone())
        }
    })?;
    if let Some(text) = text {
        let code = write(object, property, kind, shared.values[index].load(Ordering::Acquire), &text);
        if code < 0 {
            let _ = unbind(id);
            return Err(Error::new(code, "the value of the slot cannot be given to the property"));
        }
    }
    Ok(id)
}

/// A watched property changed: its value goes into the slot (unless it is the value the slot has,
/// or the change is the apply of this very slot).
fn watched_changed(index: usize, object: FerroHandle, property: i64) {
    let Ok(shared) = shared() else {
        return;
    };
    let kind = shared.kinds[index].load(Ordering::Relaxed) as i32;
    if STATE.with(|state| state.applying == Some(index as u32)).unwrap_or(false) {
        return;
    }
    let word = match kind {
        FERRO_SIGNAL_I64 => ferro_get_i64_or(object, property, 0) as u64,
        FERRO_SIGNAL_F64 => ferro_get_f64_or(object, property, 0.0).to_bits(),
        FERRO_SIGNAL_BOOL => u64::from(ferro_get_bool_or(object, property, false)),
        FERRO_SIGNAL_TEXT => {
            let text = values::peek_text(object, property).unwrap_or_default();
            let changed = STATE
                .with(|state| {
                    let stored = &mut state.slots[index].text;
                    let changed = *stored != text;
                    if changed {
                        *stored = text.clone();
                    }
                    changed
                })
                .unwrap_or(false);
            if !changed {
                return;
            }
            text.len() as u64
        }
        _ => return,
    };
    if kind != FERRO_SIGNAL_TEXT && shared.values[index].load(Ordering::Relaxed) == word {
        return;
    }
    shared.values[index].store(word, Ordering::Release);
    shared.versions[index].fetch_add(1, Ordering::Release);
    shared.changed[index / 64].fetch_or(1 << (index % 64), Ordering::Release);
}

/// Removes a binding of [`ferro_signal_bind`] or [`ferro_signal_watch`]. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signal_unbind(binding: i64) -> FerroStatus {
    status(|| unbind(binding))
}

pub(crate) fn unbind(id: i64) -> Result<()> {
    let removed = STATE.with(|state| {
        let position = state.bindings.iter().position(|(binding, _)| *binding == id)?;
        let (_, index) = state.bindings.swap_remove(position);
        let slot = &mut state.slots[index as usize];
        for list in [&mut slot.binds, &mut slot.watches] {
            if let Some(at) = list.iter().position(|binding| binding.id == id) {
                return Some(list.swap_remove(at));
            }
        }
        None
    })?;
    let binding = removed.ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("{id} is not a signal binding")))?;
    if let Some(token) = binding.watch {
        if let Ok(object) = handles::get(binding.handle) {
            object.remove_property_changed_handler(token);
        }
    }
    let _ = ferro_release(binding.handle);
    Ok(())
}

/// Applies the dirty slots now (see [`ferro_signals_apply`]); the number of slots applied.
pub(crate) fn apply() -> Result<i64> {
    let Some(shared) = SHARED.get() else {
        return Ok(0);
    };
    thread::check()?;
    let mut applied = 0;
    let mut failure = None;
    for (word_index, word) in shared.dirty.iter().enumerate() {
        if word.load(Ordering::Relaxed) == 0 {
            continue;
        }
        let mut bits = word.swap(0, Ordering::Acquire);
        while bits != 0 {
            let index = word_index * 64 + bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let kind = shared.kinds[index].load(Ordering::Relaxed);
            if kind == 0 {
                continue;
            }
            applied += 1;
            shared.versions[index].fetch_add(1, Ordering::Release);
            let value = shared.values[index].load(Ordering::Relaxed);
            // The targets, taken with no access of the state running: a write runs handlers.
            let (targets, text) = STATE.with(|state| {
                state.applying = Some(index as u32);
                let slot = &state.slots[index];
                let targets: Vec<(FerroHandle, i64)> =
                    slot.binds.iter().map(|binding| (binding.handle, binding.property)).collect();
                let text = if kind as i32 == FERRO_SIGNAL_TEXT { slot.text.clone() } else { String::new() };
                (targets, text)
            })?;
            for (handle, property) in targets {
                let code = write(handle, property, kind, value, &text);
                if code < 0 && failure.is_none() {
                    failure = Some(code);
                }
            }
            STATE.with(|state| state.applying = None)?;
        }
    }
    match failure {
        Some(code) => Err(Error::new(code, crate::error::last_message())),
        None => Ok(applied),
    }
}

/// Applies the dirty slots: for each slot whose dirty bit is set, its version is raised and its
/// value written to every property bound to it. Returns the number of slots applied, or a
/// negative status (the first failed write; the others are still made). The library applies by
/// itself at the start of every frame commit and at the end of every dispatch; a language calls
/// this after writing slots outside them (a timer of its own). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_signals_apply() -> i64 {
    status_or_value(apply)
}

/// An apply at a defined point of the library (a commit, the end of a dispatch): its failures
/// are recorded, not returned.
#[inline]
pub(crate) fn apply_pending() {
    if let Some(shared) = SHARED.get() {
        if shared.dirty.iter().any(|word| word.load(Ordering::Relaxed) != 0) {
            if let Err(error) = apply() {
                error.record();
            }
        }
    }
}

/// Clears `changed` bits of the slot for a language that cannot write memory, and returns
/// whether it was set. Any thread.
#[no_mangle]
pub extern "C" fn ferro_signal_take_changed(slot: i64) -> bool {
    match slot_of(slot) {
        Ok((shared, index)) => {
            let bit = 1 << (index % 64);
            shared.changed[index / 64].fetch_and(!bit, Ordering::AcqRel) & bit != 0
        }
        Err(error) => {
            error.record();
            false
        }
    }
}
