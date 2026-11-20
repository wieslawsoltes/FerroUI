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
use crate::{
    ferro_add_child, ferro_clear, ferro_create, ferro_release, ferro_remove_child, ferro_retain, ferro_set_bool,
    ferro_set_f64, ferro_set_i64, ferro_set_str, ferro_subscribe_data, ferro_unsubscribe, handles, thread, values,
    FerroEventFn, FerroHandle, FerroStatus, FERRO_INVALID_ARGUMENT, FERRO_INVALID_STATE, FERRO_NOT_FOUND, FERRO_OK,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;
use std::sync::{Mutex, PoisonError};

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

// ---- the writer -----------------------------------------------------------------------------

ferroui_base::ferro_thread_local! {
    /// The frame the scalar writer of this thread writes.
    static WRITER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Appends a record: the header, the words, then `tail` padded with zeros to 8 bytes.
pub(crate) fn write_record(buffer: &mut Vec<u8>, op: u32, words: &[i64], tail: &[u8]) {
    let size = 8 + 8 * words.len() + tail.len().div_ceil(8) * 8;
    buffer.extend_from_slice(&op.to_le_bytes());
    buffer.extend_from_slice(&(size as u32).to_le_bytes());
    for word in words {
        buffer.extend_from_slice(&word.to_le_bytes());
    }
    buffer.extend_from_slice(tail);
    buffer.resize(buffer.len() + (tail.len().div_ceil(8) * 8 - tail.len()), 0);
}

fn write(op: u32, words: &[i64], tail: &[u8]) -> FerroStatus {
    guard(crate::FERRO_PANIC, || {
        WRITER.with_borrow_mut(|buffer| write_record(buffer, op, words, tail));
        FERRO_OK
    })
}

/// The bytes of a string argument; a negative length means NUL-terminated.
unsafe fn text_bytes<'a>(text: *const c_char, length: i64) -> Result<&'a [u8]> {
    // SAFETY: forwarded from the caller of the exported function.
    unsafe { crate::strings::borrowed(text, length) }.map(str::as_bytes)
}

/// Starts a frame on the calling thread: empties the buffer of the scalar writer. Any thread (the
/// buffer is the calling thread's); commit it on the UI thread.
#[no_mangle]
pub extern "C" fn ferro_frame_begin() -> FerroStatus {
    guard(crate::FERRO_PANIC, || {
        WRITER.with_borrow_mut(Vec::clear);
        FERRO_OK
    })
}

/// Opens an element of the class `type_id` ([`ferro_type_id`](crate::ferro_type_id)) in the frame
/// of the calling thread. `key` tells siblings apart across frames: `0` keeps the element by its
/// position among the siblings without a key. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_open(type_id: i64, key: i64) -> FerroStatus {
    write(OP_OPEN, &[type_id, key], &[])
}

/// Gives the element opened last a text key in place of its integer key. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_key_str(text: *const c_char, length: i64) -> FerroStatus {
    // SAFETY: the caller passes a string as `ferro_set_str` documents.
    match unsafe { text_bytes(text, length) } {
        Ok(bytes) => write(OP_KEY_TEXT, &[bytes.len() as i64], bytes),
        Err(error) => error.record(),
    }
}

/// Closes the element opened last in the frame of the calling thread. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_close() -> FerroStatus {
    write(OP_CLOSE, &[], &[])
}

/// Sets `property` of the open element from text, as [`ferro_set_str`] does when the frame is
/// committed. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_frame_str(property: i64, text: *const c_char, length: i64) -> FerroStatus {
    // SAFETY: the caller passes a string as `ferro_set_str` documents.
    match unsafe { text_bytes(text, length) } {
        Ok(bytes) => write(OP_TEXT, &[property, bytes.len() as i64], bytes),
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
    write(OP_TEXT16, &[property, units as i64], &bytes)
}

/// Sets `property` of the open element from a number, as [`ferro_set_f64`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_f64(property: i64, value: f64) -> FerroStatus {
    write(OP_NUMBER, &[property, value.to_bits() as i64], &[])
}

/// Sets `property` of the open element from an integer or the value of an enumeration member, as
/// [`ferro_set_i64`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_i64(property: i64, value: i64) -> FerroStatus {
    write(OP_INTEGER, &[property, value], &[])
}

/// Sets `property` of the open element from a boolean, as [`ferro_set_bool`] does. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_bool(property: i64, value: bool) -> FerroStatus {
    write(OP_BOOL, &[property, i64::from(value)], &[])
}

/// Binds the event `event` ([`ferro_event_id`](crate::ferro_event_id)) of the open element to the
/// message tag `tag`: when it fires, the dispatcher of the thread ([`ferro_frame_dispatcher`]) is
/// called with the tag, the sender, and the value of `value_property` (`0` for none), whose text
/// and number [`ferro_frame_event_text`] and [`ferro_frame_event_f64`] give. Any thread.
#[no_mangle]
pub extern "C" fn ferro_frame_on(event: i64, tag: i64, value_property: i64) -> FerroStatus {
    write(OP_ON, &[event, tag, value_property], &[])
}

/// Commits the frame of the scalar writer of the calling thread under `root` (see
/// [`ferro_frame_commit_bytes`]) and empties the buffer. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_commit(root: FerroHandle) -> FerroStatus {
    status(|| {
        thread::check()?;
        let bytes = WRITER.with_borrow_mut(std::mem::take);
        let result = commit(root, &bytes);
        // The buffer keeps its capacity for the next frame.
        WRITER.with_borrow_mut(|buffer| {
            if buffer.is_empty() {
                *buffer = bytes;
                buffer.clear();
            }
        });
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
        let length = usize::try_from(length).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "the length is negative"))?;
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

// ---- decoding -------------------------------------------------------------------------------

/// The key of an element among its siblings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    /// No key: the element is matched by its position among the siblings without one.
    None,
    Int(i64),
    Text(String),
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

/// An element of a frame.
#[derive(Debug)]
pub(crate) struct Element {
    pub(crate) type_id: i64,
    pub(crate) key: Key,
    pub(crate) props: Vec<(i64, Value)>,
    /// (event, tag, value property).
    pub(crate) events: Vec<(i64, i64, i64)>,
    pub(crate) children: Vec<Element>,
}

#[cold]
fn malformed(what: &str, offset: usize) -> Error {
    Error::new(FERRO_INVALID_ARGUMENT, format!("the frame is malformed at byte {offset}: {what}"))
}

/// The elements at the top of a frame.
pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<Element>> {
    let word = |offset: usize| -> i64 { i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap_or_default()) };
    let mut stack: Vec<Element> = Vec::new();
    let mut top = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Err(malformed("a record header is cut short", offset));
        }
        let op = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap_or_default());
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap_or_default()) as usize;
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
        let tail = |count: usize, length: usize| -> Result<&[u8]> {
            let start = body + count * 8;
            if length > size - 8 - count * 8 {
                return Err(malformed("a text is longer than its record", offset));
            }
            Ok(&bytes[start..start + length])
        };
        let open = |stack: &mut Vec<Element>| -> Result<()> {
            if stack.is_empty() {
                Err(malformed("a value or an event outside an element", offset))
            } else {
                Ok(())
            }
        };
        match op {
            OP_OPEN => {
                need(2)?;
                let key = match word(body + 8) {
                    0 => Key::None,
                    key => Key::Int(key),
                };
                let (props, events, children) = (Vec::new(), Vec::new(), Vec::new());
                stack.push(Element { type_id: word(body), key, props, events, children });
            }
            OP_CLOSE => {
                let element = stack.pop().ok_or_else(|| {
                    let message = format!("the frame closes an element it did not open (byte {offset})");
                    Error::new(FERRO_INVALID_STATE, message)
                })?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => top.push(element),
                }
            }
            OP_TEXT | OP_TEXT16 => {
                need(2)?;
                open(&mut stack)?;
                let property = word(body);
                let count = usize::try_from(word(body + 8)).map_err(|_| malformed("a negative length", offset))?;
                let text = if op == OP_TEXT {
                    let bytes = tail(2, count)?;
                    std::str::from_utf8(bytes)
                        .map_err(|_| Error::new(crate::FERRO_INVALID_UTF8, "a text of the frame is not UTF-8"))?
                        .to_owned()
                } else {
                    let bytes = tail(2, count.checked_mul(2).ok_or_else(|| malformed("a length too large", offset))?)?;
                    let units: Vec<u16> =
                        bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
                    String::from_utf16(&units)
                        .map_err(|_| Error::new(crate::FERRO_INVALID_UTF8, "a text of the frame is not UTF-16"))?
                };
                set(&mut stack, property, Value::Text(text));
            }
            OP_NUMBER | OP_INTEGER | OP_BOOL => {
                need(2)?;
                open(&mut stack)?;
                let value = match op {
                    OP_NUMBER => Value::Number(word(body + 8) as u64),
                    OP_INTEGER => Value::Integer(word(body + 8)),
                    _ => Value::Bool(word(body + 8) != 0),
                };
                set(&mut stack, word(body), value);
            }
            OP_ON => {
                need(3)?;
                open(&mut stack)?;
                let binding = (word(body), word(body + 8), word(body + 16));
                if let Some(element) = stack.last_mut() {
                    match element.events.iter_mut().find(|(event, _, _)| *event == binding.0) {
                        Some(existing) => *existing = binding,
                        None => element.events.push(binding),
                    }
                }
            }
            OP_KEY_TEXT => {
                need(1)?;
                open(&mut stack)?;
                let count = usize::try_from(word(body)).map_err(|_| malformed("a negative length", offset))?;
                let key = std::str::from_utf8(tail(1, count)?)
                    .map_err(|_| Error::new(crate::FERRO_INVALID_UTF8, "a key of the frame is not UTF-8"))?
                    .to_owned();
                if let Some(element) = stack.last_mut() {
                    element.key = Key::Text(key);
                }
            }
            _ => return Err(malformed("an unknown record", offset)),
        }
        offset += size;
    }
    if !stack.is_empty() {
        return Err(Error::new(FERRO_INVALID_STATE, format!("the frame leaves {} elements open", stack.len())));
    }
    Ok(top)
}

/// Sets a value on the open element; a second value for the same property replaces the first.
fn set(stack: &mut [Element], property: i64, value: Value) {
    if let Some(element) = stack.last_mut() {
        match element.props.iter_mut().find(|(id, _)| *id == property) {
            Some(existing) => existing.1 = value,
            None => element.props.push((property, value)),
        }
    }
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
    fn set_frame_value(&self, props: &[(i64, Value)]) {
        let property = self.value_property.get();
        let value = props.iter().find(|(id, _)| *id == property).map(|(_, value)| value.clone());
        *self.frame_value.borrow_mut() = if property == 0 { None } else { value };
    }

    /// Whether the value property of `sender` still has the value the last frame gave it.
    fn shows_frame_value(&self, sender: FerroHandle) -> bool {
        let property = self.value_property.get();
        match &*self.frame_value.borrow() {
            None => false,
            Some(Value::Text(text)) => values::peek_text(sender, property).as_deref() == Some(text.as_str()),
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
    props: Vec<(i64, Value)>,
    bindings: Vec<Binding>,
    children: Vec<Node>,
}

/// The dispatcher of the events bound in frames: one per process, set from any thread (before
/// the application runs, so that a language passes it as the named function it is).
static DISPATCHER: Mutex<FerroEventFn> = Mutex::new(None);

fn dispatcher() -> FerroEventFn {
    *DISPATCHER.lock().unwrap_or_else(PoisonError::into_inner)
}

ferroui_base::ferro_thread_local! {
    /// The retained trees, by root.
    static RETAINED: RefCell<HashMap<FerroHandle, Vec<Node>>> = RefCell::new(HashMap::new());
    /// Whether a commit is writing values: events raised meanwhile are not dispatched.
    static COMMITTING: Cell<bool> = const { Cell::new(false) };
    /// How many dispatches run: nodes removed inside one are freed once the outermost returns.
    static DISPATCHING: Cell<u32> = const { Cell::new(0) };
    /// Nodes removed while a dispatch runs (their handler may be the one running).
    static REMOVED: RefCell<Vec<Node>> = const { RefCell::new(Vec::new()) };
    /// The value of the event being dispatched (`ferro_frame_event_text`, `_f64`).
    static EVENT_TEXT: RefCell<CString> = RefCell::new(CString::default());
    static EVENT_NUMBER: Cell<f64> = const { Cell::new(0.0) };
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

impl Drop for Committing {
    fn drop(&mut self) {
        COMMITTING.set(self.0);
    }
}

fn commit(root: FerroHandle, bytes: &[u8]) -> Result<()> {
    if COMMITTING.get() {
        return Err(Error::new(FERRO_INVALID_STATE, "a frame is being committed: commit after it, not inside it"));
    }
    if root != 0 {
        handles::get(root)?;
    }
    let frame = decode(bytes)?;
    let _committing = Committing(COMMITTING.replace(true));
    let mut failures = Failures(None);
    let old = RETAINED.with_borrow_mut(|retained| retained.remove(&root));
    let first = old.is_none();
    if first && root != 0 {
        // The handle of the root stays valid for the next commits.
        failures.check(ferro_retain(root));
    }
    let nodes = reconcile_children(root, old.unwrap_or_default(), frame, &mut failures);
    RETAINED.with_borrow_mut(|retained| retained.insert(root, nodes));
    match failures.0 {
        None => Ok(()),
        Some(error) => Err(error),
    }
}

/// The nodes of `new` under `parent`, reusing the nodes of `old`.
fn reconcile_children(parent: FerroHandle, old: Vec<Node>, new: Vec<Element>, failures: &mut Failures) -> Vec<Node> {
    let before: Vec<FerroHandle> = old.iter().map(|node| node.handle).collect();
    let mut keyed: HashMap<Key, Node> = HashMap::new();
    let mut unkeyed: VecDeque<Node> = VecDeque::new();
    for node in old {
        if node.key == Key::None {
            unkeyed.push_back(node);
        } else {
            keyed.insert(node.key.clone(), node);
        }
    }
    let mut nodes = Vec::with_capacity(new.len());
    let mut removed = Vec::new();
    for element in new {
        let candidate = if element.key == Key::None { unkeyed.pop_front() } else { keyed.remove(&element.key) };
        let node = match candidate {
            Some(node) if node.type_id == element.type_id => Some(update(node, element, failures)),
            other => {
                removed.extend(other);
                create(element, failures)
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
            failures.check(crate::ferro_show(handle));
        }
        for &handle in before.iter().filter(|handle| !after.contains(handle)) {
            let _ = crate::ferro_close(handle);
        }
    } else if before != after {
        let common = before.iter().zip(&after).take_while(|(a, b)| a == b).count();
        for &handle in &before[common..] {
            let code = ferro_remove_child(parent, handle);
            if code != crate::FERRO_NOT_FOUND {
                failures.check(code);
            }
        }
        for &handle in &after[common..] {
            failures.check(ferro_add_child(parent, handle));
        }
    }
    for node in removed {
        free(node);
    }
    nodes
}

fn write_value(handle: FerroHandle, property: i64, value: &Value) -> FerroStatus {
    match value {
        Value::Text(text) => {
            // What the object shows already is not written again (the text typed into a box).
            if values::peek_text(handle, property).as_deref() == Some(text.as_str()) {
                return FERRO_OK;
            }
            // SAFETY: a pointer and the length of a Rust string.
            unsafe { ferro_set_str(handle, property, text.as_ptr().cast(), text.len() as i64) }
        }
        Value::Number(bits) => ferro_set_f64(handle, property, f64::from_bits(*bits)),
        Value::Integer(value) => ferro_set_i64(handle, property, *value),
        Value::Bool(value) => ferro_set_bool(handle, property, *value),
    }
}

fn create(element: Element, failures: &mut Failures) -> Option<Node> {
    let handle = ferro_create(element.type_id);
    if handle == 0 {
        failures.check(crate::ferro_last_error_code());
        return None;
    }
    for (property, value) in &element.props {
        failures.check(write_value(handle, *property, value));
    }
    let children = reconcile_children(handle, Vec::new(), element.children, failures);
    let mut bindings = Vec::new();
    for (event, tag, value_property) in element.events {
        bindings.extend(bind(handle, event, tag, value_property, failures));
    }
    for binding in &bindings {
        binding.target.set_frame_value(&element.props);
    }
    Some(Node { type_id: element.type_id, key: element.key, handle, props: element.props, bindings, children })
}

fn update(mut node: Node, element: Element, failures: &mut Failures) -> Node {
    let handle = node.handle;
    for (property, value) in &element.props {
        let same = node.props.iter().any(|(id, old)| id == property && old == value);
        if !same {
            failures.check(write_value(handle, *property, value));
        }
    }
    for (property, _) in &node.props {
        if !element.props.iter().any(|(id, _)| id == property) {
            failures.check(ferro_clear(handle, *property));
        }
    }
    node.props = element.props;
    // Events: the bindings of the same events take the new tags; the others come and go.
    let mut bindings = Vec::with_capacity(element.events.len());
    let mut old = std::mem::take(&mut node.bindings);
    for (event, tag, value_property) in element.events {
        match old.iter().position(|binding| binding.event == event) {
            Some(index) => {
                let binding = old.swap_remove(index);
                binding.target.tag.set(tag);
                binding.target.value_property.set(value_property);
                bindings.push(binding);
            }
            None => bindings.extend(bind(handle, event, tag, value_property, failures)),
        }
    }
    for binding in old {
        let _ = ferro_unsubscribe(binding.subscription);
    }
    for binding in &bindings {
        binding.target.set_frame_value(&node.props);
    }
    node.bindings = bindings;
    node.children = reconcile_children(handle, std::mem::take(&mut node.children), element.children, failures);
    node.key = element.key;
    node
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

impl Drop for Dispatching {
    fn drop(&mut self) {
        let depth = DISPATCHING.get() - 1;
        DISPATCHING.set(depth);
        if depth == 0 {
            let removed = REMOVED.with_borrow_mut(std::mem::take);
            for node in removed {
                free(node);
            }
        }
    }
}

unsafe extern "C" fn on_event(user_data: *mut c_void, sender: FerroHandle, _arg: i64) -> i64 {
    if COMMITTING.get() {
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
    let mut arg = 0;
    if value_property != 0 {
        let text = values::peek_text(sender, value_property).unwrap_or_default();
        let number = values::peek_number(sender, value_property).unwrap_or(0.0);
        EVENT_TEXT.with_borrow_mut(|last| *last = CString::new(text.replace('\0', "")).unwrap_or_default());
        EVENT_NUMBER.set(number);
        arg = number as i64;
    } else {
        EVENT_TEXT.with_borrow_mut(|last| *last = CString::default());
        EVENT_NUMBER.set(0.0);
    }
    DISPATCHING.set(DISPATCHING.get() + 1);
    let _dispatching = Dispatching;
    // SAFETY: the dispatcher is a function of the documented signature.
    unsafe { dispatcher(tag, sender, arg) }
}

/// Takes a node out of service: its events unbound and its handles released, children first. A
/// node removed while a dispatch runs waits for the dispatch to end.
fn free(node: Node) {
    if DISPATCHING.get() > 0 {
        REMOVED.with_borrow_mut(|removed| removed.push(node));
        return;
    }
    for binding in node.bindings {
        let _ = ferro_unsubscribe(binding.subscription);
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
    guard(crate::FERRO_PANIC, || {
        *DISPATCHER.lock().unwrap_or_else(PoisonError::into_inner) = function;
        FERRO_OK
    })
}

/// Calls the dispatcher of the thread ([`ferro_frame_dispatcher`]) with `tag`, `sender` and `arg`
/// as an event would, and returns its result: a binding starts its loop with it (the tag `0`
/// asks for the first frame) and a test drives a message without an event. The event text and
/// number are empty. [`FERRO_INVALID_STATE`] (as a negative result) without a dispatcher. UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_dispatch(tag: i64, sender: FerroHandle, arg: i64) -> i64 {
    crate::error::status_or_value(|| {
        thread::check()?;
        let dispatcher = dispatcher()
            .ok_or_else(|| Error::new(FERRO_INVALID_STATE, "no dispatcher is set: call ferro_frame_dispatcher first"))?;
        EVENT_TEXT.with_borrow_mut(|last| *last = CString::default());
        EVENT_NUMBER.set(0.0);
        DISPATCHING.set(DISPATCHING.get() + 1);
        let _dispatching = Dispatching;
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
    EVENT_TEXT.try_with(|text| text.borrow().as_ptr()).unwrap_or(EMPTY.as_ptr() as *const c_char)
}

/// The value property of the event being dispatched as a number (`0` when it is not one). UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_frame_event_f64() -> f64 {
    EVENT_NUMBER.try_with(Cell::get).unwrap_or(0.0)
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
        let found = thread::check().and_then(|()| {
            RETAINED
                .with_borrow(|retained| retained.get(&root).and_then(|nodes| find(nodes, &Key::Int(key))))
                .ok_or_else(|| {
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
        thread::check()?;
        let Some(nodes) = RETAINED.with_borrow_mut(|retained| retained.remove(&root)) else {
            return Err(Error::new(FERRO_NOT_FOUND, "no frame was committed under the root"));
        };
        for node in nodes {
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
