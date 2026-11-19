//! Integer IDs for types, properties and events, resolved once by name.
//!
//! - A **type ID** names a class found in the tables of types (`TypeInfo::find_by_name`, or
//!   `TypeInfo::find` for a name with its namespace).
//! - A **property ID** or an **event ID** is an interned name: it means the property (event) of
//!   that name on whatever object it is used with, so one ID serves every class (`Text` of a
//!   `TextBlock` and of a `TextBox`). A name with an owner (`Grid.Row`) means the property the
//!   owner registers, attached properties included. The property behind an ID is found the first
//!   time the ID is used with a class, and kept for that class: later uses are an index and a
//!   short scan, with no string and no allocation.
//!
//! The lookups work on any thread: the names are interned in a table behind a lock, which only a
//! lookup and the first use of an ID with a class take.

use crate::error::{status_or_value, Error, Result};
use crate::strings;
use crate::{FERRO_INVALID_ARGUMENT, FERRO_NOT_FOUND};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventRegistry};
use ferroui_base::metadata::{MarkupEvent, MarkupProperty};
use ferroui_base::{FerroProperty, FerroPropertyRegistry, TypeInfo};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_char;
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

/// Interned names: an ID is the index of the name plus one.
#[derive(Default)]
struct Names {
    list: Vec<&'static str>,
    map: HashMap<&'static str, u32>,
}

impl Names {
    fn intern(&mut self, name: &str) -> Result<i64> {
        if let Some(&id) = self.map.get(name) {
            return Ok(i64::from(id));
        }
        let id = u32::try_from(self.list.len() + 1).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "too many names"))?;
        // Names are few and live as long as the IDs that stand for them: for the process.
        let name: &'static str = Box::leak(name.to_owned().into_boxed_str());
        self.list.push(name);
        self.map.insert(name, id);
        Ok(i64::from(id))
    }

    fn name(&self, id: i64) -> Option<&'static str> {
        let index = usize::try_from(id).ok()?.checked_sub(1)?;
        self.list.get(index).copied()
    }
}

#[derive(Default)]
struct Types {
    list: Vec<&'static TypeInfo>,
    map: HashMap<usize, u32>,
}

static PROPERTY_NAMES: LazyLock<Mutex<Names>> = LazyLock::new(Mutex::default);
static EVENT_NAMES: LazyLock<Mutex<Names>> = LazyLock::new(Mutex::default);
static TYPES: LazyLock<Mutex<Types>> = LazyLock::new(Mutex::default);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A name argument: UTF-8, not empty.
unsafe fn name_argument<'a>(name: *const c_char, length: i64) -> Result<&'a str> {
    // SAFETY: forwarded from the caller of the exported function.
    let name = unsafe { strings::borrowed(name, length) }?.trim();
    if name.is_empty() {
        return Err(Error::new(FERRO_INVALID_ARGUMENT, "the name is empty"));
    }
    Ok(name)
}

/// The ID of `type_`, interning it.
pub(crate) fn intern_type(type_: &'static TypeInfo) -> Option<i64> {
    let mut types = lock(&TYPES);
    let key = type_ as *const TypeInfo as usize;
    if let Some(&id) = types.map.get(&key) {
        return Some(i64::from(id));
    }
    let id = u32::try_from(types.list.len() + 1).ok()?;
    types.list.push(type_);
    types.map.insert(key, id);
    Some(i64::from(id))
}

/// The class of a type ID.
pub(crate) fn type_info(id: i64) -> Result<&'static TypeInfo> {
    let types = lock(&TYPES);
    usize::try_from(id)
        .ok()
        .and_then(|id| id.checked_sub(1))
        .and_then(|index| types.list.get(index).copied())
        .ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("{id} is not a type ID")))
}

/// Finds a class by its name (`Button`) or by its namespace and name (`FerroUI.Controls.Button`).
pub(crate) fn find_type(name: &str) -> Result<&'static TypeInfo> {
    ferroui_controls::register_types();
    if let Some(found) = TypeInfo::find_by_name(name) {
        return Ok(found);
    }
    if let Some((namespace, short)) = name.rsplit_once('.') {
        if let Some(found) = TypeInfo::find(namespace, short) {
            return Ok(found);
        }
    }
    Err(Error::new(
        FERRO_NOT_FOUND,
        format!("no type is named {name}, or more than one is (give its namespace: FerroUI.Controls.{name})"),
    ))
}

/// The type ID of the class named `name` (`Button`, or `FerroUI.Controls.Button` where two
/// namespaces have a class of the name): a positive ID, or [`FERRO_NOT_FOUND`]. `length` is the
/// length of the name in bytes; a negative length means a NUL-terminated name. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_type_id(name: *const c_char, length: i64) -> i64 {
    status_or_value(|| {
        // SAFETY: the caller passes a string as `ferro_type_id` documents.
        let name = unsafe { name_argument(name, length) }?;
        let type_ = find_type(name)?;
        intern_type(type_).ok_or_else(|| Error::new(FERRO_INVALID_ARGUMENT, "too many types"))
    })
}

/// The property ID of the name `name` (`Text`, or `Grid.Row` for the property a class owns,
/// attached properties included): a positive ID. The ID is the name: it is checked against a
/// class when it is first used with an object of that class ([`FERRO_NOT_FOUND`] then). `length`
/// as for [`ferro_type_id`]. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_property_id(name: *const c_char, length: i64) -> i64 {
    status_or_value(|| {
        // SAFETY: the caller passes a string as `ferro_property_id` documents.
        let name = unsafe { name_argument(name, length) }?;
        lock(&PROPERTY_NAMES).intern(name)
    })
}

/// The event ID of the name `name` (`Click`): a routed event of the class of the object or of a
/// base class, or an event its metadata declares; `Button.Click` is the routed event a class owns,
/// for a handler on an ancestor of the objects that raise it. As for [`ferro_property_id`], the ID is the
/// name, checked when it is first used with a class. Any thread.
#[no_mangle]
pub unsafe extern "C" fn ferro_event_id(name: *const c_char, length: i64) -> i64 {
    status_or_value(|| {
        // SAFETY: the caller passes a string as `ferro_event_id` documents.
        let name = unsafe { name_argument(name, length) }?;
        lock(&EVENT_NAMES).intern(name)
    })
}

/// What a property ID stands for on a class.
#[derive(Clone, Copy)]
pub(crate) enum Property {
    /// A registered property (styled, direct or attached).
    Registered(&'static FerroProperty),
    /// A plain property the metadata of the class declares.
    Plain(&'static MarkupProperty),
}

/// What an event ID stands for on a class.
#[derive(Clone, Copy)]
pub(crate) enum Event {
    Routed(RoutedEvent),
    Plain(&'static MarkupEvent),
}

/// What an ID was found to be on one class, `None` for nothing.
type Resolutions<T> = Vec<Vec<(usize, Option<T>)>>;

ferroui_base::ferro_thread_local! {
    /// By property ID: what it is on each class it was used with.
    static PROPERTIES: RefCell<Resolutions<Property>> = const { RefCell::new(Vec::new()) };
    /// By event ID: what it is on each class it was used with.
    static EVENTS: RefCell<Resolutions<Event>> = const { RefCell::new(Vec::new()) };
}

#[inline]
fn cached<T: Copy>(cache: &RefCell<Resolutions<T>>, id: i64, key: usize) -> Option<Option<T>> {
    let cache = cache.borrow();
    let index = usize::try_from(id).ok()?.checked_sub(1)?;
    cache.get(index)?.iter().find(|(class, _)| *class == key).map(|(_, found)| *found)
}

fn remember<T: Copy>(cache: &RefCell<Resolutions<T>>, id: i64, key: usize, found: Option<T>) {
    if let Some(index) = usize::try_from(id).ok().and_then(|id| id.checked_sub(1)) {
        let mut cache = cache.borrow_mut();
        if cache.len() <= index {
            cache.resize_with(index + 1, Vec::new);
        }
        cache[index].push((key, found));
    }
}

/// The property the ID `id` stands for on the class `type_`. UI thread.
#[inline]
pub(crate) fn property(type_: &'static TypeInfo, id: i64) -> Result<Property> {
    let key = type_ as *const TypeInfo as usize;
    let found = match PROPERTIES.with(|cache| cached(cache, id, key)) {
        Some(found) => found,
        None => {
            let name = property_name(id)?;
            // Found with no borrow of the cache held: finding initialises the class.
            let found = find_property(type_, name);
            PROPERTIES.with(|cache| remember(cache, id, key, found));
            found
        }
    };
    found.ok_or_else(|| not_found_on("property", property_name(id).unwrap_or("?"), type_))
}

fn property_name(id: i64) -> Result<&'static str> {
    lock(&PROPERTY_NAMES).name(id).ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("{id} is not a property ID")))
}

#[cold]
fn not_found_on(what: &str, name: &str, type_: &'static TypeInfo) -> Error {
    Error::new(FERRO_NOT_FOUND, format!("the class {} has no {what} {name}", type_.name()))
}

#[cold]
fn find_property(type_: &'static TypeInfo, name: &str) -> Option<Property> {
    let registry = FerroPropertyRegistry::instance();
    if let Some((owner, short)) = name.rsplit_once('.') {
        let owner = find_type(owner).ok()?;
        return registry
            .find_registered(owner, short)
            .or_else(|| registry.get_registered_attached(owner).iter().copied().find(|p| p.name() == short))
            .map(Property::Registered);
    }
    if let Some(found) = registry.find_registered(type_, name) {
        return Some(Property::Registered(found));
    }
    let mut current = Some(type_);
    while let Some(class) = current {
        if let Some(found) = class.markup().and_then(|markup| markup.find_property(name)) {
            return Some(Property::Plain(found));
        }
        current = class.base_type();
    }
    None
}

/// The event the ID `id` stands for on the class `type_`. UI thread.
pub(crate) fn event(type_: &'static TypeInfo, id: i64) -> Result<Event> {
    let key = type_ as *const TypeInfo as usize;
    let found = match EVENTS.with(|cache| cached(cache, id, key)) {
        Some(found) => found,
        None => {
            let name = event_name(id)?;
            let found = find_event(type_, name);
            EVENTS.with(|cache| remember(cache, id, key, found));
            found
        }
    };
    found.ok_or_else(|| not_found_on("event", event_name(id).unwrap_or("?"), type_))
}

fn event_name(id: i64) -> Result<&'static str> {
    lock(&EVENT_NAMES).name(id).ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("{id} is not an event ID")))
}

#[cold]
fn find_event(type_: &'static TypeInfo, name: &str) -> Option<Event> {
    // `Button.Click`: the event a class owns, for a handler on an ancestor of its raisers.
    if let Some((owner, short)) = name.rsplit_once('.') {
        let owner = find_type(owner).ok()?;
        return match find_event(owner, short)? {
            Event::Routed(event) => Some(Event::Routed(event)),
            Event::Plain(_) => None,
        };
    }
    // The routed events of a class are registered when the class is initialised on the thread.
    type_.ensure_class_init();
    if let Some(found) = RoutedEventRegistry::instance().find_by_name(type_, name) {
        return Some(Event::Routed(found));
    }
    // A routed event is registered the first time its accessor runs: the static field
    // `<Name>Event` of the metadata of the class or of a base class runs it.
    let field = format!("{}Event", name.strip_suffix("Event").unwrap_or(name));
    let mut current = Some(type_);
    while let Some(class) = current {
        if let Some(found) = class.markup().and_then(|markup| markup.find_field(&field)) {
            let value = (found.get)();
            if let Some(event) = ferroui_base::metadata::from_markup_value::<RoutedEvent>(&value) {
                return Some(Event::Routed(event));
            }
            if let Some(found) = RoutedEventRegistry::instance().find_by_name(type_, name) {
                return Some(Event::Routed(found));
            }
        }
        current = class.base_type();
    }
    let mut current = Some(type_);
    while let Some(class) = current {
        if let Some(found) = class.markup().and_then(|markup| markup.find_event(name)) {
            return Some(Event::Plain(found));
        }
        current = class.base_type();
    }
    None
}
