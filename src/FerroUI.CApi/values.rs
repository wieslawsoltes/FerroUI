//! Property values as scalars: a setter and a getter for text, numbers, integers (enumerations
//! as their numeric values), booleans and objects, which cover every property without a function
//! per control.
//!
//! A write to a property whose value type is the scalar itself (a `f64` `FontSize`, a
//! `Option<String>` `Text`, a `bool` `IsEnabled`) goes straight to the typed setter of the
//! property: no value is boxed, and nothing is allocated beyond what the property stores (the
//! `String` of a text). Any other value type takes the untyped route: the scalar is boxed and
//! converted with the conversions of markup (`ValueTypes::try_convert`: numbers, text parsed into
//! a brush or a thickness, a handle cast to the class of the property), and an integer or a name
//! of an enumeration member becomes the member.

use crate::error::{status, Error, Result};
use crate::handles;
use crate::ids::{self, Property};
use crate::strings;
use crate::thread;
use crate::{
    FerroHandle, FerroStatus, FERRO_BUFFER_TOO_SMALL, FERRO_INVALID_ARGUMENT, FERRO_INVALID_VALUE, FERRO_NULL_POINTER,
    FERRO_READ_ONLY, FERRO_TYPE_MISMATCH,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::{MarkupProperty, MarkupType, MarkupValue};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, PropertyValue, Ref};
use ferroui_controls::Control;
use std::any::TypeId;
use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::rc::Rc;

/// The object of `object` and what `property` is on its class: the start of every value call.
#[inline]
fn target(object: FerroHandle, property: i64) -> Result<(Ref<FerroObject>, Property)> {
    thread::check()?;
    let object = handles::get(object)?;
    let property = ids::property(object.get_type(), property)?;
    Ok((object, property))
}

#[cold]
fn mismatch(what: &str, property: &FerroProperty) -> Error {
    Error::new(
        FERRO_TYPE_MISMATCH,
        format!("{what} cannot be given to the property {} of type {}", property.name(), property.property_type_name()),
    )
}

#[cold]
fn plain_mismatch(what: &str, property: &MarkupProperty) -> Error {
    Error::new(FERRO_TYPE_MISMATCH, format!("{what} cannot be given to the property {}", property.name))
}

fn writable(property: &FerroProperty) -> Result<()> {
    if property.is_read_only() {
        return Err(Error::new(FERRO_READ_ONLY, format!("the property {} is read-only", property.name())));
    }
    Ok(())
}

/// Writes `value` to a property whose value type is `T`, through its typed setter.
#[inline]
fn set_typed<T: PropertyValue>(object: &FerroObject, property: &'static FerroProperty, value: T) -> Result<()> {
    if !property.is_valid_value(&value) {
        return Err(invalid(property));
    }
    object.try_set_value_typed(property, value).map_err(|_| mismatch("the value", property))
}

#[cold]
fn invalid(property: &FerroProperty) -> Error {
    Error::new(FERRO_INVALID_VALUE, format!("the property {} does not accept the value", property.name()))
}

/// The value type of a registered property.
fn value_type(property: &FerroProperty) -> ValueType {
    ValueType::new(property.property_type(), property.property_type_name())
}

/// Converts an untyped value to exactly `target`; `None` if no conversion applies.
fn convert(value: Option<&BoxedValue>, target: ValueType) -> Option<BoxedValue> {
    ValueTypes::try_convert(value, target).flatten()
}

/// Writes an untyped value to a registered property, converted to its value type.
fn set_untyped(object: &FerroObject, property: &'static FerroProperty, value: Option<BoxedValue>, what: &str) -> Result<()> {
    let exact = convert(value.as_ref(), value_type(property)).ok_or_else(|| mismatch(what, property))?;
    if !property.is_valid_value(exact.as_any()) {
        return Err(invalid(property));
    }
    object.set_value_untyped(property, exact.as_any(), BindingPriority::LocalValue);
    Ok(())
}

/// The object as the first argument of a member of its metadata.
fn instance(object: &Ref<FerroObject>) -> MarkupValue {
    Some(Rc::new(object.clone()))
}

/// Writes an untyped value to a plain property of the metadata.
fn set_plain(object: &Ref<FerroObject>, property: &'static MarkupProperty, value: Option<BoxedValue>, what: &str) -> Result<()> {
    let set = property
        .set
        .ok_or_else(|| Error::new(FERRO_READ_ONLY, format!("the property {} is read-only", property.name)))?;
    let target = (property.type_)();
    let exact = match &value {
        Some(_) => Some(convert(value.as_ref(), target).ok_or_else(|| plain_mismatch(what, property))?),
        None => None,
    };
    set(&[instance(object), exact]).map(|_| ()).map_err(|error| Error::new(FERRO_INVALID_VALUE, error.to_string()))
}

/// The metadata of the value type of a property, or of the type its nullable form wraps.
fn value_markup(type_id: TypeId) -> Option<&'static MarkupType> {
    MarkupType::find_by_handle(type_id).or_else(|| MarkupType::find_by_nullable_handle(type_id))
}

/// Writes text to `property` of `object`: the text itself to a text property; the member of that
/// name (in any case) to an enumeration; a value parsed from the text otherwise (a brush from
/// `"Red"`, a thickness from `"4,2"`), or the text as the content of a content property. `length`
/// is the length in bytes, a negative length means NUL-terminated. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_set_str(object: FerroHandle, property: i64, text: *const c_char, length: i64) -> FerroStatus {
    status(|| {
        // SAFETY: the caller passes a string as `ferro_set_str` documents.
        let text = unsafe { strings::borrowed(text, length) }?;
        match target(object, property)? {
            (object, Property::Registered(property)) => {
                writable(property)?;
                let type_id = property.property_type();
                if type_id == TypeId::of::<Option<String>>() {
                    return set_typed(&object, property, Some(text.to_owned()));
                }
                if type_id == TypeId::of::<String>() {
                    return set_typed(&object, property, text.to_owned());
                }
                set_untyped(&object, property, Some(text_value(type_id, text)?), "the text")
            }
            (object, Property::Plain(property)) => {
                let value = text_value((property.type_)().id(), text)?;
                set_plain(&object, property, Some(value), "the text")
            }
        }
    })
}

/// The untyped value text stands for in a property of the value type `type_id`: the member of an
/// enumeration, the value its type parses from text, or the text.
fn text_value(type_id: TypeId, text: &str) -> Result<BoxedValue> {
    if let Some(markup) = value_markup(type_id) {
        if let Some(member) = markup.find_enum_member(text.trim(), true) {
            if let Some(value) = member.get(markup) {
                return Ok(value);
            }
        }
        if let Some(parse) = markup.parse {
            if let Ok(Some(value)) = parse(&[Some(Rc::new(text.to_owned()))]) {
                return Ok(value);
            }
        }
    }
    Ok(Rc::new(text.to_owned()))
}

/// Writes a number to `property` of `object`: straight to a `double` property, converted to any
/// other numeric type. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_set_f64(object: FerroHandle, property: i64, value: f64) -> FerroStatus {
    status(|| match target(object, property)? {
        (object, Property::Registered(property)) => {
            writable(property)?;
            let type_id = property.property_type();
            if type_id == TypeId::of::<f64>() {
                return set_typed(&object, property, value);
            }
            if type_id == TypeId::of::<f32>() {
                return set_typed(&object, property, value as f32);
            }
            set_untyped(&object, property, Some(Rc::new(value)), "a number")
        }
        (object, Property::Plain(property)) => set_plain(&object, property, Some(Rc::new(value)), "a number"),
    })
}

/// Writes an integer to `property` of `object`: straight to an `int32_t` or `int64_t` property,
/// the member with that numeric value to an enumeration (or the combination of bits to a set of
/// flags), converted to any other numeric type. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_set_i64(object: FerroHandle, property: i64, value: i64) -> FerroStatus {
    status(|| match target(object, property)? {
        (object, Property::Registered(property)) => {
            writable(property)?;
            let type_id = property.property_type();
            if type_id == TypeId::of::<i32>() {
                let value = i32::try_from(value).map_err(|_| invalid(property))?;
                return set_typed(&object, property, value);
            }
            if type_id == TypeId::of::<i64>() {
                return set_typed(&object, property, value);
            }
            if type_id == TypeId::of::<f64>() {
                return set_typed(&object, property, value as f64);
            }
            set_untyped(&object, property, Some(integer_value(type_id, value)), "an integer")
        }
        (object, Property::Plain(property)) => {
            let value = integer_value((property.type_)().id(), value);
            set_plain(&object, property, Some(value), "an integer")
        }
    })
}

/// The untyped value an integer stands for in a property of the value type `type_id`: the member
/// of an enumeration, or the integer.
fn integer_value(type_id: TypeId, value: i64) -> BoxedValue {
    value_markup(type_id)
        .and_then(|markup| markup.enum_from_value)
        .and_then(|from_value| from_value(value))
        .unwrap_or_else(|| Rc::new(value))
}

/// Writes a boolean to `property` of `object`. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_set_bool(object: FerroHandle, property: i64, value: bool) -> FerroStatus {
    status(|| match target(object, property)? {
        (object, Property::Registered(property)) => {
            writable(property)?;
            let type_id = property.property_type();
            if type_id == TypeId::of::<bool>() {
                return set_typed(&object, property, value);
            }
            if type_id == TypeId::of::<Option<bool>>() {
                return set_typed(&object, property, Some(value));
            }
            set_untyped(&object, property, Some(Rc::new(value)), "a boolean")
        }
        (object, Property::Plain(property)) => set_plain(&object, property, Some(Rc::new(value)), "a boolean"),
    })
}

/// The untyped value of the object of a handle: a control as controls are held in a content (a
/// `Ref<Control>`), any other object as its root handle.
fn object_value(object: Ref<FerroObject>) -> BoxedValue {
    match object.cast::<Control>() {
        Some(control) => Control::boxed(control),
        None => Rc::new(object),
    }
}

/// Writes an object to `property` of `object` (a `Content`, a `Child`, a brush object): `0`
/// writes null. The property holds its own reference: the handle can be released afterwards.
/// UI thread only.
#[no_mangle]
pub extern "C" fn ferro_set_handle(object: FerroHandle, property: i64, value: FerroHandle) -> FerroStatus {
    status(|| {
        let (object, property) = target(object, property)?;
        let value = match value {
            0 => None,
            value => Some(object_value(handles::get(value)?)),
        };
        match property {
            Property::Registered(property) => {
                writable(property)?;
                set_untyped(&object, property, value, "the object")
            }
            Property::Plain(property) => set_plain(&object, property, value, "the object"),
        }
    })
}

/// Clears the local value of `property` of `object`, which then has its styled, inherited or
/// default value again. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_clear(object: FerroHandle, property: i64) -> FerroStatus {
    status(|| match target(object, property)? {
        (object, Property::Registered(property)) => {
            writable(property)?;
            object.clear_value_untyped(property);
            Ok(())
        }
        (object, Property::Plain(property)) => set_plain(&object, property, None, "null"),
    })
}

// ---- getters ---------------------------------------------------------------------------------

/// Reads a property whose value type is `T` through its typed getter, `None` for another type.
#[inline]
fn get_typed<T: PropertyValue>(object: &FerroObject, property: &'static FerroProperty) -> Option<T> {
    if let Some(styled) = property.as_styled::<T>() {
        Some(object.get_value(styled))
    } else {
        property.as_direct::<T>().map(|direct| object.get_direct_value(direct))
    }
}

/// The value of a property in its untyped form (null, or the contents of a nullable).
fn get_untyped(object: &Ref<FerroObject>, property: Property) -> Result<Option<BoxedValue>> {
    match property {
        Property::Registered(property) => Ok(ValueTypes::normalize(object.get_value_untyped(property))),
        Property::Plain(property) => {
            let get = property
                .get
                .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, format!("the property {} cannot be read", property.name)))?;
            let value = get(&[instance(object)]).map_err(|error| Error::new(FERRO_INVALID_VALUE, error.to_string()))?;
            Ok(value.and_then(ValueTypes::normalize))
        }
    }
}

#[cold]
fn read_mismatch(what: &str) -> Error {
    Error::new(FERRO_TYPE_MISMATCH, format!("the value of the property is not {what}"))
}

fn out<T>(pointer: *mut T) -> Result<*mut T> {
    if pointer.is_null() {
        return Err(Error::new(FERRO_NULL_POINTER, "the pointer for the result is NULL"));
    }
    Ok(pointer)
}

fn number_of(value: &BoxedValue) -> Option<f64> {
    if let Some(value) = value.downcast_ref::<f64>() {
        return Some(*value);
    }
    if let Some(value) = value.downcast_ref::<f32>() {
        return Some(f64::from(*value));
    }
    integer_of(value).map(|value| value as f64)
}

fn integer_of(value: &BoxedValue) -> Option<i64> {
    if let Some(value) = value.downcast_ref::<i32>() {
        return Some(i64::from(*value));
    }
    if let Some(value) = value.downcast_ref::<i64>() {
        return Some(*value);
    }
    if let Some(value) = value.downcast_ref::<u32>() {
        return Some(i64::from(*value));
    }
    if let Some(value) = value.downcast_ref::<u8>() {
        return Some(i64::from(*value));
    }
    let markup = MarkupType::find_by_handle(value.value_type_id())?;
    (markup.enum_to_value?)(&**value)
}

/// Reads a number from `property` of `object` into `*result`: a `double` property directly, any
/// other numeric property converted. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_get_f64(object: FerroHandle, property: i64, result: *mut f64) -> FerroStatus {
    status(|| {
        let result = out(result)?;
        let (object, property) = target(object, property)?;
        let value = match property {
            Property::Registered(registered) if registered.property_type() == TypeId::of::<f64>() => {
                get_typed::<f64>(&object, registered).ok_or_else(|| read_mismatch("a number"))?
            }
            property => get_untyped(&object, property)?.as_ref().and_then(number_of).ok_or_else(|| read_mismatch("a number"))?,
        };
        // SAFETY: checked for NULL; the caller passes a pointer to a `double`.
        unsafe { result.write(value) };
        Ok(())
    })
}

/// Reads an integer from `property` of `object` into `*result`: an integer property, or the
/// numeric value of the member of an enumeration. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_get_i64(object: FerroHandle, property: i64, result: *mut i64) -> FerroStatus {
    status(|| {
        let result = out(result)?;
        let (object, property) = target(object, property)?;
        let value = match property {
            Property::Registered(registered) if registered.property_type() == TypeId::of::<i32>() => {
                i64::from(get_typed::<i32>(&object, registered).ok_or_else(|| read_mismatch("an integer"))?)
            }
            property => get_untyped(&object, property)?.as_ref().and_then(integer_of).ok_or_else(|| read_mismatch("an integer"))?,
        };
        // SAFETY: checked for NULL; the caller passes a pointer to an `int64_t`.
        unsafe { result.write(value) };
        Ok(())
    })
}

/// Reads a boolean from `property` of `object` into `*result`. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_get_bool(object: FerroHandle, property: i64, result: *mut bool) -> FerroStatus {
    status(|| {
        let result = out(result)?;
        let (object, property) = target(object, property)?;
        let value = match property {
            Property::Registered(registered) if registered.property_type() == TypeId::of::<bool>() => {
                get_typed::<bool>(&object, registered).ok_or_else(|| read_mismatch("a boolean"))?
            }
            property => get_untyped(&object, property)?
                .and_then(|value| value.downcast_ref::<bool>().copied())
                .ok_or_else(|| read_mismatch("a boolean"))?,
        };
        // SAFETY: checked for NULL; the caller passes a pointer to a `bool`.
        unsafe { result.write(value) };
        Ok(())
    })
}

/// The text of a property: a text property itself (null is the empty text), the name of the
/// member of an enumeration, or text in a content.
fn text_of(object: &Ref<FerroObject>, property: Property) -> Result<String> {
    if let Property::Registered(registered) = property {
        let type_id = registered.property_type();
        if type_id == TypeId::of::<Option<String>>() {
            return Ok(get_typed::<Option<String>>(object, registered).flatten().unwrap_or_default());
        }
        if type_id == TypeId::of::<String>() {
            return get_typed::<String>(object, registered).ok_or_else(|| read_mismatch("text"));
        }
    }
    let Some(value) = get_untyped(object, property)? else {
        return Ok(String::new());
    };
    if let Some(text) = value.downcast_ref::<String>() {
        return Ok(text.clone());
    }
    if let Some(member) = MarkupType::find_by_handle(value.value_type_id()).and_then(|markup| markup.find_enum_member_of(&*value)) {
        return Ok(member.name.to_owned());
    }
    Err(read_mismatch("text"))
}

/// Reads the text of `property` of `object` (see [`ferro_set_str`]) as UTF-8 into `buffer`, with
/// a NUL at the end, and its length in bytes (without the NUL) into `*length` when `length` is not
/// `NULL`. When the text and its NUL do not fit in `capacity` bytes nothing is written to `buffer`
/// and [`FERRO_BUFFER_TOO_SMALL`] is returned, with the length written: a caller can ask for the
/// length first with a `NULL` buffer and a capacity of `0`. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_get_str(
    object: FerroHandle,
    property: i64,
    buffer: *mut c_char,
    capacity: i64,
    length: *mut i64,
) -> FerroStatus {
    status(|| {
        let capacity = usize::try_from(capacity).map_err(|_| Error::new(FERRO_INVALID_ARGUMENT, "the capacity is negative"))?;
        if buffer.is_null() && capacity > 0 {
            return Err(Error::new(FERRO_NULL_POINTER, "the buffer is NULL"));
        }
        let (object, property) = target(object, property)?;
        let text = text_of(&object, property)?;
        if !length.is_null() {
            // SAFETY: the caller passes NULL or a pointer to an `int64_t`.
            unsafe { length.write(text.len() as i64) };
        }
        if text.len() + 1 > capacity {
            return Err(Error::new(
                FERRO_BUFFER_TOO_SMALL,
                format!("the text needs {} bytes with its NUL; the buffer has {capacity}", text.len() + 1),
            ));
        }
        // SAFETY: the buffer has `capacity` bytes, and the text and its NUL fit in them.
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), buffer.cast::<u8>(), text.len());
            buffer.add(text.len()).write(0);
        }
        Ok(())
    })
}

ferroui_base::ferro_thread_local! {
    /// The text the last `ferro_get_cstr` of the thread returned.
    static LAST_TEXT: RefCell<CString> = RefCell::new(CString::default());
}

/// Reads the text of `property` of `object` (see [`ferro_set_str`]) and returns it as a
/// NUL-terminated UTF-8 string that the library owns: it stays valid until the next call of
/// `ferro_get_cstr` on the thread, and the caller copies it to keep it. For a language whose
/// strings are C strings and that cannot pass a buffer. `NULL` on a failure (a text with a NUL
/// inside included: [`FERRO_INVALID_VALUE`]). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_get_cstr(object: FerroHandle, property: i64) -> *const c_char {
    let mut pointer = std::ptr::null();
    let code = status(|| {
        let (object, property) = target(object, property)?;
        let text = CString::new(text_of(&object, property)?)
            .map_err(|_| Error::new(FERRO_INVALID_VALUE, "the text has a NUL inside"))?;
        pointer = LAST_TEXT.with_borrow_mut(|last| {
            *last = text;
            last.as_ptr()
        });
        Ok(())
    });
    if code == crate::FERRO_OK {
        pointer
    } else {
        std::ptr::null()
    }
}

/// Reads the object held by `property` of `object` into `*result`: its handle with one more
/// reference counted, which the caller releases ([`ferro_release`](crate::ferro_release)), or `0`
/// for null. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_get_handle(object: FerroHandle, property: i64, result: *mut FerroHandle) -> FerroStatus {
    status(|| {
        let result = out(result)?;
        let (object, property) = target(object, property)?;
        let handle = match get_untyped(&object, property)? {
            None => 0,
            Some(value) => {
                let object = ValueTypes::as_object(&*value).ok_or_else(|| read_mismatch("an object"))?;
                handles::insert(object)
            }
        };
        // SAFETY: checked for NULL; the caller passes a pointer to a `FerroHandle`.
        unsafe { result.write(handle) };
        Ok(())
    })
}

// ---- getters that return the value ----------------------------------------------------------
//
// For languages that cannot pass a pointer to a local (Osprey, WebAssembly hosts): the value is
// the result, and a fallback the caller chooses is the result of a failure, which is recorded as
// for every failure (`ferro_last_error`, `ferro_last_error_code`).

/// The result of a getter with a fallback: the value, or `fallback` when the read failed (the
/// failure is recorded).
fn value_or<T: Copy>(fallback: T, read: impl FnOnce(*mut T) -> FerroStatus) -> T {
    let mut value = fallback;
    if read(&mut value) == crate::FERRO_OK {
        value
    } else {
        fallback
    }
}

/// Reads a number from `property` of `object`, as [`ferro_get_f64`] does, and returns it; returns
/// `fallback` when the read fails (the failure is recorded: [`ferro_last_error`]). For a
/// language that cannot pass a pointer to a result. UI thread only.
///
/// [`ferro_last_error`]: crate::ferro_last_error
#[no_mangle]
pub extern "C" fn ferro_get_f64_or(object: FerroHandle, property: i64, fallback: f64) -> f64 {
    // SAFETY: a pointer to a local.
    value_or(fallback, |result| unsafe {
        ferro_get_f64(object, property, result)
    })
}

/// Reads an integer from `property` of `object`, as [`ferro_get_i64`] does, and returns it;
/// returns `fallback` when the read fails (the failure is recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_get_i64_or(object: FerroHandle, property: i64, fallback: i64) -> i64 {
    // SAFETY: a pointer to a local.
    value_or(fallback, |result| unsafe {
        ferro_get_i64(object, property, result)
    })
}

/// Reads a boolean from `property` of `object`, as [`ferro_get_bool`] does, and returns it;
/// returns `fallback` when the read fails (the failure is recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_get_bool_or(object: FerroHandle, property: i64, fallback: bool) -> bool {
    // SAFETY: a pointer to a local.
    value_or(fallback, |result| unsafe {
        ferro_get_bool(object, property, result)
    })
}
