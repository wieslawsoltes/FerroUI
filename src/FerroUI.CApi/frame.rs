//! Frames: an interface described again on every update, as a list of records, and reconciled
//! into the retained tree of a root (docs/bindings-design.md, section 4).
//!
//! A frame is a byte buffer of records of 8-byte words (little-endian): open an element of a
//! class with a key, set its properties from scalars, bind its events to integer message tags,
//! close it. A language that writes memory fills such a buffer and commits it with one call
//! ([`ferro_frame_commit_bytes`]); a language that cannot calls the scalar writer
//! ([`ferro_frame_open`], [`ferro_frame_str`], ...), which writes the same records into a buffer
//! of the calling thread that [`ferro_frame_commit`] commits. The layout is part of the API model
//! (`bindings/model/ferroui-api.json`, `frame`).
//!
//! A commit compares the frame with the one before it under the same root and changes only what
//! differs: an element keeps its object while its class and its key (or, without a key, its
//! position among the siblings without one) stay, a property is written when its value changed
//! (a text not even then when the object already shows it: what the user typed into a text box
//! is not written back), cleared when the frame no longer sets it, and events are bound once
//! and given their new tags in place. Elements that went away are taken out of their parent and
//! their handles released. The writes go through the setters of the ABI (`ferro_set_str`, ...),
//! so a frame converts and checks values exactly as they do.
//!
//! **What a commit costs** (docs/c-abi.md, section 13). The frame is indexed in one pass over its
//! record headers into a flat list of elements (no allocation per element: the list and its
//! buffers are kept from one commit to the next), and each element of the retained tree keeps
//! the bytes of its attribute records from the frame before. An element whose attribute bytes
//! are the same is not looked at further: one comparison of a few bytes per unchanged element,
//! with no decoding, no lookup and no write. Siblings whose keys and classes are the ones of the
//! frame before, in the same order, are matched by position, with no table.
//!
//! **Memo elements** make the cost of a frame the cost of what changed. An element with a memo
//! record ([`ferro_frame_memo`]: a version and a tag base) stands for a subtree that the version
//! fixes: while its key and version stay, its subtree is not compared at all. A binding that
//! knows the version of a subtree asks [`ferro_frame_reuse`] before writing it: when the last
//! frame under the root has it, one reuse record replaces the whole subtree in the frame, and
//! the binding does not even build it. The library keeps the bytes of each memo element of a
//! root, so a reuse record also works for an element that moved to another parent.
//!
//! An event bound in a frame calls the one dispatcher of the thread ([`ferro_frame_dispatcher`])
//! with its tag, its sender and the value of the property the binding names (the text of a text
//! box for `TextChanged`): a language without closures dispatches on the tag. What a commit
//! causes is not dispatched: events raised while it writes values, and events whose value is
//! still the one the last frame set (the `TextChanged` a text box posts after the commit wrote
//! its text), so two text boxes that follow each other do not echo.
//!
//! This is a keyed reconcile of its own, not the reconciler of `ferroui-view`: that one compares
//! typed Rust elements (`ElementClass`, `&'static FerroProperty`), while a frame names classes,
//! properties and events by their IDs and writes them through the setters of the ABI.

use crate::error::{guard, status, Error, Result};
use crate::thread::UiCell;
use crate::{
    ferro_add_child, ferro_clear, ferro_create, ferro_release, ferro_remove_child, ferro_retain, ferro_set_bool,
    ferro_set_f64, ferro_set_i64, ferro_set_str, ferro_subscribe_data, ferro_unsubscribe, handles, thread, values,
    FerroEventFn, FerroHandle, FerroStatus, FERRO_INVALID_ARGUMENT, FERRO_INVALID_STATE, FERRO_NOT_FOUND, FERRO_OK,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_void};
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicPtr, Ordering};

/// The record that opens an element: `type`, `key`.
pub(crate) const OP_OPEN: u32 = 1;
/// The record that closes the element opened last.
pub(crate) const OP_CLOSE: u32 = 2;
/// A property from UTF-8 text: `property`, `length`, the bytes.
pub(crate) const OP_TEXT: u32 = 3;
/// A property from a number: `property`, the bits of an `f64`.
pub(crate) const OP_NUMBER: u32 = 4;
/// A property from an integer: `property`, `value`.
pub(crate) const OP_INTEGER: u32 = 5;
/// A property from a boolean: `property`, `0` or `1`.
pub(crate) const OP_BOOL: u32 = 6;
/// An event bound to a message tag: `event`, `tag`, `value_property`.
pub(crate) const OP_ON: u32 = 7;
/// A property from UTF-16 text: `property`, the number of code units, the units.
pub(crate) const OP_TEXT16: u32 = 8;
/// A text key for the open element: `length`, the bytes.
pub(crate) const OP_KEY_TEXT: u32 = 9;
/// The open element is a memo element: `version`, `tag base`.
pub(crate) const OP_MEMO: u32 = 10;
/// The memo element of the last frame with this key and version, in place of an element: `key`,
/// `version`, `tag base`.
pub(crate) const OP_REUSE: u32 = 11;
/// A property of the open element bound to a signal slot: `property`, `slot`.
pub(crate) const OP_BIND: u32 = 12;

// ---- the writer -----------------------------------------------------------------------------

ferroui_base::ferro_thread_local! {
    /// The frame the scalar writer of a thread other than the UI thread writes, as the 8-byte
    /// words of its records (little-endian in memory on every host): a record is appended word
    /// by word, with no copy of a variable length but its text.
    static WRITER: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// The frame the scalar writer of the UI thread writes: a static, so that a record costs no
/// thread-local lookup on the thread that commits frames.
static UI_WRITER: UiCell<Vec<u64>> = UiCell::new(Vec::new());

/// Runs `write` with the frame of the calling thread.
#[inline]
fn with_writer<R>(write: impl FnOnce(&mut Vec<u64>) -> R + Copy) -> R {
    if thread::on_ui_thread() {
        if let Ok(result) = UI_WRITER.with(write) {
            return result;
        }
    }
    WRITER.with_borrow_mut(write)
}

/// The bytes of the words of a frame.
fn word_bytes(words: &[u64]) -> &[u8] {
    // SAFETY: the memory of `words` is `8 * words.len()` initialised bytes, and bytes have no
    // alignment.
    unsafe { std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), words.len() * 8) }
}

/// The first word of a record: the operation and the size in bytes.
#[inline]
fn header(op: u32, size: usize) -> u64 {
    (u64::from(op) | ((size as u64) << 32)).to_le()
}

/// Appends a record: the header, the words, then `tail` padded with zeros to 8 bytes.
#[cfg(test)]
pub(crate) fn write_record(buffer: &mut Vec<u8>, op: u32, words: &[i64], tail: &[u8]) {
    let padding = tail.len().wrapping_neg() & 7;
    let size = 8 + 8 * words.len() + tail.len() + padding;
    buffer.reserve(size);
    buffer.extend_from_slice(&op.to_le_bytes());
    buffer.extend_from_slice(&(size as u32).to_le_bytes());
    for word in words {
        buffer.extend_from_slice(&word.to_le_bytes());
    }
    buffer.extend_from_slice(tail);
    buffer.extend_from_slice(&[0; 8][..padding]);
}

/// The bytes the scalar writer of the calling thread has written since the frame began.
#[cfg(test)]
pub(crate) fn writer_bytes() -> Vec<u8> {
    with_writer(|words| word_bytes(words).to_vec())
}

/// Appends `tail` padded with zeros to whole words.
#[inline]
fn push_tail(buffer: &mut Vec<u64>, tail: &[u8]) {
    let words = tail.len().div_ceil(8);
    buffer.reserve(words);
    let length = buffer.len();
    // SAFETY: `words` words are reserved after the length; the tail and its zeros fill them all.
    unsafe {
        let end = buffer.as_mut_ptr().add(length).cast::<u8>();
        std::ptr::copy_nonoverlapping(tail.as_ptr(), end, tail.len());
        std::ptr::write_bytes(end.add(tail.len()), 0, words * 8 - tail.len());
        buffer.set_len(length + words);
    }
}

/// A record of `N` words and no tail.
#[inline]
fn write_words<const N: usize>(op: u32, words: [i64; N]) -> FerroStatus {
    guard(crate::FERRO_PANIC, || {
        with_writer(|buffer| {
            buffer.reserve(1 + N);
            buffer.push(header(op, 8 + 8 * N));
            for word in words {
                buffer.push((word as u64).to_le());
            }
        });
        FERRO_OK
    })
}

/// A record of `N` words and a tail padded to 8 bytes.
#[inline]
fn write_tail<const N: usize>(op: u32, words: [i64; N], tail: &[u8]) -> FerroStatus {
    if tail.len() > u32::MAX as usize / 2 {
        return Error::new(FERRO_INVALID_ARGUMENT, "the text is too long for a record").record();
    }
    guard(crate::FERRO_PANIC, || {
        let size = 8 + 8 * N + tail.len().div_ceil(8) * 8;
        with_writer(|buffer| {
            buffer.reserve(size / 8);
            buffer.push(header(op, size));
            for word in words {
                buffer.push((word as u64).to_le());
            }
            push_tail(buffer, tail);
        });
        FERRO_OK
    })
}

/// The bytes of a string argument; a negative length means NUL-terminated. Not checked for
/// UTF-8 here: a commit checks the text of a value it writes (an unchanged value was checked when
/// it was written), and the key of every element.
unsafe fn text_bytes<'a>(text: *const c_char, length: i64) -> Result<&'a [u8]> {
    if text.is_null() {
        return if length == 0 { Ok(&[]) } else { Err(Error::new(crate::FERRO_NULL_POINTER, "the string is NULL")) };
    }
    if length < 0 {
        // SAFETY: the caller passes a NUL-terminated string for a negative length.
        return Ok(unsafe { std::ffi::CStr::from_ptr(text) }.to_bytes());
    }
    let length = usize::try_from(length).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "the string is too long"))?;
    // SAFETY: the caller passes `length` readable bytes.
    Ok(unsafe { std::slice::from_raw_parts(text.cast::<u8>(), length) })
}

/// Starts a frame on the calling thread: empties the buffer of the scalar writer. Any thread (the
/// buffer is the calling thread's); commit it on the UI thread.
#[no_mangle]
pub extern "C" fn ferro_frame_begin() -> FerroStatus {
    guard(crate::FERRO_PANIC, || {
        with_writer(Vec::clear);
        FERRO_OK
    })
}

/// Opens an element of the class `type_id` ([`ferro_type_id`](crate::ferro_type_id)) in the frame
/// of the calling thread. `key` tells siblings apart across frames: `0` keeps the element by its
/// position among the siblings without a key. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_open(type_id: i64, key: i64) -> FerroStatus {
    write_words(OP_OPEN, [type_id, key])
}

/// Gives the element opened last a text key in place of its integer key. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_key_str(text: *const c_char, length: i64) -> FerroStatus {
    // SAFETY: the caller passes a string as `ferro_set_str` documents.
    match unsafe { text_bytes(text, length) } {
        Ok(bytes) => write_tail(OP_KEY_TEXT, [bytes.len() as i64], bytes),
        Err(error) => error.record(),
    }
}

/// Closes the element opened last in the frame of the calling thread. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_close() -> FerroStatus {
    write_words(OP_CLOSE, [])
}

/// Sets `property` of the open element from text, as [`ferro_set_str`] does when the frame is
/// committed. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_str(property: i64, text: *const c_char, length: i64) -> FerroStatus {
    // SAFETY: the caller passes a string as `ferro_set_str` documents.
    match unsafe { text_bytes(text, length) } {
        Ok(bytes) => write_tail(OP_TEXT, [property, bytes.len() as i64], bytes),
        Err(error) => error.record(),
    }
}

/// Sets `property` of the open element from UTF-16 text (`units` code units), for a language
/// whose strings are UTF-16. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_str16(property: i64, text: *const u16, units: i64) -> FerroStatus {
    let units = match usize::try_from(units) {
        Ok(units) if !text.is_null() || units == 0 => units,
        _ => return Error::new(FERRO_INVALID_ARGUMENT, "the text is NULL or its length is negative").record(),
    };
    let slice: &[u16] = if units == 0 {
        &[]
    } else {
        // SAFETY: the caller passes `units` readable code units.
        unsafe { std::slice::from_raw_parts(text, units) }
    };
    let bytes: Vec<u8> = slice.iter().flat_map(|unit| unit.to_le_bytes()).collect();
    write_tail(OP_TEXT16, [property, units as i64], &bytes)
}

/// Sets `property` of the open element from a number, as [`ferro_set_f64`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_f64(property: i64, value: f64) -> FerroStatus {
    write_words(OP_NUMBER, [property, value.to_bits() as i64])
}

/// Sets `property` of the open element from an integer or the value of an enumeration member, as
/// [`ferro_set_i64`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_i64(property: i64, value: i64) -> FerroStatus {
    write_words(OP_INTEGER, [property, value])
}

/// Sets `property` of the open element from a boolean, as [`ferro_set_bool`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_bool(property: i64, value: bool) -> FerroStatus {
    write_words(OP_BOOL, [property, i64::from(value)])
}

/// Binds the event `event` ([`ferro_event_id`](crate::ferro_event_id)) of the open element to the
/// message tag `tag`: when it fires, the dispatcher of the thread ([`ferro_frame_dispatcher`]) is
/// called with the tag, the sender, and the value of `value_property` (`0` for none), whose text
/// and number [`ferro_frame_event_text`] and [`ferro_frame_event_f64`] give. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_on(event: i64, tag: i64, value_property: i64) -> FerroStatus {
    write_words(OP_ON, [event, tag, value_property])
}

/// Binds `property` of the open element to the signal slot `slot` (docs/c-abi.md, section 14):
/// the commit binds it ([`ferro_signal_bind`](crate::ferro_signal_bind)) and keeps the binding
/// while the frames keep the record, so a write of the slot updates the property with no new
/// frame. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_bind(property: i64, slot: i64) -> FerroStatus {
    write_words(OP_BIND, [property, slot])
}

/// Makes the open element a memo element: its subtree is what `version` says it is (a revision
/// of the data it shows, a hash of it), and the tags of its events are `tag_base` plus offsets
/// that the version fixes. While the element keeps its key and its version, a commit does not
/// look into its subtree (a new `tag_base` moves its tags), and with an integer key the next
/// frame can reuse it whole ([`ferro_frame_reuse`]). A different subtree under the same key and
/// version is the caller's error: the commit keeps the old one. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_memo(version: i64, tag_base: i64) -> FerroStatus {
    write_words(OP_MEMO, [version, tag_base])
}

/// Writes the memo element with the integer key `key` and the version `version` of the last
/// frame committed under `root` ([`ferro_frame_memo`]) into the frame of the calling thread as one
/// record, in place of its whole subtree, with its tags moved to start at `tag_base`. Returns the
/// number of tags the subtree uses (`0` or more: the next tag of the caller is `tag_base` plus
/// it), or a negative status, and then writes nothing: [`FERRO_NOT_FOUND`] when the root has no
/// such element (nothing is recorded; the caller writes the element in full), or
/// [`FERRO_WRONG_THREAD`](crate::FERRO_WRONG_THREAD) off the UI thread. Commit the frame under
/// the same root, before another commit under it. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_reuse(root: FerroHandle, key: i64, version: i64, tag_base: i64) -> i64 {
    guard(i64::from(crate::FERRO_PANIC), || {
        if let Err(error) = thread::check() {
            return i64::from(error.record());
        }
        let tags = RETAINED
            .with(|retained| {
                retained
                    .iter()
                    .find(|(handle, _)| *handle == root)
                    .and_then(|(_, state)| state.memo.get(&key))
                    .filter(|stored| stored.version == version)
                    .map(|stored| stored.tags)
            })
            .ok()
            .flatten();
        match tags {
            Some(tags) => {
                let status = write_words(OP_REUSE, [key, version, tag_base]);
                if status == FERRO_OK {
                    tags
                } else {
                    i64::from(status)
                }
            }
            None => i64::from(FERRO_NOT_FOUND),
        }
    })
}

/// Commits the frame of the scalar writer of the calling thread under `root` (see
/// [`ferro_frame_commit_bytes`]) and empties the buffer. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_commit(root: FerroHandle) -> FerroStatus {
    status(|| {
        thread::check()?;
        let words = UI_WRITER.with(std::mem::take)?;
        let result = commit(root, word_bytes(&words));
        // The buffer keeps its capacity for the next frame.
        UI_WRITER.with(|buffer| {
            if buffer.is_empty() {
                *buffer = words;
                buffer.clear();
            }
        })?;
        result
    })
}

/// Commits the frame of `length` bytes at `data` (records of the layout of the API model) under
/// `root`: the elements at the top of the frame become the children of `root` (the content of a
/// window, the children of a panel), and the frame before it under the same root is reconciled
/// into this one. With a `root` of `0` the elements at the top are windows: shown
/// ([`ferro_show`](crate::ferro_show)) when they come and closed when they go, which needs the
/// application to run ([`ferro_app_run`](crate::ferro_app_run)). The library keeps a reference
/// to `root` until [`ferro_frame_clear`]. A failed write does not stop the commit: the first
/// failure is returned and recorded. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_commit_bytes(root: FerroHandle, data: *const u8, length: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        let length =
            usize::try_from(length).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "the length is negative"))?;
        let bytes: &[u8] = if length == 0 {
            &[]
        } else if data.is_null() {
            return Err(Error::new(crate::FERRO_NULL_POINTER, "the frame is NULL"));
        } else {
            // SAFETY: the caller passes `length` readable bytes.
            unsafe { std::slice::from_raw_parts(data, length) }
        };
        commit(root, bytes)
    })
}

// ---- the index of a frame -------------------------------------------------------------------

/// No entry.
const NONE: u32 = u32::MAX;

/// The key of an element of a frame: a text key is a range of the frame.
#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyRef {
    None,
    Int(i64),
    Text(u32, u32),
}

/// An element of a frame, or a reuse record in place of one.
#[derive(Clone, Copy, Debug)]
struct Entry {
    reuse: bool,
    type_id: i64,
    key: KeyRef,
    /// The bytes of the element and its subtree: from its open record to the end of its close
    /// record (the reuse record itself for a reuse).
    start: u32,
    end: u32,
    /// The attribute records (values and events), a range of the frame or, for an element whose
    /// attribute records are not next to each other, of the scratch buffer of the index.
    attrs: (u32, u32),
    scattered: bool,
    /// The version and the tag base of a memo element or of a reuse.
    memo: Option<(i64, i64)>,
    first_child: u32,
    last_child: u32,
    next: u32,
}

/// The elements of a frame in one flat list, kept from one commit to the next.
#[derive(Default)]
struct Index {
    entries: Vec<Entry>,
    scratch: Vec<u8>,
    stack: Vec<u32>,
    first: u32,
    last: u32,
}

#[cold]
fn malformed(what: &str, offset: usize) -> Error {
    Error::new(FERRO_INVALID_ARGUMENT, format!("the frame is malformed at byte {offset}: {what}"))
}

#[inline]
fn word_at(bytes: &[u8], offset: usize) -> i64 {
    let mut word = [0; 8];
    word.copy_from_slice(&bytes[offset..offset + 8]);
    i64::from_le_bytes(word)
}

#[inline]
fn header_at(bytes: &[u8], offset: usize) -> (u32, usize) {
    let header = word_at(bytes, offset) as u64;
    (header as u32, (header >> 32) as usize)
}

impl Index {
    const fn new() -> Self {
        Index { entries: Vec::new(), scratch: Vec::new(), stack: Vec::new(), first: NONE, last: NONE }
    }

    /// Indexes `bytes`, checking the layout of every record and the text of every key. The text of
    /// a value is checked when a commit writes it: an unchanged value is not looked at.
    fn build(&mut self, bytes: &[u8]) -> Result<()> {
        self.entries.clear();
        self.scratch.clear();
        self.stack.clear();
        self.first = NONE;
        self.last = NONE;
        if bytes.len() >= NONE as usize {
            return Err(Error::new(FERRO_INVALID_ARGUMENT, "the frame is larger than 4 GB"));
        }
        let mut offset = 0;
        while offset < bytes.len() {
            if bytes.len() - offset < 8 {
                return Err(malformed("a record header is cut short", offset));
            }
            let (op, size) = header_at(bytes, offset);
            if size < 8 || size % 8 != 0 || size > bytes.len() - offset {
                return Err(malformed("the size of a record is not a multiple of 8 inside the frame", offset));
            }
            let words = (size - 8) / 8;
            let body = offset + 8;
            let need = |count: usize| -> Result<()> {
                if words < count {
                    Err(malformed("a record is shorter than its fields", offset))
                } else {
                    Ok(())
                }
            };
            // The bytes of the text of a record whose first `count` words are fields.
            let tail = |count: usize, length: i64| -> Result<&[u8]> {
                let length = usize::try_from(length).map_err(|_| malformed("a negative length", offset))?;
                let start = body + count * 8;
                if length > size - 8 - count * 8 {
                    return Err(malformed("a text is longer than its record", offset));
                }
                Ok(&bytes[start..start + length])
            };
            match op {
                OP_OPEN => {
                    need(2)?;
                    let key = match word_at(bytes, body + 8) {
                        0 => KeyRef::None,
                        key => KeyRef::Int(key),
                    };
                    let index = self.push(false, word_at(bytes, body), key, offset, None);
                    self.stack.push(index);
                }
                OP_CLOSE => {
                    let Some(index) = self.stack.pop() else {
                        let message = format!("the frame closes an element it did not open (byte {offset})");
                        return Err(Error::new(FERRO_INVALID_STATE, message));
                    };
                    self.entries[index as usize].end = (offset + size) as u32;
                }
                OP_REUSE => {
                    need(3)?;
                    let key = word_at(bytes, body);
                    if key == 0 {
                        return Err(malformed("a reuse record without a key", offset));
                    }
                    let memo = Some((word_at(bytes, body + 8), word_at(bytes, body + 16)));
                    let index = self.push(true, 0, KeyRef::Int(key), offset, memo);
                    self.entries[index as usize].end = (offset + size) as u32;
                }
                OP_KEY_TEXT => {
                    need(1)?;
                    let open = self.open(offset)?;
                    let key = tail(1, word_at(bytes, body))?;
                    if std::str::from_utf8(key).is_err() {
                        return Err(Error::new(crate::FERRO_INVALID_UTF8, "a key of the frame is not UTF-8"));
                    }
                    self.entries[open as usize].key = KeyRef::Text((body + 8) as u32, key.len() as u32);
                }
                OP_MEMO => {
                    need(2)?;
                    let open = self.open(offset)?;
                    self.entries[open as usize].memo = Some((word_at(bytes, body), word_at(bytes, body + 8)));
                }
                OP_TEXT | OP_TEXT16 => {
                    need(2)?;
                    let open = self.open(offset)?;
                    let count = word_at(bytes, body + 8);
                    let length = if op == OP_TEXT { Some(count) } else { count.checked_mul(2) };
                    tail(2, length.ok_or_else(|| malformed("a length too large", offset))?)?;
                    self.attribute(bytes, open, offset, size);
                }
                OP_NUMBER | OP_INTEGER | OP_BOOL => {
                    need(2)?;
                    let open = self.open(offset)?;
                    self.attribute(bytes, open, offset, size);
                }
                OP_BIND => {
                    need(2)?;
                    let open = self.open(offset)?;
                    self.attribute(bytes, open, offset, size);
                }
                OP_ON => {
                    need(3)?;
                    let open = self.open(offset)?;
                    self.attribute(bytes, open, offset, size);
                }
                _ => return Err(malformed("an unknown record", offset)),
            }
            offset += size;
        }
        if !self.stack.is_empty() {
            return Err(Error::new(
                FERRO_INVALID_STATE,
                format!("the frame leaves {} elements open", self.stack.len()),
            ));
        }
        Ok(())
    }

    /// The element a value or an event belongs to: the one opened last.
    fn open(&self, offset: usize) -> Result<u32> {
        self.stack.last().copied().ok_or_else(|| malformed("a value or an event outside an element", offset))
    }

    /// A new entry, linked as the last child of the open element (or at the top).
    fn push(&mut self, reuse: bool, type_id: i64, key: KeyRef, offset: usize, memo: Option<(i64, i64)>) -> u32 {
        let index = self.entries.len() as u32;
        self.entries.push(Entry {
            reuse,
            type_id,
            key,
            start: offset as u32,
            end: offset as u32,
            attrs: (0, 0),
            scattered: false,
            memo,
            first_child: NONE,
            last_child: NONE,
            next: NONE,
        });
        let last = match self.stack.last() {
            Some(&parent) => {
                let parent = &mut self.entries[parent as usize];
                let last = parent.last_child;
                parent.last_child = index;
                if last == NONE {
                    parent.first_child = index;
                }
                last
            }
            None => {
                let last = self.last;
                self.last = index;
                if last == NONE {
                    self.first = index;
                }
                last
            }
        };
        if last != NONE {
            self.entries[last as usize].next = index;
        }
        index
    }

    /// Adds the record at `offset` to the attribute records of the entry `open`.
    fn attribute(&mut self, bytes: &[u8], open: u32, offset: usize, size: usize) {
        let entry = &mut self.entries[open as usize];
        let (start, end) = (entry.attrs.0 as usize, entry.attrs.1 as usize);
        if !entry.scattered && start == end {
            entry.attrs = (offset as u32, (offset + size) as u32);
        } else if !entry.scattered && end == offset {
            entry.attrs.1 = (offset + size) as u32;
        } else {
            // Attribute records after a child: the records of the element are gathered at the end of
            // the scratch buffer.
            if !entry.scattered {
                let at = self.scratch.len();
                self.scratch.extend_from_slice(&bytes[start..end]);
                entry.attrs = (at as u32, self.scratch.len() as u32);
                entry.scattered = true;
            } else if end != self.scratch.len() {
                let at = self.scratch.len();
                self.scratch.extend_from_within(start..end);
                entry.attrs = (at as u32, self.scratch.len() as u32);
            }
            self.scratch.extend_from_slice(&bytes[offset..offset + size]);
            entry.attrs.1 = self.scratch.len() as u32;
        }
    }
}

/// A frame and its index.
struct Frame<'a> {
    bytes: &'a [u8],
    index: &'a Index,
}

impl<'a> Frame<'a> {
    fn entry(&self, index: u32) -> &'a Entry {
        &self.index.entries[index as usize]
    }

    fn attrs(&self, entry: &Entry) -> &'a [u8] {
        let (start, end) = (entry.attrs.0 as usize, entry.attrs.1 as usize);
        if entry.scattered {
            &self.index.scratch[start..end]
        } else {
            &self.bytes[start..end]
        }
    }

    fn subtree(&self, entry: &Entry) -> &'a [u8] {
        &self.bytes[entry.start as usize..entry.end as usize]
    }

    fn key_text(&self, start: u32, length: u32) -> &'a str {
        // Checked when the frame was indexed.
        std::str::from_utf8(&self.bytes[start as usize..(start + length) as usize]).unwrap_or_default()
    }

    /// Whether the key of `entry` is `key`.
    fn same_key(&self, key: &Key, entry: &Entry) -> bool {
        match (key, entry.key) {
            (Key::None, KeyRef::None) => true,
            (Key::Int(a), KeyRef::Int(b)) => *a == b,
            (Key::Text(a), KeyRef::Text(start, length)) => **a == *self.key_text(start, length),
            _ => false,
        }
    }

    fn owned_key(&self, entry: &Entry) -> Key {
        match entry.key {
            KeyRef::None => Key::None,
            KeyRef::Int(key) => Key::Int(key),
            KeyRef::Text(start, length) => Key::Text(self.key_text(start, length).into()),
        }
    }
}

// ---- attribute records ----------------------------------------------------------------------

/// A value of a property in a frame, borrowed from its record.
#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueRef<'a> {
    /// UTF-8, checked when the frame was indexed.
    Text(&'a [u8]),
    /// UTF-16 code units, little-endian, checked when the frame was indexed.
    Text16(&'a [u8]),
    /// The bits of the number, so that the same number compares equal (NaN included).
    Number(u64),
    Integer(i64),
    Bool(bool),
}

impl ValueRef<'_> {
    /// The text of a text value: `None` for another value, and for text that is not UTF-8
    /// (UTF-16 for `Text16`).
    fn text(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            ValueRef::Text(bytes) => std::str::from_utf8(bytes).ok().map(std::borrow::Cow::Borrowed),
            ValueRef::Text16(bytes) => {
                let units: Vec<u16> =
                    bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
                String::from_utf16(&units).ok().map(std::borrow::Cow::Owned)
            }
            _ => None,
        }
    }

    fn owned(&self) -> Value {
        match self {
            ValueRef::Text(_) | ValueRef::Text16(_) => {
                Value::Text(self.text().map(std::borrow::Cow::into_owned).unwrap_or_default())
            }
            ValueRef::Number(bits) => Value::Number(*bits),
            ValueRef::Integer(value) => Value::Integer(*value),
            ValueRef::Bool(value) => Value::Bool(*value),
        }
    }
}

/// An attribute record.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Attr<'a> {
    Prop(i64, ValueRef<'a>),
    /// Event, tag, value property.
    On(i64, i64, i64),
    /// Property, signal slot.
    Bind(i64, i64),
}

/// The attribute records of `bytes` (records checked when the frame was indexed), with their
/// offsets.
fn records(bytes: &[u8]) -> impl Iterator<Item = (usize, Attr<'_>)> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        while offset + 8 <= bytes.len() {
            let at = offset;
            let (op, size) = header_at(bytes, at);
            if size < 8 || at + size > bytes.len() {
                return None;
            }
            offset += size;
            let word = |index: usize| word_at(bytes, at + 8 + index * 8);
            let attr = match op {
                OP_TEXT => {
                    let length = word(1) as usize;
                    Attr::Prop(word(0), ValueRef::Text(&bytes[at + 24..at + 24 + length]))
                }
                OP_TEXT16 => {
                    let length = word(1) as usize * 2;
                    Attr::Prop(word(0), ValueRef::Text16(&bytes[at + 24..at + 24 + length]))
                }
                OP_NUMBER => Attr::Prop(word(0), ValueRef::Number(word(1) as u64)),
                OP_INTEGER => Attr::Prop(word(0), ValueRef::Integer(word(1))),
                OP_BOOL => Attr::Prop(word(0), ValueRef::Bool(word(1) != 0)),
                OP_ON => Attr::On(word(0), word(1), word(2)),
                OP_BIND => Attr::Bind(word(0), word(1)),
                _ => continue,
            };
            return Some((at, attr));
        }
        None
    })
}

/// The value the attribute records `bytes` give `property`: the last one for it.
fn last_value(bytes: &[u8], property: i64) -> Option<ValueRef<'_>> {
    records(bytes)
        .filter_map(|(_, attr)| match attr {
            Attr::Prop(id, value) if id == property => Some(value),
            _ => None,
        })
        .last()
}

/// Whether the attribute record at `at` is the last one of `bytes` for its property or event: a
/// second value for the same property replaces the first, a second binding of an event too.
fn is_last(bytes: &[u8], at: usize, attr: Attr<'_>) -> bool {
    records(&bytes[at..]).skip(1).all(|(_, other)| match (attr, other) {
        (Attr::Prop(a, _), Attr::Prop(b, _)) => a != b,
        (Attr::On(a, _, _), Attr::On(b, _, _)) => a != b,
        (Attr::Bind(a, _), Attr::Bind(b, _)) => a != b,
        _ => true,
    })
}

/// The attribute records of `bytes` that count: the last one of each property and event.
fn effective(bytes: &[u8]) -> impl Iterator<Item = Attr<'_>> {
    records(bytes).filter(move |(at, attr)| is_last(bytes, *at, *attr)).map(|(_, attr)| attr)
}

/// Adds `delta` to the tags of the event records and to the tag bases of the memo and reuse
/// records of `bytes`.
fn move_tags(bytes: &mut [u8], delta: i64) {
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let (op, size) = header_at(bytes, offset);
        if size < 8 || offset + size > bytes.len() {
            return;
        }
        let field = match op {
            OP_ON => Some(offset + 16),
            OP_MEMO => Some(offset + 16),
            OP_REUSE => Some(offset + 24),
            _ => None,
        };
        if let Some(at) = field {
            let moved = word_at(bytes, at).wrapping_add(delta);
            bytes[at..at + 8].copy_from_slice(&moved.to_le_bytes());
        }
        offset += size;
    }
}

// ---- decoding, for the tests ----------------------------------------------------------------

/// The key of an element among its siblings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    /// No key: the element is matched by its position among the siblings without one.
    None,
    Int(i64),
    Text(Box<str>),
}

/// A value of a property in a frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Text(String),
    /// The bits of the number, so that the same number compares equal (NaN included).
    Number(u64),
    Integer(i64),
    Bool(bool),
}

/// An element of a frame, decoded.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct Element {
    pub(crate) type_id: i64,
    pub(crate) key: Key,
    pub(crate) props: Vec<(i64, Value)>,
    /// (event, tag, value property).
    pub(crate) events: Vec<(i64, i64, i64)>,
    #[allow(dead_code)]
    pub(crate) children: Vec<Element>,
}

/// The elements at the top of a frame, decoded from its index.
#[cfg(test)]
pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<Element>> {
    fn element(frame: &Frame<'_>, index: u32) -> Element {
        let entry = frame.entry(index);
        let mut props = Vec::new();
        let mut events = Vec::new();
        for attr in effective(frame.attrs(entry)) {
            match attr {
                Attr::Prop(id, value) => props.push((id, value.owned())),
                Attr::On(event, tag, property) => events.push((event, tag, property)),
                Attr::Bind(..) => {}
            }
        }
        let children = siblings(frame, entry.first_child).map(|child| element(frame, child)).collect();
        Element { type_id: entry.type_id, key: frame.owned_key(entry), props, events, children }
    }
    let mut index = Index::default();
    index.build(bytes)?;
    let frame = Frame { bytes, index: &index };
    Ok(siblings(&frame, index.first).map(|top| element(&frame, top)).collect())
}

/// The entries of a list of siblings, from `first`.
fn siblings<'a>(frame: &'a Frame<'a>, first: u32) -> impl Iterator<Item = u32> + 'a {
    let mut next = first;
    std::iter::from_fn(move || {
        if next == NONE {
            return None;
        }
        let current = next;
        next = frame.entry(current).next;
        Some(current)
    })
}

// ---- the retained tree ----------------------------------------------------------------------

/// What an event binding dispatches: changed in place when a frame gives the event a new tag.
struct Target {
    tag: Cell<i64>,
    value_property: Cell<i64>,
    /// The value the last frame gave the value property: an event whose value is still this one
    /// was caused by the frame (a `TextChanged` posted after a commit wrote the text), not by the
    /// user, and is not dispatched.
    frame_value: RefCell<Option<Value>>,
}

impl Target {
    fn set_frame_value(&self, attrs: &[u8]) {
        let property = self.value_property.get();
        let value = if property == 0 { None } else { last_value(attrs, property).map(|value| value.owned()) };
        *self.frame_value.borrow_mut() = value;
    }

    /// Whether the value property of `sender` still has the value the last frame gave it.
    fn shows_frame_value(&self, sender: FerroHandle) -> bool {
        let property = self.value_property.get();
        match &*self.frame_value.borrow() {
            None => false,
            Some(Value::Text(text)) => values::shows_text(sender, property, text),
            Some(Value::Number(bits)) => values::peek_number(sender, property) == Some(f64::from_bits(*bits)),
            Some(Value::Integer(value)) => values::peek_number(sender, property) == Some(*value as f64),
            Some(Value::Bool(value)) => values::peek_number(sender, property) == Some(if *value { 1.0 } else { 0.0 }),
        }
    }
}

struct Binding {
    event: i64,
    subscription: i64,
    target: Rc<Target>,
}

/// An element of the retained tree: the object of an element of the last frame.
struct Node {
    type_id: i64,
    key: Key,
    handle: FerroHandle,
    /// The attribute records the element had in the last frame, as they were.
    attrs: Vec<u8>,
    bindings: Vec<Binding>,
    children: Vec<Node>,
    /// The version and the tag base of a memo element.
    memo: Option<(i64, i64)>,
    /// The signal bindings of `bind` records: property, slot, binding ID.
    signals: Vec<(i64, i64, i64)>,
}

/// The bytes of a memo element with an integer key, as the frame that last wrote it in full had
/// them: what a reuse record stands for.
struct Stored {
    version: i64,
    tag_base: i64,
    /// The number of tags the subtree uses from its tag base.
    tags: i64,
    bytes: Vec<u8>,
    /// The number of elements of the retained tree that are memo elements of this key.
    nodes: u32,
}

/// The memo elements of a root by key: integer keys the application chose, hashed with one
/// multiplication (the default hasher costs more than the rest of a reuse).
type MemoMap = HashMap<i64, Stored, BuildHasherDefault<KeyHasher>>;

#[derive(Default)]
struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn write_i64(&mut self, value: i64) {
        self.0 = (value as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(26);
    }
}

/// The retained tree of a root and the memo elements in it.
#[derive(Default)]
struct RootState {
    nodes: Vec<Node>,
    memo: MemoMap,
}

/// The dispatcher of the events bound in frames: one per process, set from any thread (before
/// the application runs, so that a language passes it as the named function it is). An atomic
/// pointer, so that an event reads it with one load.
static DISPATCHER: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[inline]
fn dispatcher() -> FerroEventFn {
    let function = DISPATCHER.load(Ordering::Acquire);
    if function.is_null() {
        None
    } else {
        // SAFETY: the pointer was stored from a `FerroEventFn` by `ferro_frame_dispatcher`.
        Some(unsafe {
            std::mem::transmute::<*mut c_void, unsafe extern "C" fn(i64, FerroHandle, i64) -> i64>(function)
        })
    }
}

// The state of the frames belongs to the UI thread: statics behind `UiCell`, so that an event
// and a commit reach it with one check of the thread and no thread-local lookup.

/// The retained trees, by root (few roots: a list).
static RETAINED: UiCell<Vec<(FerroHandle, RootState)>> = UiCell::new(Vec::new());
/// The index of the frame being committed, kept for the next commit.
static INDEX: UiCell<Index> = UiCell::new(Index::new());
/// Commits and dispatches.
static STATE: UiCell<State> = UiCell::new(State {
    committing: false,
    dispatching: 0,
    removed: Vec::new(),
    event_text: Vec::new(),
    event_number: 0.0,
});

struct State {
    /// Whether a commit is writing values: events raised meanwhile are not dispatched.
    committing: bool,
    /// How many dispatches run: nodes removed inside one are freed once the outermost returns.
    dispatching: u32,
    /// Nodes removed while a dispatch runs (their handler may be the one running).
    removed: Vec<Node>,
    /// The value of the event being dispatched (`ferro_frame_event_text`, `_f64`): the text with
    /// its NUL, in a buffer kept from one event to the next.
    event_text: Vec<u8>,
    event_number: f64,
}

/// Whether a commit is writing values.
#[inline]
fn committing() -> bool {
    STATE.with(|state| state.committing).unwrap_or(false)
}

/// Remembers the first failure of a commit; the others are recorded as they happen.
struct Failures(Option<Error>);

impl Failures {
    fn check(&mut self, code: FerroStatus) {
        if code < 0 && self.0.is_none() {
            // SAFETY: `ferro_last_error` returns a NUL-terminated string.
            let message = unsafe { std::ffi::CStr::from_ptr(crate::ferro_last_error()) }.to_string_lossy().into_owned();
            self.0 = Some(Error::new(code, message));
        }
    }
}

/// Resets the commit flag also when a commit panics.
struct Committing(bool);

impl Committing {
    fn start() -> Self {
        Committing(STATE.with(|state| std::mem::replace(&mut state.committing, true)).unwrap_or(false))
    }
}

impl Drop for Committing {
    fn drop(&mut self) {
        let _ = STATE.with(|state| state.committing = self.0);
    }
}

/// The state of one commit.
struct Commit<'s> {
    failures: Failures,
    memo: &'s mut MemoMap,
    /// The keys of the memo elements taken out by this commit: their bytes are dropped at its
    /// end when no element of the retained tree has the key then.
    dropped: Vec<i64>,
}

fn commit(root: FerroHandle, bytes: &[u8]) -> Result<()> {
    if committing() {
        return Err(Error::new(FERRO_INVALID_STATE, "a frame is being committed: commit after it, not inside it"));
    }
    // The start of a commit is a point where the signals written since the last one are applied.
    crate::signals::apply_pending();
    if root != 0 {
        handles::get(root)?;
    }
    let mut index = INDEX.with(std::mem::take)?;
    if let Err(error) = index.build(bytes) {
        INDEX.with(|kept| *kept = index)?;
        return Err(error);
    }
    let _committing = Committing::start();
    let state = RETAINED.with(|retained| {
        let position = retained.iter().position(|(handle, _)| *handle == root)?;
        Some(retained.swap_remove(position).1)
    })?;
    let first = state.is_none();
    let mut state = state.unwrap_or_default();
    let mut commit = Commit { failures: Failures(None), memo: &mut state.memo, dropped: Vec::new() };
    if first && root != 0 {
        // The handle of the root stays valid for the next commits.
        commit.failures.check(ferro_retain(root));
    }
    let frame = Frame { bytes, index: &index };
    reconcile_children(&mut commit, root, &mut state.nodes, &frame, index.first);
    for key in std::mem::take(&mut commit.dropped) {
        if commit.memo.get(&key).is_some_and(|stored| stored.nodes == 0) {
            commit.memo.remove(&key);
        }
    }
    let failures = commit.failures.0.take();
    RETAINED.with(|retained| retained.push((root, state)))?;
    INDEX.with(|kept| *kept = index)?;
    match failures {
        None => Ok(()),
        Some(error) => Err(error),
    }
}

/// Whether the node `node` stands for the entry `entry` by position: the same key and class, or
/// for a reuse record, the memo element of its key and version.
fn matches(frame: &Frame<'_>, node: &Node, entry: &Entry) -> bool {
    if !frame.same_key(&node.key, entry) {
        return false;
    }
    if entry.reuse {
        node.memo.map(|(version, _)| version) == entry.memo.map(|(version, _)| version)
    } else {
        node.type_id == entry.type_id
    }
}

/// Reconciles the children `old` of `parent` into the siblings of the frame from `first`.
fn reconcile_children(
    commit: &mut Commit<'_>,
    parent: FerroHandle,
    old: &mut Vec<Node>,
    frame: &Frame<'_>,
    first: u32,
) {
    if first == NONE && old.is_empty() {
        return;
    }
    // The same keys and classes in the same order: matched by position, in place.
    let mut count = 0;
    let mut same = true;
    for index in siblings(frame, first) {
        match old.get(count) {
            Some(node) if matches(frame, node, frame.entry(index)) => count += 1,
            _ => {
                same = false;
                break;
            }
        }
    }
    if same && count == old.len() {
        for (node, index) in old.iter_mut().zip(siblings(frame, first)) {
            reconcile(commit, node, frame, index);
        }
        return;
    }
    let previous = std::mem::take(old);
    let before: Vec<FerroHandle> = previous.iter().map(|node| node.handle).collect();
    let mut keyed: HashMap<Key, Node> = HashMap::new();
    let mut unkeyed: VecDeque<Node> = VecDeque::new();
    for node in previous {
        if node.key == Key::None {
            unkeyed.push_back(node);
        } else {
            keyed.insert(node.key.clone(), node);
        }
    }
    let mut nodes = Vec::new();
    // Taken out of service after the children of the parent are set: their handles are valid
    // until then.
    let mut removed = Vec::new();
    for index in siblings(frame, first) {
        let entry = frame.entry(index);
        let candidate = match entry.key {
            KeyRef::None => unkeyed.pop_front(),
            _ => keyed.remove(&frame.owned_key(entry)),
        };
        let node = match candidate {
            Some(mut node) if matches(frame, &node, entry) => {
                reconcile(commit, &mut node, frame, index);
                Some(node)
            }
            Some(node) if entry.reuse => replay(commit, Some(node), entry, &mut removed),
            other => {
                removed.extend(other);
                create(commit, frame, index)
            }
        };
        nodes.extend(node);
    }
    removed.extend(unkeyed);
    removed.extend(keyed.into_values());
    // The children of the parent: what stays in order stays; the rest is taken out and put in.
    let after: Vec<FerroHandle> = nodes.iter().map(|node| node.handle).collect();
    if parent == 0 {
        // The top of the frames of no root: windows, shown when they come and closed when they go.
        for &handle in after.iter().filter(|handle| !before.contains(handle)) {
            commit.failures.check(crate::ferro_show(handle));
        }
        for &handle in before.iter().filter(|handle| !after.contains(handle)) {
            let _ = crate::ferro_close(handle);
        }
    } else if before != after {
        let common = before.iter().zip(&after).take_while(|(a, b)| a == b).count();
        for &handle in &before[common..] {
            let code = ferro_remove_child(parent, handle);
            if code != crate::FERRO_NOT_FOUND {
                commit.failures.check(code);
            }
        }
        for &handle in &after[common..] {
            commit.failures.check(ferro_add_child(parent, handle));
        }
    }
    for node in removed {
        retire(commit, node);
    }
    *old = nodes;
}

/// Reconciles `node` into the entry `index`, which [`matches`] it.
fn reconcile(commit: &mut Commit<'_>, node: &mut Node, frame: &Frame<'_>, index: u32) {
    let entry = frame.entry(index);
    if let (Some((version, tag_base)), Some((kept, kept_base))) = (entry.memo, node.memo) {
        if version == kept {
            // The memo element of the last frame: its subtree is not looked at.
            if tag_base != kept_base {
                move_node_tags(node, tag_base - kept_base);
            }
            return;
        }
    }
    update(commit, node, frame.attrs(entry));
    reconcile_children(commit, node.handle, &mut node.children, frame, entry.first_child);
    let counted = node.memo.is_some();
    if counted && entry.memo.is_none() {
        if let Key::Int(key) = node.key {
            uncount(commit, key);
        }
    }
    node.memo = entry.memo;
    store(commit, node, frame, entry, counted);
}

/// Keeps the bytes of a memo element with an integer key written in full, and counts the node
/// under its key unless it is `counted` already.
#[inline]
fn store(commit: &mut Commit<'_>, node: &Node, frame: &Frame<'_>, entry: &Entry, counted: bool) {
    let (Some((version, tag_base)), Key::Int(key)) = (entry.memo, &node.key) else {
        return;
    };
    let bytes = frame.subtree(entry);
    let tags = tags_of(commit, bytes, tag_base);
    let stored =
        commit.memo.entry(*key).or_insert_with(|| Stored { version, tag_base, tags, bytes: Vec::new(), nodes: 0 });
    stored.version = version;
    stored.tag_base = tag_base;
    stored.tags = tags;
    stored.bytes.clear();
    stored.bytes.extend_from_slice(bytes);
    if !counted {
        stored.nodes += 1;
    }
}

/// One node fewer under the memo key `key`; its bytes are dropped at the end of the commit when
/// no node has the key then (a reuse later in the same commit may still need them).
fn uncount(commit: &mut Commit<'_>, key: i64) {
    if let Some(stored) = commit.memo.get_mut(&key) {
        stored.nodes = stored.nodes.saturating_sub(1);
        commit.dropped.push(key);
    }
}

/// The number of tags the subtree `bytes` uses from `tag_base`: up to its largest tag, the
/// tags of the memo elements it reuses included.
fn tags_of(commit: &Commit<'_>, bytes: &[u8], tag_base: i64) -> i64 {
    let mut last = tag_base - 1;
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let (op, size) = header_at(bytes, offset);
        if size < 8 || offset + size > bytes.len() {
            break;
        }
        match op {
            OP_ON => last = last.max(word_at(bytes, offset + 16)),
            OP_REUSE => {
                let (key, version, base) =
                    (word_at(bytes, offset + 8), word_at(bytes, offset + 16), word_at(bytes, offset + 24));
                if let Some(stored) = commit.memo.get(&key).filter(|stored| stored.version == version) {
                    if stored.tags > 0 {
                        last = last.max(base + stored.tags - 1);
                    }
                }
            }
            _ => {}
        }
        offset += size;
    }
    (last - tag_base + 1).max(0)
}

/// Moves the tags of the events of a subtree of the retained tree by `delta`.
fn move_node_tags(node: &mut Node, delta: i64) {
    move_tags(&mut node.attrs, delta);
    for binding in &node.bindings {
        binding.target.tag.set(binding.target.tag.get().wrapping_add(delta));
    }
    if let Some((_, base)) = &mut node.memo {
        *base = base.wrapping_add(delta);
    }
    for child in &mut node.children {
        move_node_tags(child, delta);
    }
}

/// A reuse record whose memo element is not the node in its place (it moved, or the node has
/// another version): the bytes kept for its key are committed in its place, into `candidate`
/// when it is of the same class.
fn replay(commit: &mut Commit<'_>, candidate: Option<Node>, entry: &Entry, removed: &mut Vec<Node>) -> Option<Node> {
    let (KeyRef::Int(key), Some((version, tag_base))) = (entry.key, entry.memo) else {
        return candidate;
    };
    let kept = commit.memo.get(&key).filter(|stored| stored.version == version);
    let kept = kept.map(|stored| (stored.bytes.clone(), stored.tag_base));
    let Some((mut bytes, kept_base)) = kept else {
        let error =
            Error::new(FERRO_INVALID_STATE, format!("no memo element of the key {key} has the version {version}"));
        commit.failures.check(error.record());
        removed.extend(candidate);
        return None;
    };
    if tag_base != kept_base {
        move_tags(&mut bytes, tag_base - kept_base);
    }
    let mut index = Index::default();
    if let Err(error) = index.build(&bytes) {
        commit.failures.check(error.record());
        return candidate;
    }
    if index.first == NONE {
        return candidate;
    }
    let frame = Frame { bytes: &bytes, index: &index };
    let top = frame.entry(index.first);
    match candidate {
        Some(mut node) if node.type_id == top.type_id && !top.reuse => {
            reconcile(commit, &mut node, &frame, index.first);
            Some(node)
        }
        other => {
            removed.extend(other);
            create(commit, &frame, index.first)
        }
    }
}

fn write_value(handle: FerroHandle, property: i64, value: ValueRef<'_>) -> FerroStatus {
    match value {
        ValueRef::Text(_) | ValueRef::Text16(_) => {
            let Some(text) = value.text() else {
                let unit = if matches!(value, ValueRef::Text(_)) { "UTF-8" } else { "UTF-16" };
                return Error::new(crate::FERRO_INVALID_UTF8, format!("a text of the frame is not {unit}")).record();
            };
            // What the object shows already is not written again (the text typed into a box).
            if values::shows_text(handle, property, &text) {
                return FERRO_OK;
            }
            // SAFETY: a pointer and the length of a Rust string.
            unsafe { ferro_set_str(handle, property, text.as_ptr().cast(), text.len() as i64) }
        }
        ValueRef::Number(bits) => ferro_set_f64(handle, property, f64::from_bits(bits)),
        ValueRef::Integer(value) => ferro_set_i64(handle, property, value),
        ValueRef::Bool(value) => ferro_set_bool(handle, property, value),
    }
}

fn create(commit: &mut Commit<'_>, frame: &Frame<'_>, index: u32) -> Option<Node> {
    let entry = frame.entry(index);
    if entry.reuse {
        return replay(commit, None, entry, &mut Vec::new());
    }
    let handle = ferro_create(entry.type_id);
    if handle == 0 {
        commit.failures.check(crate::ferro_last_error_code());
        return None;
    }
    let attrs = frame.attrs(entry);
    for attr in effective(attrs) {
        if let Attr::Prop(property, value) = attr {
            commit.failures.check(write_value(handle, property, value));
        }
    }
    let mut children = Vec::new();
    reconcile_children(commit, handle, &mut children, frame, entry.first_child);
    let mut bindings = Vec::new();
    for attr in effective(attrs) {
        if let Attr::On(event, tag, value_property) = attr {
            bindings.extend(bind(handle, event, tag, value_property, &mut commit.failures));
        }
    }
    for binding in &bindings {
        binding.target.set_frame_value(attrs);
    }
    let mut signals = Vec::new();
    for attr in effective(attrs) {
        if let Attr::Bind(property, slot) = attr {
            signals.extend(bind_signal(handle, property, slot, &mut commit.failures));
        }
    }
    let node = Node {
        type_id: entry.type_id,
        key: frame.owned_key(entry),
        handle,
        attrs: attrs.to_vec(),
        bindings,
        children,
        memo: entry.memo,
        signals,
    };
    store(commit, &node, frame, entry, false);
    Some(node)
}

/// Writes what changed in the attribute records of `node`.
fn update(commit: &mut Commit<'_>, node: &mut Node, attrs: &[u8]) {
    if node.attrs == attrs {
        return;
    }
    let handle = node.handle;
    for attr in effective(attrs) {
        if let Attr::Prop(property, value) = attr {
            if last_value(&node.attrs, property) != Some(value) {
                commit.failures.check(write_value(handle, property, value));
            }
        }
    }
    for attr in effective(&node.attrs) {
        if let Attr::Prop(property, _) = attr {
            if last_value(attrs, property).is_none() {
                commit.failures.check(ferro_clear(handle, property));
            }
        }
    }
    // Events: the bindings of the same events take the new tags; the others come and go.
    let bound = |event: i64| {
        effective(attrs).find_map(|attr| match attr {
            Attr::On(id, tag, value_property) if id == event => Some((tag, value_property)),
            _ => None,
        })
    };
    node.bindings.retain(|binding| match bound(binding.event) {
        Some((tag, value_property)) => {
            binding.target.tag.set(tag);
            binding.target.value_property.set(value_property);
            true
        }
        None => {
            let _ = ferro_unsubscribe(binding.subscription);
            false
        }
    });
    for attr in effective(attrs) {
        if let Attr::On(event, tag, value_property) = attr {
            if !node.bindings.iter().any(|binding| binding.event == event) {
                node.bindings.extend(bind(handle, event, tag, value_property, &mut commit.failures));
            }
        }
    }
    for binding in &node.bindings {
        binding.target.set_frame_value(attrs);
    }
    // Signal bindings: the same property and slot stays bound; the others come and go.
    let bound_to = |property: i64| {
        effective(attrs).find_map(|attr| match attr {
            Attr::Bind(id, slot) if id == property => Some(slot),
            _ => None,
        })
    };
    node.signals.retain(|&(property, slot, id)| {
        let keep = bound_to(property) == Some(slot);
        if !keep {
            let _ = crate::signals::unbind(id);
        }
        keep
    });
    for attr in effective(attrs) {
        if let Attr::Bind(property, slot) = attr {
            if !node.signals.iter().any(|&(bound, _, _)| bound == property) {
                node.signals.extend(bind_signal(handle, property, slot, &mut commit.failures));
            }
        }
    }
    node.attrs.clear();
    node.attrs.extend_from_slice(attrs);
}

/// Binds a property of an element to a signal slot: (property, slot, binding ID).
fn bind_signal(handle: FerroHandle, property: i64, slot: i64, failures: &mut Failures) -> Option<(i64, i64, i64)> {
    match crate::signals::bind(slot, handle, property, false) {
        Ok(id) => Some((property, slot, id)),
        Err(error) => {
            failures.check(error.record());
            None
        }
    }
}

fn bind(handle: FerroHandle, event: i64, tag: i64, value_property: i64, failures: &mut Failures) -> Option<Binding> {
    let target = Rc::new(Target {
        tag: Cell::new(tag),
        value_property: Cell::new(value_property),
        frame_value: RefCell::new(None),
    });
    let user_data = Rc::into_raw(target.clone()) as *mut c_void;
    let subscription = ferro_subscribe_data(handle, event, Some(on_event), user_data, Some(release_target));
    if subscription < 0 {
        failures.check(subscription as FerroStatus);
        return None;
    }
    Some(Binding { event, subscription, target })
}

unsafe extern "C" fn release_target(user_data: *mut c_void) {
    // SAFETY: the user data is the `Rc<Target>` given to `ferro_subscribe_data` by `bind`.
    drop(unsafe { Rc::from_raw(user_data as *const Target) });
}

/// Ends a dispatch, also when the dispatcher panics: the nodes removed meanwhile are freed once
/// the outermost dispatch returns.
struct Dispatching;

impl Dispatching {
    /// Starts a dispatch whose event has the value `text` and `number`.
    #[inline]
    fn start(text: &str, number: f64) -> Self {
        let _ = STATE.with(|state| {
            state.event_text.clear();
            if !text.is_empty() {
                state.event_text.extend(text.bytes().filter(|byte| *byte != 0));
            }
            state.event_text.push(0);
            state.event_number = number;
            state.dispatching += 1;
        });
        Dispatching
    }
}

impl Drop for Dispatching {
    #[inline]
    fn drop(&mut self) {
        let (outermost, removed) = STATE
            .with(|state| {
                state.dispatching -= 1;
                let outermost = state.dispatching == 0;
                if outermost && !state.removed.is_empty() {
                    (true, std::mem::take(&mut state.removed))
                } else {
                    (outermost, Vec::new())
                }
            })
            .unwrap_or_default();
        // Freed with no access of the state running: freeing runs code of the framework.
        for node in removed {
            free(node);
        }
        // The end of a dispatch is a point where the signals the dispatcher wrote are applied.
        if outermost {
            crate::signals::apply_pending();
        }
    }
}

unsafe extern "C" fn on_event(user_data: *mut c_void, sender: FerroHandle, _arg: i64) -> i64 {
    if committing() {
        return 0;
    }
    let Some(dispatcher) = dispatcher() else {
        return 0;
    };
    // Read before the dispatcher runs: the commit it makes may unbind this very event.
    // SAFETY: the user data is the `Rc<Target>` of the subscription, alive while it is called.
    let (tag, value_property) = {
        let target = unsafe { &*(user_data as *const Target) };
        if target.shows_frame_value(sender) {
            return 0;
        }
        (target.tag.get(), target.value_property.get())
    };
    let _dispatching;
    let mut arg = 0;
    if value_property != 0 {
        let text = values::peek_text(sender, value_property).unwrap_or_default();
        let number = values::peek_number(sender, value_property).unwrap_or(0.0);
        _dispatching = Dispatching::start(&text, number);
        arg = number as i64;
    } else {
        _dispatching = Dispatching::start("", 0.0);
    }
    // SAFETY: the dispatcher is a function of the documented signature.
    unsafe { dispatcher(tag, sender, arg) }
}

/// Takes a node removed by a commit out of service: the bytes of the memo elements in it are
/// dropped at the end of the commit, unless the commit kept their keys elsewhere.
fn retire(commit: &mut Commit<'_>, node: Node) {
    fn uncount_all(commit: &mut Commit<'_>, node: &Node) {
        if let (Some(_), Key::Int(key)) = (node.memo, &node.key) {
            uncount(commit, *key);
        }
        for child in &node.children {
            uncount_all(commit, child);
        }
    }
    if !commit.memo.is_empty() {
        uncount_all(commit, &node);
    }
    free(node);
}

/// Takes a node out of service: its events unbound and its handles released, children first. A
/// node removed while a dispatch runs waits for the dispatch to end.
fn free(node: Node) {
    let mut node = Some(node);
    let _ = STATE.with(|state| {
        if state.dispatching > 0 {
            state.removed.extend(node.take());
        }
    });
    let Some(node) = node else {
        return;
    };
    for binding in node.bindings {
        let _ = ferro_unsubscribe(binding.subscription);
    }
    for (_, _, id) in node.signals {
        let _ = crate::signals::unbind(id);
    }
    for child in node.children {
        free(child);
    }
    let _ = ferro_release(node.handle);
}

/// Sets the dispatcher of the events bound in frames, one for the process: called on the UI thread
/// with the tag of the binding, the sender (borrowed for the call) and the value of its value
/// property as an integer (a number truncated, a boolean as `0` or `1`). A result other than `0`
/// marks a routed event as handled. `NULL` stops dispatching. Any thread: a language whose
/// callbacks must be named functions at the call site (Osprey) sets it before
/// [`ferro_app_run`](crate::ferro_app_run).
#[no_mangle]
pub extern "C" fn ferro_frame_dispatcher(function: FerroEventFn) -> FerroStatus {
    let pointer = function.map_or(std::ptr::null_mut(), |function| function as *mut c_void);
    DISPATCHER.store(pointer, Ordering::Release);
    FERRO_OK
}

/// Calls the dispatcher for an event subscribed with
/// [`ferro_subscribe_dispatch`](crate::ferro_subscribe_dispatch), on the UI thread: `0` without a
/// dispatcher.
#[inline]
pub(crate) fn dispatch_tag(tag: i64, sender: FerroHandle, arg: i64) -> i64 {
    let Some(dispatcher) = dispatcher() else {
        return 0;
    };
    let _dispatching = Dispatching::start("", 0.0);
    // SAFETY: the dispatcher is a function of the documented signature.
    unsafe { dispatcher(tag, sender, arg) }
}

/// Calls the dispatcher of the thread ([`ferro_frame_dispatcher`]) with `tag`, `sender` and `arg`
/// as an event would, and returns its result: a binding starts its loop with it (the tag `0`
/// asks for the first frame), its one trampoline forwards the events of its subscriptions with
/// it, and a test drives a message without an event. The event text and number are empty.
/// [`FERRO_INVALID_STATE`] (as a negative result) without a dispatcher. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_dispatch(tag: i64, sender: FerroHandle, arg: i64) -> i64 {
    crate::error::status_or_value(|| {
        thread::check()?;
        let dispatcher = dispatcher().ok_or_else(|| {
            Error::new(FERRO_INVALID_STATE, "no dispatcher is set: call ferro_frame_dispatcher first")
        })?;
        let _dispatching = Dispatching::start("", 0.0);
        // SAFETY: the dispatcher is a function of the documented signature.
        Ok(unsafe { dispatcher(tag, sender, arg) })
    })
}

/// The text of the value property of the event being dispatched, NUL-terminated and owned by the
/// library until the next event; empty for a binding without a value property or outside a
/// dispatch. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_event_text() -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    STATE
        .with(|state| if state.event_text.is_empty() { EMPTY.as_ptr() } else { state.event_text.as_ptr() })
        .unwrap_or(EMPTY.as_ptr())
        .cast()
}

/// The value property of the event being dispatched as a number (`0` when it is not one). UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_event_f64() -> f64 {
    STATE.with(|state| state.event_number).unwrap_or(0.0)
}

/// The handle of the element with the integer key `key` in the retained tree of `root`, with one
/// reference counted (release it), or `0` with [`FERRO_NOT_FOUND`]: for tests and for driving an
/// interface built from frames. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_find(root: FerroHandle, key: i64) -> FerroHandle {
    fn find(nodes: &[Node], key: &Key) -> Option<FerroHandle> {
        nodes.iter().find_map(|node| if &node.key == key { Some(node.handle) } else { find(&node.children, key) })
    }
    guard(0, || {
        let found = RETAINED
            .with(|retained| {
                retained
                    .iter()
                    .find(|(handle, _)| *handle == root)
                    .and_then(|(_, state)| find(&state.nodes, &Key::Int(key)))
            })
            .and_then(|found| {
                found.ok_or_else(|| {
                    Error::new(FERRO_NOT_FOUND, format!("no element of the frames of the root has the key {key}"))
                })
            });
        match found {
            Ok(handle) if ferro_retain(handle) == FERRO_OK => handle,
            Ok(_) => 0,
            Err(error) => {
                error.record();
                0
            }
        }
    })
}

/// Takes the elements of the frames of `root` out of it, frees them, and lets the reference to
/// `root` go. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_clear(root: FerroHandle) -> FerroStatus {
    status(|| {
        let state = RETAINED.with(|retained| {
            let position = retained.iter().position(|(handle, _)| *handle == root)?;
            Some(retained.swap_remove(position).1)
        })?;
        let Some(state) = state else {
            return Err(Error::new(FERRO_NOT_FOUND, "no frame was committed under the root"));
        };
        for node in state.nodes {
            if root == 0 {
                let _ = crate::ferro_close(node.handle);
            } else {
                let _ = ferro_remove_child(root, node.handle);
            }
            free(node);
        }
        if root != 0 {
            let _ = ferro_release(root);
        }
        Ok(())
    })
}
