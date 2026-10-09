/*
Copyright (c) 2010, Karl Seguin - http://www.openmymind.net/
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of the <organization> nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Ported to Rust from the C# sources of https://github.com/elaberge/Metsys.Bson
*/

//! The BSON serializer of the protocol.
//!
//! The original finds the properties of a class by reflection. Here a class
//! states them: [`bson_class!`] lists the properties of a struct in the order
//! and under the names the original writes them, and [`bson_enum!`] states
//! that an enumeration is written as its number. From those declarations the
//! serializer and the deserializer work as the original does: a value is
//! handed to them in its untyped form ([`ValueRef`] to write, [`Value`] when
//! read), which is what `object` is in the original, and the declared type of
//! a property ([`Type`]) decides how an element is read.
//!
//! The types of this file are internal to the assembly in the original. They
//! are public here because the declarations of a message class of another
//! crate name them.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Error;
use crate::guid::Guid;

use self::configuration::BsonConfiguration;

/// The element types of BSON, by the byte that precedes an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Types {
    Double = 1,
    String = 2,
    Object = 3,
    Array = 4,
    Binary = 5,
    Undefined = 6,
    ObjectId = 7,
    Boolean = 8,
    DateTime = 9,
    Null = 10,
    Regex = 11,
    Reference = 12,
    Code = 13,
    Symbol = 14,
    ScopedCode = 15,
    Int32 = 16,
    Timestamp = 17,
    Int64 = 18,
}

impl Types {
    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<Types> {
        Some(match value {
            1 => Types::Double,
            2 => Types::String,
            3 => Types::Object,
            4 => Types::Array,
            5 => Types::Binary,
            6 => Types::Undefined,
            7 => Types::ObjectId,
            8 => Types::Boolean,
            9 => Types::DateTime,
            10 => Types::Null,
            11 => Types::Regex,
            12 => Types::Reference,
            13 => Types::Code,
            14 => Types::Symbol,
            15 => Types::ScopedCode,
            16 => Types::Int32,
            17 => Types::Timestamp,
            18 => Types::Int64,
            _ => return None,
        })
    }
}

/// The element type as it was read: `(Types)_reader.ReadByte()` of the
/// original, which may be a value the enumeration does not define.
#[derive(Clone, Copy, PartialEq, Eq)]
struct StoredType(u8);

impl StoredType {
    fn is(self, types: Types) -> bool {
        self.0 as i32 == types as i32
    }

    fn known(self) -> Option<Types> {
        Types::from_value(self.0 as i32)
    }
}

/// `Enum.ToString()`: the name of the member, or the number when there is
/// none.
impl fmt::Display for StoredType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.known() {
            Some(types) => write!(f, "{:?}", types),
            None => write!(f, "{}", self.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Declarations: what reflection tells the original about a type.
// ---------------------------------------------------------------------------

/// The declared type of a property, of the items of a collection or of the
/// root of a document: what the original reads from a `System.Type`, as far
/// as the serializer distinguishes types.
#[derive(Clone, Copy, Debug)]
pub enum Type {
    /// `object`.
    Object,
    /// `string`.
    String,
    /// `int`.
    Int32,
    /// `long`.
    Int64,
    /// `bool`.
    Boolean,
    /// `double`.
    Double,
    /// `float`.
    Single,
    /// `Guid`.
    Guid,
    /// `Regex`.
    Regex,
    /// `DateTime`.
    DateTime,
    /// `byte[]`.
    ByteArray,
    /// `ObjectId`.
    ObjectId,
    /// `ScopedCode`.
    ScopedCode,
    /// An enumeration, with the element type of its underlying type
    /// ([`Types::Int32`] or [`Types::Int64`]).
    Enum(Types),
    /// `Nullable<T>`, and a reference that may be null.
    Nullable(&'static Type),
    /// `T[]`.
    Array(&'static Type),
    /// `List<T>` and `IList<T>`.
    List(&'static Type),
    /// A collection that is not a list (`ICollection<T>`: a set, a queue).
    Collection(&'static Type),
    /// The non-generic `IEnumerable` and `IList`.
    Enumerable,
    /// `IDictionary<string, T>`.
    Dictionary(&'static Type),
    /// A class with declared properties.
    Class(&'static ClassType),
}

impl Type {
    /// `typeof(IEnumerable).IsAssignableFrom(type)` for the types that reach
    /// the test (a string is handled before it).
    fn is_enumerable(&self) -> bool {
        matches!(
            self,
            Type::ByteArray
                | Type::Array(_)
                | Type::List(_)
                | Type::Collection(_)
                | Type::Enumerable
                | Type::Dictionary(_)
        )
    }

    /// `Type.FullName`, for messages.
    fn full_name(&self) -> String {
        match self {
            Type::Class(class) => class.name.to_string(),
            other => format!("{:?}", other),
        }
    }
}

/// A property of a class, as declared: `PropertyInfo` of the original.
#[derive(Debug)]
pub struct PropertyInfo {
    /// The name of the property, which is its name on the wire unless the
    /// configuration gives it an alias.
    pub name: &'static str,
    /// `PropertyInfo.PropertyType`.
    pub property_type: &'static Type,
    /// Whether the property has a `set` accessor. A property without one is
    /// a container the deserializer fills.
    pub can_write: bool,
}

/// A class, as declared: what the original reads from the `System.Type` of
/// an object. [`bson_class!`] writes one.
pub struct ClassType {
    /// `Type.Name`.
    pub name: &'static str,
    /// The identity of the Rust type.
    pub type_id: fn() -> TypeId,
    /// `Activator.CreateInstance(type, true)`.
    pub create_instance: fn() -> Box<dyn BsonObject>,
    /// The properties in the order `Type.GetProperties` returns them: the
    /// ones the class declares, then the ones of its base classes.
    pub properties: &'static [PropertyInfo],
    /// `typeof(IExpando).IsAssignableFrom(type)`.
    pub is_expando: bool,
}

impl ClassType {
    /// The identity of the Rust type: the key of every table over types.
    pub fn id(&self) -> TypeId {
        (self.type_id)()
    }
}

impl fmt::Debug for ClassType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// An object of a class with declared properties: what `object` is for the
/// serializer when it is none of the simple types.
pub trait BsonObject: Any + Send + Sync + fmt::Debug {
    /// `object.GetType()`.
    fn get_type(&self) -> &'static ClassType;
    /// Reads the property with the given index in [`ClassType::properties`].
    fn get_property_value(&self, index: usize) -> ValueRef<'_>;
    /// Assigns the property with the given index. The value is cast to the
    /// type of the property; one of another type is an `InvalidCastException`.
    fn set_property_value(&mut self, index: usize, value: Value) -> Result<(), Error>;
    /// `instance as IExpando`.
    fn as_expando(&mut self) -> Option<&mut dyn IExpando> {
        None
    }
    /// A copy of the object behind a new handle.
    fn clone_object(&self) -> Box<dyn BsonObject>;
    /// Whether the other object is of the same class and has equal
    /// properties.
    fn equals(&self, other: &dyn BsonObject) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any + Send + Sync>;
}

impl dyn BsonObject {
    /// `obj is T`.
    pub fn is<T: Any>(&self) -> bool {
        self.as_any().is::<T>()
    }

    /// `obj as T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

impl Clone for Box<dyn BsonObject> {
    fn clone(&self) -> Self {
        self.clone_object()
    }
}

impl PartialEq for dyn BsonObject {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

/// A class that states its properties: `typeof(T)`.
pub trait BsonClass: BsonObject + Sized {
    const CLASS: &'static ClassType;
}

/// A type that a property, an item or a root can have: its declared type and
/// its conversions to and from the untyped form.
pub trait BsonValue: Sized {
    /// `typeof(T)`.
    const TYPE: Type;
    /// The value as the serializer receives it: boxing, without a copy.
    fn as_value(&self) -> ValueRef<'_>;
    /// The cast of a deserialized value to the type of the property it is
    /// assigned to. The deserializer does not assign null.
    fn from_value(value: Value) -> Result<Self, Error>;
}

/// The `InvalidCastException` of a value that is not of the type of the
/// property it is assigned to.
pub fn invalid_cast(value: &Value, expected: &str) -> Error {
    Error::InvalidCast(format!("Unable to cast object of type '{}' to type '{}'.", value.type_name(), expected))
}

/// States the properties of a class for the serializer: the counterpart of
/// the reflection over the properties of a type.
///
/// ```ignore
/// #[derive(Clone, Debug, Default, PartialEq)]
/// pub struct MeasureViewportMessage { pub width: f64, pub height: f64 }
///
/// bson_class!(MeasureViewportMessage as "MeasureViewportMessage" {
///     "Width" => width: f64,
///     "Height" => height: f64,
/// });
/// ```
///
/// Each entry is the name of the property as the original declares it (it is
/// the name on the wire), the field that holds it (a path for a property of
/// an embedded base class: `base.x`) and the type of the field. The entries
/// are in the order `Type.GetProperties` returns the properties: the ones the
/// class declares, in declaration order, then the ones of its base classes.
/// The class is `Clone + Debug + Default + PartialEq + Send + Sync`.
#[macro_export]
macro_rules! bson_class {
    ($class:ident as $name:literal { $($property:literal => $($field:ident).+ : $ty:ty),* $(,)? }) => {
        impl $crate::metsys_bson::BsonClass for $class {
            const CLASS: &'static $crate::metsys_bson::ClassType = {
                fn type_id() -> ::std::any::TypeId {
                    ::std::any::TypeId::of::<$class>()
                }
                fn create_instance() -> ::std::boxed::Box<dyn $crate::metsys_bson::BsonObject> {
                    ::std::boxed::Box::new(<$class as ::std::default::Default>::default())
                }
                &$crate::metsys_bson::ClassType {
                    name: $name,
                    type_id,
                    create_instance,
                    properties: &[$($crate::metsys_bson::PropertyInfo {
                        name: $property,
                        property_type: &<$ty as $crate::metsys_bson::BsonValue>::TYPE,
                        can_write: true,
                    }),*],
                    is_expando: false,
                }
            };
        }

        impl $crate::metsys_bson::BsonObject for $class {
            fn get_type(&self) -> &'static $crate::metsys_bson::ClassType {
                <$class as $crate::metsys_bson::BsonClass>::CLASS
            }

            #[allow(unused_mut, unused_assignments, unused_variables)]
            fn get_property_value(&self, index: usize) -> $crate::metsys_bson::ValueRef<'_> {
                let mut current = 0usize;
                $(
                    if index == current {
                        return $crate::metsys_bson::BsonValue::as_value(&self.$($field).+);
                    }
                    current += 1;
                )*
                panic!("{} has no property with the index {}", $name, index)
            }

            #[allow(unused_mut, unused_assignments, unused_variables)]
            fn set_property_value(
                &mut self,
                index: usize,
                value: $crate::metsys_bson::Value,
            ) -> ::std::result::Result<(), $crate::Error> {
                let mut current = 0usize;
                $(
                    if index == current {
                        self.$($field).+ = <$ty as $crate::metsys_bson::BsonValue>::from_value(value)?;
                        return ::std::result::Result::Ok(());
                    }
                    current += 1;
                )*
                panic!("{} has no property with the index {}", $name, index)
            }

            fn clone_object(&self) -> ::std::boxed::Box<dyn $crate::metsys_bson::BsonObject> {
                ::std::boxed::Box::new(::std::clone::Clone::clone(self))
            }

            fn equals(&self, other: &dyn $crate::metsys_bson::BsonObject) -> bool {
                match other.as_any().downcast_ref::<$class>() {
                    ::std::option::Option::Some(other) => self == other,
                    ::std::option::Option::None => false,
                }
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }

            fn into_any(
                self: ::std::boxed::Box<Self>,
            ) -> ::std::boxed::Box<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync> {
                self
            }
        }

        impl $crate::metsys_bson::BsonValue for $class {
            const TYPE: $crate::metsys_bson::Type =
                $crate::metsys_bson::Type::Class(<$class as $crate::metsys_bson::BsonClass>::CLASS);

            fn as_value(&self) -> $crate::metsys_bson::ValueRef<'_> {
                $crate::metsys_bson::ValueRef::Object(self)
            }

            fn from_value(value: $crate::metsys_bson::Value) -> ::std::result::Result<Self, $crate::Error> {
                match value {
                    $crate::metsys_bson::Value::Object(object) => {
                        if object.as_any().is::<$class>() {
                            match object.into_any().downcast::<$class>() {
                                ::std::result::Result::Ok(object) => ::std::result::Result::Ok(*object),
                                ::std::result::Result::Err(_) => ::std::result::Result::Err($crate::Error::InvalidCast(
                                    ::std::format!("Unable to cast an object to type '{}'.", $name),
                                )),
                            }
                        } else {
                            ::std::result::Result::Err($crate::metsys_bson::invalid_cast(
                                &$crate::metsys_bson::Value::Object(object),
                                $name,
                            ))
                        }
                    }
                    other => ::std::result::Result::Err($crate::metsys_bson::invalid_cast(&other, $name)),
                }
            }
        }
    };
}

/// States that an enumeration is serialized as its number: the counterpart
/// of `Type.IsEnum` and `Enum.GetUnderlyingType`. The enumeration is
/// `#[repr(i32)]`, `Copy`, and has `fn from_value(i32) -> Option<Self>`.
#[macro_export]
macro_rules! bson_enum {
    ($enum:ty) => {
        impl $crate::metsys_bson::BsonValue for $enum {
            const TYPE: $crate::metsys_bson::Type = $crate::metsys_bson::Type::Enum($crate::metsys_bson::Types::Int32);

            fn as_value(&self) -> $crate::metsys_bson::ValueRef<'_> {
                $crate::metsys_bson::ValueRef::Enum {
                    value: *self as i32 as i64,
                    underlying: $crate::metsys_bson::Types::Int32,
                }
            }

            fn from_value(value: $crate::metsys_bson::Value) -> ::std::result::Result<Self, $crate::Error> {
                match value {
                    // Deviation (DEVIATIONS.md, Remote protocol): `Enum.Parse` of the
                    // original accepts a number no member has; a Rust enumeration
                    // cannot hold one, so it is an error here.
                    $crate::metsys_bson::Value::Enum { value, .. } => {
                        match <i32 as ::std::convert::TryFrom<i64>>::try_from(value).ok().and_then(<$enum>::from_value)
                        {
                            ::std::option::Option::Some(member) => ::std::result::Result::Ok(member),
                            ::std::option::Option::None => ::std::result::Result::Err($crate::Error::Argument(
                                ::std::format!("Requested value '{}' was not found.", value),
                            )),
                        }
                    }
                    other => ::std::result::Result::Err($crate::metsys_bson::invalid_cast(&other, stringify!($enum))),
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Values: what `object` is for the serializer.
// ---------------------------------------------------------------------------

/// `RegexOptions`, as far as the serializer reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RegexOptions(pub i32);

#[allow(non_upper_case_globals)]
impl RegexOptions {
    pub const None: RegexOptions = RegexOptions(0);
    pub const IgnoreCase: RegexOptions = RegexOptions(1);
    pub const Multiline: RegexOptions = RegexOptions(2);
    pub const ExplicitCapture: RegexOptions = RegexOptions(4);
    pub const Compiled: RegexOptions = RegexOptions(8);
    pub const Singleline: RegexOptions = RegexOptions(16);
    pub const IgnorePatternWhitespace: RegexOptions = RegexOptions(32);
    pub const RightToLeft: RegexOptions = RegexOptions(64);
    pub const ECMAScript: RegexOptions = RegexOptions(256);
    pub const CultureInvariant: RegexOptions = RegexOptions(512);

    /// `(options & flag) == flag`.
    pub fn contains(self, flag: RegexOptions) -> bool {
        self.0 & flag.0 == flag.0
    }
}

impl std::ops::BitOr for RegexOptions {
    type Output = RegexOptions;

    fn bitor(self, rhs: RegexOptions) -> RegexOptions {
        RegexOptions(self.0 | rhs.0)
    }
}

/// A regular expression as the serializer sees one: its pattern and its
/// options. The library does not match with it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Regex {
    pattern: String,
    options: RegexOptions,
}

impl Regex {
    pub fn new(pattern: impl Into<String>, options: RegexOptions) -> Regex {
        Regex { pattern: pattern.into(), options }
    }

    /// `Regex.Options`.
    pub fn options(&self) -> RegexOptions {
        self.options
    }
}

/// `Regex.ToString()`: the pattern.
impl fmt::Display for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pattern)
    }
}

/// A point in time in UTC, as the serializer sees a `DateTime`: a count of
/// ticks of 100 nanoseconds from [`Helper::EPOCH`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DateTime {
    ticks: i64,
}

const TICKS_PER_MILLISECOND: i64 = 10_000;

impl DateTime {
    /// The time the given count of ticks after the epoch (before it when
    /// negative).
    pub const fn from_unix_ticks(ticks: i64) -> DateTime {
        DateTime { ticks }
    }

    /// `Helper.Epoch.AddMilliseconds(value)`: an `ArgumentOutOfRangeException`
    /// when the result cannot be represented.
    pub fn from_unix_milliseconds(milliseconds: i64) -> Result<DateTime, Error> {
        milliseconds.checked_mul(TICKS_PER_MILLISECOND).map(DateTime::from_unix_ticks).ok_or_else(|| {
            Error::Argument("The added or subtracted value results in an un-representable DateTime.".to_string())
        })
    }

    /// `DateTime.UtcNow`.
    pub fn utc_now() -> DateTime {
        DateTime::from(SystemTime::now())
    }

    /// The ticks from the epoch.
    pub const fn unix_ticks(&self) -> i64 {
        self.ticks
    }

    /// `(long)value.ToUniversalTime().Subtract(Helper.Epoch).TotalMilliseconds`:
    /// the whole milliseconds from the epoch, truncated towards zero.
    pub const fn unix_milliseconds(&self) -> i64 {
        self.ticks / TICKS_PER_MILLISECOND
    }

    /// `ToString("s", CultureInfo.InvariantCulture)`: `yyyy-MM-ddTHH:mm:ss`.
    pub fn to_sortable_string(&self) -> String {
        let seconds = self.ticks.div_euclid(10_000_000);
        let days = seconds.div_euclid(86_400);
        let second_of_day = seconds.rem_euclid(86_400);
        // The civil date of a day count (the algorithm of Howard Hinnant).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_index = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_index + 2) / 5 + 1;
        let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
        let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            year,
            month,
            day,
            second_of_day / 3_600,
            second_of_day % 3_600 / 60,
            second_of_day % 60
        )
    }
}

impl From<SystemTime> for DateTime {
    fn from(time: SystemTime) -> Self {
        let ticks = match time.duration_since(UNIX_EPOCH) {
            Ok(after) => (after.as_nanos() / 100) as i64,
            Err(before) => -((before.duration().as_nanos() / 100) as i64),
        };
        DateTime { ticks }
    }
}

/// A dictionary with string keys in insertion order: `IDictionary` as the
/// serializer writes one, and `Dictionary<string, object>` as it reads one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dictionary {
    entries: Vec<(String, Value)>,
}

impl Dictionary {
    pub fn new() -> Dictionary {
        Dictionary::default()
    }

    /// `Count`.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `ContainsKey`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// `TryGetValue`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// The indexer's setter: replaces the value of the key or adds the key.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    /// `Add`: a key that is already present is an `ArgumentException`.
    pub fn add(&mut self, key: impl Into<String>, value: Value) -> Result<(), Error> {
        let key = key.into();
        if self.contains_key(&key) {
            return Err(Error::Argument(format!("An item with the same key has already been added. Key: {}", key)));
        }
        self.entries.push((key, value));
        Ok(())
    }

    /// The keys and their values, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// A value in its untyped form, owned: what the deserializer produces.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Int32(i32),
    Int64(i64),
    Boolean(bool),
    String(String),
    Double(f64),
    Single(f32),
    Guid(Guid),
    Regex(Regex),
    DateTime(DateTime),
    ByteArray(Vec<u8>),
    ObjectId(ObjectId),
    ScopedCode(Box<ScopedCode>),
    /// A member of an enumeration, as its number and the element type of the
    /// underlying type.
    Enum {
        value: i64,
        underlying: Types,
    },
    Dictionary(Dictionary),
    /// An array, a list or another collection.
    Enumerable(Vec<Value>),
    Object(Box<dyn BsonObject>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The value by reference, as the serializer receives one.
    pub fn as_ref(&self) -> ValueRef<'_> {
        match self {
            Value::Null => ValueRef::Null,
            Value::Int32(v) => ValueRef::Int32(*v),
            Value::Int64(v) => ValueRef::Int64(*v),
            Value::Boolean(v) => ValueRef::Boolean(*v),
            Value::String(v) => ValueRef::String(v),
            Value::Double(v) => ValueRef::Double(*v),
            Value::Single(v) => ValueRef::Single(*v),
            Value::Guid(v) => ValueRef::Guid(*v),
            Value::Regex(v) => ValueRef::Regex(v),
            Value::DateTime(v) => ValueRef::DateTime(*v),
            Value::ByteArray(v) => ValueRef::ByteArray(v),
            Value::ObjectId(v) => ValueRef::ObjectId(v),
            Value::ScopedCode(v) => ValueRef::ScopedCode(v),
            Value::Enum { value, underlying } => ValueRef::Enum { value: *value, underlying: *underlying },
            Value::Dictionary(v) => ValueRef::Dictionary(v),
            Value::Enumerable(v) => ValueRef::Enumerable(v.iter().map(Value::as_ref).collect()),
            Value::Object(v) => ValueRef::Object(&**v),
        }
    }

    /// `value.GetType().Name`, for messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Int32(_) => "Int32",
            Value::Int64(_) => "Int64",
            Value::Boolean(_) => "Boolean",
            Value::String(_) => "String",
            Value::Double(_) => "Double",
            Value::Single(_) => "Single",
            Value::Guid(_) => "Guid",
            Value::Regex(_) => "Regex",
            Value::DateTime(_) => "DateTime",
            Value::ByteArray(_) => "Byte[]",
            Value::ObjectId(_) => "ObjectId",
            Value::ScopedCode(_) => "ScopedCode",
            Value::Enum { .. } => "Enum",
            Value::Dictionary(_) => "Dictionary`2",
            Value::Enumerable(_) => "List`1",
            Value::Object(object) => object.get_type().name,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Int32(a), Value::Int32(b)) => a == b,
            (Value::Int64(a), Value::Int64(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Double(a), Value::Double(b)) => a == b,
            (Value::Single(a), Value::Single(b)) => a == b,
            (Value::Guid(a), Value::Guid(b)) => a == b,
            (Value::Regex(a), Value::Regex(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::ByteArray(a), Value::ByteArray(b)) => a == b,
            (Value::ObjectId(a), Value::ObjectId(b)) => a == b,
            (Value::ScopedCode(a), Value::ScopedCode(b)) => a == b,
            (Value::Enum { value: a, underlying: x }, Value::Enum { value: b, underlying: y }) => a == b && x == y,
            (Value::Dictionary(a), Value::Dictionary(b)) => a == b,
            (Value::Enumerable(a), Value::Enumerable(b)) => a == b,
            (Value::Object(a), Value::Object(b)) => a.equals(&**b),
            _ => false,
        }
    }
}

/// A value in its untyped form, borrowed: what the serializer is given, so
/// that writing an object copies none of its strings, arrays or objects.
#[derive(Clone, Debug)]
pub enum ValueRef<'a> {
    Null,
    Int32(i32),
    Int64(i64),
    Boolean(bool),
    String(&'a str),
    Double(f64),
    Single(f32),
    Guid(Guid),
    Regex(&'a Regex),
    DateTime(DateTime),
    ByteArray(&'a [u8]),
    ObjectId(&'a ObjectId),
    ScopedCode(&'a ScopedCode),
    Enum { value: i64, underlying: Types },
    Dictionary(&'a Dictionary),
    Enumerable(Vec<ValueRef<'a>>),
    Object(&'a dyn BsonObject),
}

impl ValueRef<'_> {
    pub fn is_null(&self) -> bool {
        matches!(self, ValueRef::Null)
    }

    /// A copy of the value that owns what it refers to.
    pub fn to_value(&self) -> Value {
        match self {
            ValueRef::Null => Value::Null,
            ValueRef::Int32(v) => Value::Int32(*v),
            ValueRef::Int64(v) => Value::Int64(*v),
            ValueRef::Boolean(v) => Value::Boolean(*v),
            ValueRef::String(v) => Value::String((*v).to_string()),
            ValueRef::Double(v) => Value::Double(*v),
            ValueRef::Single(v) => Value::Single(*v),
            ValueRef::Guid(v) => Value::Guid(*v),
            ValueRef::Regex(v) => Value::Regex((*v).clone()),
            ValueRef::DateTime(v) => Value::DateTime(*v),
            ValueRef::ByteArray(v) => Value::ByteArray(v.to_vec()),
            ValueRef::ObjectId(v) => Value::ObjectId((*v).clone()),
            ValueRef::ScopedCode(v) => Value::ScopedCode(Box::new((*v).clone())),
            ValueRef::Enum { value, underlying } => Value::Enum { value: *value, underlying: *underlying },
            ValueRef::Dictionary(v) => Value::Dictionary((*v).clone()),
            ValueRef::Enumerable(v) => Value::Enumerable(v.iter().map(ValueRef::to_value).collect()),
            ValueRef::Object(v) => Value::Object(v.clone_object()),
        }
    }
}

macro_rules! bson_simple_value {
    ($ty:ty, $type:ident, $name:literal, |$this:ident| $as_value:expr, $variant:ident) => {
        impl BsonValue for $ty {
            const TYPE: Type = Type::$type;

            fn as_value(&self) -> ValueRef<'_> {
                let $this = self;
                $as_value
            }

            fn from_value(value: Value) -> Result<Self, Error> {
                match value {
                    Value::$variant(value) => Ok(value),
                    other => Err(invalid_cast(&other, $name)),
                }
            }
        }
    };
}

bson_simple_value!(i32, Int32, "Int32", |v| ValueRef::Int32(*v), Int32);
bson_simple_value!(i64, Int64, "Int64", |v| ValueRef::Int64(*v), Int64);
bson_simple_value!(bool, Boolean, "Boolean", |v| ValueRef::Boolean(*v), Boolean);
bson_simple_value!(String, String, "String", |v| ValueRef::String(v), String);
bson_simple_value!(f64, Double, "Double", |v| ValueRef::Double(*v), Double);
bson_simple_value!(f32, Single, "Single", |v| ValueRef::Single(*v), Single);
bson_simple_value!(Guid, Guid, "Guid", |v| ValueRef::Guid(*v), Guid);
bson_simple_value!(Regex, Regex, "Regex", |v| ValueRef::Regex(v), Regex);
bson_simple_value!(DateTime, DateTime, "DateTime", |v| ValueRef::DateTime(*v), DateTime);
bson_simple_value!(Vec<u8>, ByteArray, "Byte[]", |v| ValueRef::ByteArray(v), ByteArray);
bson_simple_value!(ObjectId, ObjectId, "ObjectId", |v| ValueRef::ObjectId(v), ObjectId);

impl BsonValue for ScopedCode {
    const TYPE: Type = Type::ScopedCode;

    fn as_value(&self) -> ValueRef<'_> {
        ValueRef::ScopedCode(self)
    }

    fn from_value(value: Value) -> Result<Self, Error> {
        match value {
            Value::ScopedCode(value) => Ok(*value),
            other => Err(invalid_cast(&other, "ScopedCode")),
        }
    }
}

/// `object`: any value.
impl BsonValue for Value {
    const TYPE: Type = Type::Object;

    fn as_value(&self) -> ValueRef<'_> {
        self.as_ref()
    }

    fn from_value(value: Value) -> Result<Self, Error> {
        Ok(value)
    }
}

/// `IDictionary<string, object>`.
impl BsonValue for Dictionary {
    const TYPE: Type = Type::Dictionary(&Type::Object);

    fn as_value(&self) -> ValueRef<'_> {
        ValueRef::Dictionary(self)
    }

    fn from_value(value: Value) -> Result<Self, Error> {
        match value {
            Value::Dictionary(value) => Ok(value),
            other => Err(invalid_cast(&other, "Dictionary`2")),
        }
    }
}

/// `T[]`. (`byte[]` is `Vec<u8>`, which is written as binary data.)
impl<T: BsonValue> BsonValue for Vec<T> {
    const TYPE: Type = Type::Array(&T::TYPE);

    fn as_value(&self) -> ValueRef<'_> {
        ValueRef::Enumerable(self.iter().map(T::as_value).collect())
    }

    fn from_value(value: Value) -> Result<Self, Error> {
        match value {
            Value::Enumerable(items) => items.into_iter().map(T::from_value).collect(),
            other => Err(invalid_cast(&other, "Array")),
        }
    }
}

/// `Nullable<T>`, and a reference that may be null.
impl<T: BsonValue> BsonValue for Option<T> {
    const TYPE: Type = Type::Nullable(&T::TYPE);

    fn as_value(&self) -> ValueRef<'_> {
        match self {
            Some(value) => value.as_value(),
            None => ValueRef::Null,
        }
    }

    fn from_value(value: Value) -> Result<Self, Error> {
        match value {
            Value::Null => Ok(None),
            value => T::from_value(value).map(Some),
        }
    }
}

// ---------------------------------------------------------------------------
// Serializer
// ---------------------------------------------------------------------------

pub struct Serializer {
    writer: Vec<u8>,
    current: Option<Box<Document>>,
}

impl Serializer {
    /// `_typeMap`: the element type of a simple value. `None` for a value
    /// that "isn't a simple type".
    fn type_map(value: &ValueRef<'_>) -> Option<Types> {
        match value {
            ValueRef::Int32(_) => Some(Types::Int32),
            ValueRef::Int64(_) => Some(Types::Int64),
            ValueRef::Boolean(_) => Some(Types::Boolean),
            ValueRef::String(_) => Some(Types::String),
            ValueRef::Double(_) => Some(Types::Double),
            ValueRef::Guid(_) => Some(Types::Binary),
            ValueRef::Regex(_) => Some(Types::Regex),
            ValueRef::DateTime(_) => Some(Types::DateTime),
            ValueRef::Single(_) => Some(Types::Double),
            ValueRef::ByteArray(_) => Some(Types::Binary),
            ValueRef::ObjectId(_) => Some(Types::ObjectId),
            ValueRef::ScopedCode(_) => Some(Types::ScopedCode),
            // `if (type.IsEnum) type = Enum.GetUnderlyingType(type)`.
            ValueRef::Enum { underlying: Types::Int32, .. } => Some(Types::Int32),
            ValueRef::Enum { underlying: Types::Int64, .. } => Some(Types::Int64),
            _ => None,
        }
    }

    /// `Serialize<T>(T document)`.
    pub fn serialize<T: BsonValue>(document: &T) -> Result<Vec<u8>, Error> {
        Serializer::serialize_object(document.as_value())
    }

    /// `Serialize(object document)`.
    pub fn serialize_object(document: ValueRef<'_>) -> Result<Vec<u8>, Error> {
        match document {
            // `document.GetType()` of null.
            ValueRef::Null => return Err(Error::NullReference),
            ValueRef::Object(_) | ValueRef::Dictionary(_) => {}
            // A value type, or an enumerable that is not a dictionary. A
            // regular expression, an object identifier and scoped code pass
            // the test of the original, which then writes their properties by
            // reflection; the port has no declaration of them as classes and
            // rejects them as it rejects the other values.
            _ => return Err(BsonException::with_message("Root type must be an object").into()),
        }
        let mut serializer = Serializer::new(Vec::with_capacity(250));
        serializer.write_document(&document)?;
        Ok(serializer.writer)
    }

    fn new(writer: Vec<u8>) -> Serializer {
        Serializer { writer, current: None }
    }

    fn new_document(&mut self) {
        let old = self.current.take();
        self.current = Some(Box::new(Document { parent: old, length: self.writer.len() as i32, digested: 4 }));
        self.writer.extend_from_slice(&0i32.to_le_bytes()); // length placeholder
    }

    fn end_document(&mut self, include_eeo: bool) {
        if include_eeo {
            self.written(1);
            self.writer.push(0);
        }

        let old = *self.current.take().expect("end_document without a document");
        let at = old.length as usize;
        self.writer[at..at + 4].copy_from_slice(&old.digested.to_le_bytes()); // override the document length placeholder
        self.current = old.parent;
        if self.current.is_some() {
            self.written(old.digested);
        }
    }

    fn written(&mut self, length: i32) {
        self.current.as_mut().expect("written without a document").digested += length;
    }

    fn write_document(&mut self, document: &ValueRef<'_>) -> Result<(), Error> {
        self.new_document();
        self.write_object(document)?;
        self.end_document(true);
        Ok(())
    }

    fn write_object(&mut self, document: &ValueRef<'_>) -> Result<(), Error> {
        let object = match document {
            ValueRef::Dictionary(dictionary) => return self.write_dictionary(dictionary),
            ValueRef::Object(object) => *object,
            // `document.GetType()` of null.
            ValueRef::Null => return Err(Error::NullReference),
            // A value that is neither: the original reflects over its
            // properties; the simple values have none that it could write.
            _ => return Ok(()),
        };

        let type_helper = TypeHelper::get_helper_for_type(object.get_type());
        for property in type_helper.get_properties() {
            if property.ignored() {
                continue;
            }
            let name = property.name();
            let value = property.getter(object);
            if value.is_null() && property.ignored_if_null() {
                continue;
            }
            self.serialize_member(name, &value)?;
        }
        Ok(())
    }

    fn serialize_member(&mut self, name: &str, value: &ValueRef<'_>) -> Result<(), Error> {
        if value.is_null() {
            self.write_type(Types::Null);
            self.write_name(name);
            return Ok(());
        }

        let storage_type = match Serializer::type_map(value) {
            Some(storage_type) => storage_type,
            None => {
                // this isn't a simple type;
                return self.write_named(name, value);
            }
        };

        self.write_type(storage_type);
        self.write_name(name);
        match value {
            ValueRef::Int32(value) => {
                self.written(4);
                self.writer.extend_from_slice(&value.to_le_bytes());
            }
            ValueRef::Int64(value) => {
                self.written(8);
                self.writer.extend_from_slice(&value.to_le_bytes());
            }
            // `(int)value` and `(long)value` of a boxed member of an enumeration.
            ValueRef::Enum { value, underlying } => {
                if *underlying == Types::Int32 {
                    self.written(4);
                    self.writer.extend_from_slice(&(*value as i32).to_le_bytes());
                } else {
                    self.written(8);
                    self.writer.extend_from_slice(&value.to_le_bytes());
                }
            }
            ValueRef::String(value) => self.write_string(value),
            ValueRef::Single(value) => {
                self.written(8);
                self.writer.extend_from_slice(&(*value as f64).to_le_bytes());
            }
            ValueRef::Double(value) => {
                self.written(8);
                self.writer.extend_from_slice(&value.to_le_bytes());
            }
            ValueRef::Boolean(value) => {
                self.written(1);
                self.writer.push(if *value { 1 } else { 0 });
            }
            ValueRef::DateTime(value) => {
                self.written(8);
                self.writer.extend_from_slice(&value.unix_milliseconds().to_le_bytes());
            }
            ValueRef::ByteArray(_) | ValueRef::Guid(_) => self.write_binary(value),
            ValueRef::ScopedCode(value) => self.write_scoped_code(value)?,
            ValueRef::ObjectId(value) => {
                let bytes = value.value().ok_or(Error::NullReference)?;
                self.written(bytes.len() as i32);
                self.writer.extend_from_slice(bytes);
            }
            ValueRef::Regex(value) => self.write_regex(value),
            ValueRef::Null | ValueRef::Dictionary(_) | ValueRef::Enumerable(_) | ValueRef::Object(_) => {
                unreachable!("not a simple type")
            }
        }
        Ok(())
    }

    /// `Write(string name, object value)`.
    fn write_named(&mut self, name: &str, value: &ValueRef<'_>) -> Result<(), Error> {
        match value {
            ValueRef::Dictionary(dictionary) => {
                self.write_type(Types::Object);
                self.write_name(name);
                self.new_document();
                self.write_dictionary(dictionary)?;
                self.end_document(true);
            }
            ValueRef::Enumerable(enumerable) => {
                self.write_type(Types::Array);
                self.write_name(name);
                self.new_document();
                self.write_enumerable(enumerable)?;
                self.end_document(true);
            }
            _ => {
                self.write_type(Types::Object);
                self.write_name(name);
                self.write_document(value)?; // Write manages new/end document
            }
        }
        Ok(())
    }

    /// `Write(IEnumerable enumerable)`.
    fn write_enumerable(&mut self, enumerable: &[ValueRef<'_>]) -> Result<(), Error> {
        for (index, value) in enumerable.iter().enumerate() {
            self.serialize_member(&index.to_string(), value)?;
        }
        Ok(())
    }

    /// `Write(IDictionary dictionary)`.
    fn write_dictionary(&mut self, dictionary: &Dictionary) -> Result<(), Error> {
        for (key, value) in dictionary.iter() {
            self.serialize_member(key, &value.as_ref())?;
        }
        Ok(())
    }

    fn write_binary(&mut self, value: &ValueRef<'_>) {
        match value {
            ValueRef::ByteArray(bytes) => {
                let length = bytes.len() as i32;
                self.writer.extend_from_slice(&(length + 4).to_le_bytes());
                self.writer.push(2);
                self.writer.extend_from_slice(&length.to_le_bytes());
                self.writer.extend_from_slice(bytes);
                self.written(9 + length);
            }
            ValueRef::Guid(guid) => {
                let bytes = guid.to_byte_array();
                self.writer.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
                self.writer.push(3);
                self.writer.extend_from_slice(&bytes);
                self.written(5 + bytes.len() as i32);
            }
            _ => {}
        }
    }

    /// `Write(Types type)`.
    fn write_type(&mut self, types: Types) {
        self.writer.push(types as i32 as u8);
        self.written(1);
    }

    fn write_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        self.writer.extend_from_slice(bytes);
        self.writer.push(0);
        self.written(bytes.len() as i32 + 1);
    }

    /// `Write(string name)`: a string value.
    fn write_string(&mut self, name: &str) {
        let bytes = name.as_bytes();
        self.writer.extend_from_slice(&(bytes.len() as i32 + 1).to_le_bytes());
        self.writer.extend_from_slice(bytes);
        self.writer.push(0);
        self.written(bytes.len() as i32 + 5); // stringLength + length + null byte
    }

    /// `Write(Regex regex)`.
    fn write_regex(&mut self, regex: &Regex) {
        self.write_name(&regex.to_string());

        let mut options = String::new();
        if regex.options().contains(RegexOptions::ECMAScript) {
            options.push('e');
        }

        if regex.options().contains(RegexOptions::IgnoreCase) {
            options.push('i');
        }

        if regex.options().contains(RegexOptions::CultureInvariant) {
            options.push('l');
        }

        if regex.options().contains(RegexOptions::Multiline) {
            options.push('m');
        }

        if regex.options().contains(RegexOptions::Singleline) {
            options.push('s');
        }

        options.push('u'); // all .net regex are unicode regex, therefore:
        if regex.options().contains(RegexOptions::IgnorePatternWhitespace) {
            options.push('w');
        }

        if regex.options().contains(RegexOptions::ExplicitCapture) {
            options.push('x');
        }

        self.write_name(&options);
    }

    /// `Write(ScopedCode value)`.
    fn write_scoped_code(&mut self, value: &ScopedCode) -> Result<(), Error> {
        self.new_document();
        self.write_string(value.code_string.as_deref().ok_or(Error::NullReference)?);
        self.write_document(&value.scope.as_ref())?;
        self.end_document(false);
        Ok(())
    }
}

/// Code with the scope it runs in. The original has a second, generic class
/// whose scope is typed; a scope here is a [`Value`], which holds an object
/// of any class, so the one struct serves both.
#[derive(Clone, Debug, PartialEq)]
pub struct ScopedCode {
    pub code_string: Option<String>,
    pub scope: Value,
}

impl Default for ScopedCode {
    fn default() -> Self {
        ScopedCode { code_string: None, scope: Value::Null }
    }
}

pub struct ObjectIdGenerator;

static COUNTER: AtomicI32 = AtomicI32::new(0);

impl ObjectIdGenerator {
    pub fn generate() -> Vec<u8> {
        let mut oid = vec![0u8; 12];
        let mut copyidx = 0;

        oid[copyidx..copyidx + 4].copy_from_slice(&ObjectIdGenerator::generate_time().to_le_bytes());
        copyidx += 4;

        oid[copyidx..copyidx + 3].copy_from_slice(&ObjectIdGenerator::machine_hash()[0..3]);
        copyidx += 3;

        oid[copyidx..copyidx + 2].copy_from_slice(&ObjectIdGenerator::generate_proc_id().to_le_bytes()[0..2]);
        copyidx += 2;

        oid[copyidx..copyidx + 3].copy_from_slice(&ObjectIdGenerator::generate_inc().to_le_bytes()[0..3]);
        oid
    }

    /// The milliseconds of the current time of day in UTC: the original
    /// builds a time on the day of the epoch from the hour, minute, second
    /// and millisecond of now.
    fn generate_time() -> i32 {
        let now = DateTime::utc_now();
        let milliseconds = now.unix_ticks().div_euclid(TICKS_PER_MILLISECOND);
        milliseconds.rem_euclid(86_400_000) as i32
    }

    fn generate_inc() -> i32 {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn machine_hash() -> &'static [u8; 8] {
        static MACHINE_HASH: OnceLock<[u8; 8]> = OnceLock::new();
        MACHINE_HASH.get_or_init(ObjectIdGenerator::generate_host_hash)
    }

    // Deviation (DEVIATIONS.md, Remote protocol): the original hashes
    // `Dns.GetHostName()` with MD5. The standard library has neither, so the
    // name comes from the environment and the hash is the one of the standard
    // library. The three bytes only tell machines apart.
    fn generate_host_hash() -> [u8; 8] {
        use std::hash::{Hash, Hasher};

        let host = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")).unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        host.hash(&mut hasher);
        hasher.finish().to_le_bytes()
    }

    fn generate_proc_id() -> i32 {
        std::process::id() as i32
    }
}

#[derive(Clone, Debug, Default)]
pub struct ObjectId {
    value: Option<Vec<u8>>,
}

impl ObjectId {
    /// `new ObjectId()`: an identifier without a value.
    pub fn new() -> ObjectId {
        ObjectId::default()
    }

    /// `new ObjectId(string value)`.
    pub fn from_string(value: &str) -> Result<ObjectId, Error> {
        Ok(ObjectId::from_bytes(ObjectId::decode_hex(value)?))
    }

    /// `new ObjectId(byte[] value)`.
    pub fn from_bytes(value: Vec<u8>) -> ObjectId {
        ObjectId { value: Some(value) }
    }

    pub fn empty() -> ObjectId {
        ObjectId::from_bytes(vec![0; 12])
    }

    pub fn value(&self) -> Option<&[u8]> {
        self.value.as_deref()
    }

    pub fn new_object_id() -> ObjectId {
        // TODO: generate random-ish bits.
        ObjectId { value: Some(ObjectIdGenerator::generate()) }
    }

    /// `TryParse(string value, out ObjectId id)`: `None` where the original
    /// returns false (and hands out the empty identifier).
    pub fn try_parse(value: Option<&str>) -> Option<ObjectId> {
        let value = value?;
        if value.chars().count() != 24 {
            return None;
        }

        match ObjectId::from_string(value) {
            Ok(id) => Some(id),
            Err(Error::Format(_)) => None,
            Err(_) => None,
        }
    }

    /// `DecodeHex`: `Convert.ToByte(pair, 16)` for every two characters.
    pub fn decode_hex(val: &str) -> Result<Vec<u8>, Error> {
        let chars: Vec<char> = val.chars().collect();
        let number_chars = chars.len();
        let mut bytes = vec![0u8; number_chars / 2];

        let mut i = 0;
        while i < number_chars {
            if i + 2 > number_chars {
                // `new string(chars, i, 2)` past the end of the array.
                return Err(Error::Argument(
                    "Index and length must refer to a location within the string.".to_string(),
                ));
            }
            let pair: String = chars[i..i + 2].iter().collect();
            if !pair.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(Error::Format("Could not find any recognizable digits.".to_string()));
            }
            bytes[i / 2] = u8::from_str_radix(&pair, 16)
                .map_err(|_| Error::Format("Could not find any recognizable digits.".to_string()))?;
            i += 2;
        }

        Ok(bytes)
    }

    /// The text of the value, `None` without one: what `ToString()` returns.
    fn to_option_string(&self) -> Option<String> {
        self.value.as_ref().map(|value| value.iter().map(|b| format!("{:02x}", b)).collect())
    }
}

/// `operator ==`, `operator !=` and `Equals`: by the text of the value.
impl PartialEq for ObjectId {
    fn eq(&self, other: &Self) -> bool {
        self.to_option_string() == other.to_option_string()
    }
}

impl Eq for ObjectId {}

/// `GetHashCode`: of the text of the value.
impl std::hash::Hash for ObjectId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_option_string().hash(state);
    }
}

/// `ToString()`: the value in lower-case hexadecimal digits; nothing for an
/// identifier without a value (the original returns null).
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_option_string() {
            Some(text) => f.write_str(&text),
            None => Ok(()),
        }
    }
}

/// `implicit operator string(ObjectId oid)`.
impl From<&ObjectId> for Option<String> {
    fn from(oid: &ObjectId) -> Self {
        oid.to_option_string()
    }
}

/// `implicit operator ObjectId(string oidString)`.
impl TryFrom<&str> for ObjectId {
    type Error = Error;

    fn try_from(oid_string: &str) -> Result<Self, Self::Error> {
        ObjectId::from_string(oid_string)
    }
}

/// A class that keeps the members a document has and the class does not.
pub trait IExpando {
    fn expando(&mut self) -> &mut Dictionary;
}

/// A property of a class as the serializer uses it: under its alias, with
/// what the configuration says about it.
#[derive(Clone, Debug)]
pub struct MagicProperty {
    property: &'static PropertyInfo,
    index: usize,
    name: String,
    ignored: bool,
    ignored_if_null: bool,
}

impl MagicProperty {
    pub fn new(
        property: &'static PropertyInfo,
        index: usize,
        name: String,
        ignored: bool,
        ignored_if_null: bool,
    ) -> Self {
        MagicProperty { property, index, name, ignored, ignored_if_null }
    }

    /// `Type`: the declared type of the property.
    pub fn property_type(&self) -> &'static Type {
        self.property.property_type
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ignored(&self) -> bool {
        self.ignored
    }

    pub fn ignored_if_null(&self) -> bool {
        self.ignored_if_null
    }

    /// `Setter != null`: whether the property has a `set` accessor.
    pub fn has_setter(&self) -> bool {
        self.property.can_write
    }

    /// `Setter(instance, value)`.
    pub fn setter(&self, instance: &mut dyn BsonObject, value: Value) -> Result<(), Error> {
        instance.set_property_value(self.index, value)
    }

    /// `Getter(instance)`.
    pub fn getter<'a>(&self, instance: &'a dyn BsonObject) -> ValueRef<'a> {
        instance.get_property_value(self.index)
    }
}

/// `StringComparer.CurrentCultureIgnoreCase` of the property table. The port
/// compares the lower-case forms of the characters.
fn equals_ignore_case(a: &str, b: &str) -> bool {
    a.chars().flat_map(char::to_lowercase).eq(b.chars().flat_map(char::to_lowercase))
}

pub struct TypeHelper {
    properties: Vec<MagicProperty>,
    expando: Option<MagicProperty>,
}

impl TypeHelper {
    fn new(class: &'static ClassType) -> TypeHelper {
        let properties = TypeHelper::load_magic_properties(class, class.properties);
        let mut expando = None;
        if class.is_expando {
            expando = Some(
                properties
                    .iter()
                    .find(|p| equals_ignore_case(p.name(), "Expando"))
                    .unwrap_or_else(|| panic!("The given key 'Expando' was not present in the dictionary."))
                    .clone(),
            );
        }
        TypeHelper { properties, expando }
    }

    pub fn expando(&self) -> Option<&MagicProperty> {
        self.expando.as_ref()
    }

    pub fn get_properties(&self) -> &[MagicProperty] {
        &self.properties
    }

    /// `FindProperty(string name)`: without regard to case.
    pub fn find_property(&self, name: &str) -> Option<&MagicProperty> {
        self.properties.iter().find(|p| equals_ignore_case(p.name(), name))
    }

    pub fn get_helper_for_type(class: &'static ClassType) -> Arc<TypeHelper> {
        static CACHED_TYPE_LOOKUP: OnceLock<Mutex<HashMap<TypeId, Arc<TypeHelper>>>> = OnceLock::new();
        let mut cache = CACHED_TYPE_LOOKUP.get_or_init(Default::default).lock().unwrap();
        cache.entry(class.id()).or_insert_with(|| Arc::new(TypeHelper::new(class))).clone()
    }

    /// `FindProperty(Type type, string name)`: the declared property with
    /// exactly that name. The original throws when there is none (`First()`).
    pub fn find_property_info(class: &'static ClassType, name: &str) -> Result<&'static PropertyInfo, Error> {
        class
            .properties
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| Error::InvalidOperation("Sequence contains no matching element".to_string()))
    }

    fn load_magic_properties(class: &'static ClassType, properties: &'static [PropertyInfo]) -> Vec<MagicProperty> {
        let configuration = BsonConfiguration::instance();
        let mut magic: Vec<MagicProperty> = Vec::with_capacity(properties.len());
        for (index, property) in properties.iter().enumerate() {
            let name = configuration.alias_for(class.id(), property.name);
            let ignored = configuration.is_ignored(class.id(), property.name);
            let ignored_if_null = configuration.is_ignored_if_null(class.id(), property.name);
            // `Dictionary.Add` of a key that is present.
            assert!(
                !magic.iter().any(|p| equals_ignore_case(p.name(), &name)),
                "An item with the same key has already been added. Key: {}",
                name
            );
            magic.push(MagicProperty::new(property, index, name, ignored, ignored_if_null));
        }
        magic
    }
}

/// A list the deserializer fills.
pub struct ListWrapper {
    list: Vec<Value>,
}

impl BaseWrapper for ListWrapper {
    fn collection(&mut self) -> Value {
        Value::Enumerable(std::mem::take(&mut self.list))
    }

    fn add(&mut self, value: Value) -> Result<(), Error> {
        self.list.push(value);
        Ok(())
    }

    fn create_container(&self, _ty: &Type, _item_type: &Type) -> Option<Value> {
        // An interface gets a `List<T>`, a class with a parameterless
        // constructor an instance of itself, anything else nothing (and then
        // an `ArrayList`): an empty list of values in every case.
        Some(Value::Enumerable(Vec::new()))
    }

    fn set_container(&mut self, container: Option<Value>) -> Result<(), Error> {
        self.list = match container {
            None => Vec::new(),
            Some(Value::Enumerable(list)) => list,
            Some(other) => return Err(invalid_cast(&other, "IList")),
        };
        Ok(())
    }
}

pub struct ListHelper;

impl ListHelper {
    pub fn get_list_item_type(enumerable_type: &Type) -> Type {
        match enumerable_type {
            Type::Array(item) => **item,
            Type::List(item) | Type::Collection(item) => **item,
            // `GetGenericArguments()[0]` of a dictionary is its key type.
            Type::Dictionary(_) => Type::String,
            _ => Type::Object,
        }
    }

    pub fn get_dictionary_key_type(enumerable_type: &Type) -> Type {
        match enumerable_type {
            Type::Dictionary(_) => Type::String,
            Type::Array(item) | Type::List(item) | Type::Collection(item) => **item,
            _ => Type::Object,
        }
    }

    pub fn get_dictionary_value_type(enumerable_type: &Type) -> Type {
        match enumerable_type {
            Type::Dictionary(value) => **value,
            _ => Type::Object,
        }
    }

    pub fn create_dictionary(_dictionary_type: &Type, _key_type: &Type, _value_type: &Type) -> Dictionary {
        Dictionary::new()
    }
}

/// A collection that is not a list, which the deserializer fills.
pub struct CollectionWrapper {
    list: Vec<Value>,
}

impl BaseWrapper for CollectionWrapper {
    fn collection(&mut self) -> Value {
        Value::Enumerable(std::mem::take(&mut self.list))
    }

    fn add(&mut self, value: Value) -> Result<(), Error> {
        self.list.push(value);
        Ok(())
    }

    fn create_container(&self, _ty: &Type, _item_type: &Type) -> Option<Value> {
        Some(Value::Enumerable(Vec::new()))
    }

    fn set_container(&mut self, container: Option<Value>) -> Result<(), Error> {
        self.list = match container {
            None => Vec::new(),
            Some(Value::Enumerable(list)) => list,
            Some(other) => return Err(invalid_cast(&other, "ICollection`1")),
        };
        Ok(())
    }
}

/// What the deserializer adds the items of an array element to. The casts of
/// the items to the item type, which the generic wrappers of the original
/// make in `Add`, are made when the collection is assigned to its property
/// ([`BsonValue::from_value`]).
pub trait BaseWrapper {
    fn add(&mut self, value: Value) -> Result<(), Error>;
    /// The collection that was filled. The wrapper is empty afterwards.
    fn collection(&mut self) -> Value;

    fn create_container(&self, ty: &Type, item_type: &Type) -> Option<Value>;
    fn set_container(&mut self, container: Option<Value>) -> Result<(), Error>;
}

impl dyn BaseWrapper {
    pub fn create(
        ty: &Type,
        item_type: &Type,
        existing_container: Option<Value>,
    ) -> Result<Box<dyn BaseWrapper>, Error> {
        // The original takes the type of the existing container when there is
        // one; a container here is of the declared type of its property.
        let mut instance = <dyn BaseWrapper>::create_wrapper_from_type(ty, item_type)?;
        let container = match existing_container {
            Some(container) => Some(container),
            None => instance.create_container(ty, item_type),
        };
        instance.set_container(container)?;
        Ok(instance)
    }

    fn create_wrapper_from_type(ty: &Type, _item_type: &Type) -> Result<Box<dyn BaseWrapper>, Error> {
        match ty {
            Type::Array(_) | Type::ByteArray => Ok(Box::new(ArrayWrapper { list: Vec::new() })),
            Type::List(_) | Type::Enumerable => Ok(Box::new(ListWrapper { list: Vec::new() })),
            Type::Collection(_) | Type::Dictionary(_) => Ok(Box::new(CollectionWrapper { list: Vec::new() })),
            // a last-ditch pass: a string is an enumerable of characters.
            Type::String => Ok(Box::new(ListWrapper { list: Vec::new() })),
            other => Err(BsonException::with_message(format!(
                "Collection of type {} cannot be deserialized",
                other.full_name()
            ))
            .into()),
        }
    }
}

/// An array the deserializer fills.
pub struct ArrayWrapper {
    list: Vec<Value>,
}

impl BaseWrapper for ArrayWrapper {
    fn add(&mut self, value: Value) -> Result<(), Error> {
        self.list.push(value);
        Ok(())
    }

    fn create_container(&self, _ty: &Type, _item_type: &Type) -> Option<Value> {
        None
    }

    fn set_container(&mut self, container: Option<Value>) -> Result<(), Error> {
        if container.is_some() {
            return Err(
                BsonException::with_message("An container cannot exist when trying to deserialize an array").into()
            );
        }
        Ok(())
    }

    fn collection(&mut self) -> Value {
        Value::Enumerable(std::mem::take(&mut self.list))
    }
}

pub struct Helper;

impl Helper {
    /// The first of January 1970, 00:00:00 UTC.
    pub const EPOCH: DateTime = DateTime::from_unix_ticks(0);
}

pub struct Document {
    pub length: i32,
    pub parent: Option<Box<Document>>,
    pub digested: i32,
}

/// `Deserializer.Options`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub long_integers: bool,
    pub string_dates: bool,
}

/// `System.IO.BinaryReader` over a block of bytes: little-endian numbers,
/// an `EndOfStreamException` when a number is cut short.
pub struct BinaryReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> BinaryReader<'a> {
        BinaryReader { data, position: 0 }
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        if self.data.len() - self.position < N {
            self.position = self.data.len();
            return Err(Error::EndOfStream);
        }
        let mut bytes = [0u8; N];
        bytes.copy_from_slice(&self.data[self.position..self.position + N]);
        self.position += N;
        Ok(bytes)
    }

    pub fn read_byte(&mut self) -> Result<u8, Error> {
        Ok(self.read_array::<1>()?[0])
    }

    pub fn read_boolean(&mut self) -> Result<bool, Error> {
        Ok(self.read_byte()? != 0)
    }

    pub fn read_int32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    pub fn read_int64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.read_array()?))
    }

    pub fn read_double(&mut self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.read_array()?))
    }

    /// `ReadBytes(count)`: fewer bytes than asked for at the end of the data
    /// (not an error); a negative count is an `ArgumentOutOfRangeException`.
    pub fn read_bytes(&mut self, count: i32) -> Result<Vec<u8>, Error> {
        if count < 0 {
            return Err(Error::Argument("Non-negative number required. (Parameter 'count')".to_string()));
        }
        let end = self.data.len().min(self.position + count as usize);
        let bytes = self.data[self.position..end].to_vec();
        self.position = end;
        Ok(bytes)
    }
}

pub struct Deserializer<'r, 'a> {
    reader: &'r mut BinaryReader<'a>,
    current: Option<Box<Document>>,
}

impl<'r, 'a> Deserializer<'r, 'a> {
    /// `_typeMap`: the type an element is read as where no declared type
    /// says. `None` for an element type that is not in the map; `Some(None)`
    /// for null, whose type is null.
    fn type_map(stored_type: StoredType) -> Option<Option<Type>> {
        Some(Some(match stored_type.known()? {
            Types::Int32 => Type::Int32,
            Types::Int64 => Type::Int64,
            Types::Boolean => Type::Boolean,
            Types::String => Type::String,
            Types::Double => Type::Double,
            Types::Binary => Type::ByteArray,
            Types::Regex => Type::Regex,
            Types::DateTime => Type::DateTime,
            Types::ObjectId => Type::ObjectId,
            Types::Array => Type::List(&Type::Object),
            Types::Object => Type::Dictionary(&Type::Object),
            Types::Null => return Some(None),
            _ => return None,
        }))
    }

    fn new(reader: &'r mut BinaryReader<'a>) -> Self {
        Deserializer { reader, current: None }
    }

    /// `Deserialize<T>(byte[] objectData, Options options = null)`.
    pub fn deserialize<T: BsonValue>(object_data: &[u8], options: Option<Options>) -> Result<T, Error> {
        let mut reader = BinaryReader::new(object_data);
        let value = Deserializer::new(&mut reader).read(&T::TYPE, options.unwrap_or_default())?;
        T::from_value(value)
    }

    /// `Deserialize(BinaryReader stream, Type t, Options options = null)`.
    pub fn deserialize_type(
        stream: &'r mut BinaryReader<'a>,
        t: &Type,
        options: Option<Options>,
    ) -> Result<Value, Error> {
        Deserializer::new(stream).read(t, options.unwrap_or_default())
    }

    /// `Read(Type t, Options options)` and `Read<T>(Options options)`.
    fn read(&mut self, t: &Type, options: Options) -> Result<Value, Error> {
        let length = self.reader.read_int32()?;
        self.new_document(length);
        self.deserialize_value(Some(*t), StoredType(Types::Object as i32 as u8), options)
    }

    /// `Read(int read)`.
    fn read_count(&mut self, read: i32) -> Result<(), Error> {
        self.current.as_mut().ok_or(Error::NullReference)?.digested += read;
        Ok(())
    }

    fn is_done(&mut self) -> Result<bool, Error> {
        let current = self.current.as_ref().ok_or(Error::NullReference)?;
        let is_done = current.digested + 1 == current.length;
        if is_done {
            self.reader.read_byte()?; // EOO
            let old = *self.current.take().ok_or(Error::NullReference)?;
            self.current = old.parent;
            if self.current.is_some() {
                self.read_count(old.length)?;
            }
        }
        Ok(is_done)
    }

    fn new_document(&mut self, length: i32) {
        let old = self.current.take();
        self.current = Some(Box::new(Document { length, parent: old, digested: 4 }));
    }

    fn deserialize_value(
        &mut self,
        ty: Option<Type>,
        stored_type: StoredType,
        options: Options,
    ) -> Result<Value, Error> {
        self.deserialize_value_into(ty, stored_type, None, options)
    }

    /// `DeserializeValue(Type type, Types storedType, object container, Options options)`.
    fn deserialize_value_into(
        &mut self,
        ty: Option<Type>,
        stored_type: StoredType,
        container: Option<Value>,
        options: Options,
    ) -> Result<Value, Error> {
        if stored_type.is(Types::Null) {
            return Ok(Value::Null);
        }
        let mut ty = ty.ok_or(Error::NullReference)?;
        while let Type::Nullable(underlying) = ty {
            ty = *underlying;
        }
        if matches!(ty, Type::String) {
            return Ok(Value::String(self.read_string()?));
        }
        if matches!(ty, Type::Int32) {
            let val = self.read_int(stored_type)?;
            return Ok(if options.long_integers { Value::Int64(val as i64) } else { Value::Int32(val) });
        }
        if let Type::Enum(underlying) = ty {
            return self.read_enum(underlying, stored_type);
        }
        if matches!(ty, Type::Single) {
            self.read_count(8)?;
            return Ok(Value::Single(self.reader.read_double()? as f32));
        }
        if stored_type.is(Types::Binary) {
            return self.read_binary();
        }
        if ty.is_enumerable() {
            return self.read_list(&ty, container, options);
        }
        if matches!(ty, Type::Boolean) {
            self.read_count(1)?;
            return Ok(Value::Boolean(self.reader.read_boolean()?));
        }
        if matches!(ty, Type::DateTime) {
            let value = DateTime::from_unix_milliseconds(self.read_long(StoredType(Types::Int64 as i32 as u8))?)?;
            return Ok(if options.string_dates {
                Value::String(value.to_sortable_string())
            } else {
                Value::DateTime(value)
            });
        }
        if matches!(ty, Type::ObjectId) {
            self.read_count(12)?;
            return Ok(Value::ObjectId(ObjectId::from_bytes(self.reader.read_bytes(12)?)));
        }
        if matches!(ty, Type::Int64) {
            return Ok(Value::Int64(self.read_long(stored_type)?));
        }
        if matches!(ty, Type::Double) {
            self.read_count(8)?;
            return Ok(Value::Double(self.reader.read_double()?));
        }
        if matches!(ty, Type::Regex) {
            return self.read_regular_expression();
        }
        if matches!(ty, Type::ScopedCode) {
            return Ok(Value::ScopedCode(Box::new(self.read_scoped_code(options)?)));
        }
        self.read_object(&ty, options)
    }

    fn read_object(&mut self, ty: &Type, options: Options) -> Result<Value, Error> {
        let class = match ty {
            Type::Class(class) => *class,
            Type::Object => Object::CLASS,
            // Deviation (DEVIATIONS.md, Remote protocol): for a type that is
            // not a class (a `Guid` whose element is not binary, an item of a
            // `byte[]` that was written as an array) the original creates a
            // default value of the type and reads the rest of the enclosing
            // document as members it does not have. The port reports the
            // mismatch.
            other => {
                return Err(Error::InvalidCast(format!(
                    "An element cannot be read as an object of type '{}'.",
                    other.full_name()
                )))
            }
        };
        let mut instance = (class.create_instance)();
        let type_helper = TypeHelper::get_helper_for_type(class);
        loop {
            let storage_type = self.read_type()?;
            let name = self.read_name()?;
            let mut is_null = false;
            if storage_type.is(Types::Object) {
                let length = self.reader.read_int32()?;
                if length == 5 {
                    self.reader.read_byte()?; //eoo
                    self.read_count(5)?;
                    is_null = true;
                } else {
                    self.new_document(length);
                }
            }
            let mut container = None;
            let property = type_helper.find_property(&name);
            let property_type = match property {
                Some(property) => *property.property_type(),
                None => Deserializer::type_map(storage_type).flatten().unwrap_or(Type::Object),
            };
            if let Some(property) = property {
                if !property.has_setter() {
                    container = Some(property.getter(&*instance).to_value());
                }
            }
            let has_container = container.is_some();
            let value = if is_null {
                Value::Null
            } else {
                self.deserialize_value_into(Some(property_type), storage_type, container, options)?
            };
            match property {
                None => {
                    if type_helper.expando().is_some() {
                        if let Some(expando) = instance.as_expando() {
                            expando.expando().set(name, value);
                        }
                    }
                }
                Some(property) => {
                    if !has_container && !value.is_null() && !property.ignored() {
                        property.setter(&mut *instance, value)?;
                    } else if has_container && !value.is_null() {
                        // The original fills the container of a property
                        // without a setter in place. The container here is a
                        // copy of the content, so it is stored back.
                        property.setter(&mut *instance, value)?;
                    }
                }
            }
            if self.is_done()? {
                break;
            }
        }
        Ok(Value::Object(instance))
    }

    fn read_list(
        &mut self,
        list_type: &Type,
        existing_container: Option<Value>,
        options: Options,
    ) -> Result<Value, Error> {
        if Deserializer::is_dictionary(list_type) {
            return self.read_dictionary(list_type, existing_container, options);
        }

        let length = self.reader.read_int32()?;
        self.new_document(length);
        let item_type = ListHelper::get_list_item_type(list_type);
        let is_object = matches!(item_type, Type::Object);
        let mut wrapper = <dyn BaseWrapper>::create(list_type, &item_type, existing_container)?;

        while !self.is_done()? {
            let storage_type = self.read_type()?;
            self.read_name()?;
            if storage_type.is(Types::Object) {
                let length = self.reader.read_int32()?;
                self.new_document(length);
            }
            let specific_item_type =
                if is_object { Deserializer::key_of_type_map(storage_type)? } else { Some(item_type) };
            let value = self.deserialize_value(specific_item_type, storage_type, options)?;
            wrapper.add(value)?;
        }
        Ok(wrapper.collection())
    }

    /// `_typeMap[storageType]`: a `KeyNotFoundException` for an element type
    /// that is not in the map.
    fn key_of_type_map(storage_type: StoredType) -> Result<Option<Type>, Error> {
        Deserializer::type_map(storage_type).ok_or_else(|| {
            Error::KeyNotFound(format!("The given key '{}' was not present in the dictionary.", storage_type))
        })
    }

    fn is_dictionary(ty: &Type) -> bool {
        matches!(ty, Type::Dictionary(_))
    }

    fn read_dictionary(
        &mut self,
        list_type: &Type,
        existing_container: Option<Value>,
        options: Options,
    ) -> Result<Value, Error> {
        let value_type = ListHelper::get_dictionary_value_type(list_type);
        let is_object = matches!(value_type, Type::Object);
        let mut container = match existing_container {
            None => {
                ListHelper::create_dictionary(list_type, &ListHelper::get_dictionary_key_type(list_type), &value_type)
            }
            Some(Value::Dictionary(dictionary)) => dictionary,
            Some(other) => return Err(invalid_cast(&other, "IDictionary")),
        };

        while !self.is_done()? {
            let storage_type = self.read_type()?;

            let key = self.read_name()?;
            if storage_type.is(Types::Object) {
                let length = self.reader.read_int32()?;
                self.new_document(length);
            }
            let specific_item_type =
                if is_object { Deserializer::key_of_type_map(storage_type)? } else { Some(value_type) };
            let value = self.deserialize_value(specific_item_type, storage_type, options)?;
            container.add(key, value)?;
        }
        Ok(Value::Dictionary(container))
    }

    fn read_binary(&mut self) -> Result<Value, Error> {
        let length = self.reader.read_int32()?;
        let sub_type = self.reader.read_byte()?;
        self.read_count(5i32.wrapping_add(length))?;
        if sub_type == 2 {
            let count = self.reader.read_int32()?;
            return Ok(Value::ByteArray(self.reader.read_bytes(count)?));
        }
        if sub_type == 3 {
            return Ok(Value::Guid(Guid::from_byte_array(&self.reader.read_bytes(length)?)?));
        }
        Err(BsonException::with_message(format!("No support for binary type: {}", sub_type)).into())
    }

    fn read_name(&mut self) -> Result<String, Error> {
        let mut buffer = Vec::with_capacity(128); //todo: use a pool to prevent fragmentation
        loop {
            let b = self.reader.read_byte()?;
            if b == 0 {
                break;
            }
            buffer.push(b);
        }
        self.read_count(buffer.len() as i32 + 1)?;
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    fn read_string(&mut self) -> Result<String, Error> {
        let length = self.reader.read_int32()?;
        let buffer = self.reader.read_bytes(length.wrapping_sub(1))?; //todo: again, look at fragmentation prevention
        self.reader.read_byte()?; //null;
        self.read_count(4i32.wrapping_add(length))?;

        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }

    fn read_int(&mut self, stored_type: StoredType) -> Result<i32, Error> {
        match stored_type.known() {
            Some(Types::Int32) => {
                self.read_count(4)?;
                self.reader.read_int32()
            }
            Some(Types::Int64) => {
                self.read_count(8)?;
                Ok(self.reader.read_int64()? as i32)
            }
            Some(Types::Double) => {
                self.read_count(8)?;
                // The unchecked conversion of the original is unspecified for
                // a number an `int` cannot hold; the conversion here saturates.
                Ok(self.reader.read_double()? as i32)
            }
            _ => Err(BsonException::with_message(format!("Could not create an int from {}", stored_type)).into()),
        }
    }

    fn read_long(&mut self, stored_type: StoredType) -> Result<i64, Error> {
        match stored_type.known() {
            Some(Types::Int32) => {
                self.read_count(4)?;
                Ok(self.reader.read_int32()? as i64)
            }
            Some(Types::Int64) => {
                self.read_count(8)?;
                self.reader.read_int64()
            }
            Some(Types::Double) => {
                self.read_count(8)?;
                Ok(self.reader.read_double()? as i64)
            }
            _ => Err(BsonException::with_message(format!("Could not create an int64 from {}", stored_type)).into()),
        }
    }

    /// `ReadEnum(Type type, Types storedType)`: the number of the member,
    /// which `Enum.Parse` turns into a value of the enumeration. A number the
    /// underlying type cannot hold is an `OverflowException`.
    fn read_enum(&mut self, underlying: Types, stored_type: StoredType) -> Result<Value, Error> {
        let value = if stored_type.is(Types::Int64) {
            self.read_long(stored_type)?
        } else {
            self.read_int(stored_type)? as i64
        };
        if underlying == Types::Int32 && i32::try_from(value).is_err() {
            return Err(Error::Overflow("Value was either too large or too small for an Int32.".to_string()));
        }
        Ok(Value::Enum { value, underlying })
    }

    fn read_regular_expression(&mut self) -> Result<Value, Error> {
        let pattern = self.read_name()?;
        let options_string = self.read_name()?;

        let mut options = RegexOptions::Compiled;
        if options_string.contains('e') {
            options = options | RegexOptions::ECMAScript;
        }
        if options_string.contains('i') {
            options = options | RegexOptions::IgnoreCase;
        }
        if options_string.contains('l') {
            options = options | RegexOptions::CultureInvariant;
        }
        if options_string.contains('m') {
            options = options | RegexOptions::Multiline;
        }
        if options_string.contains('s') {
            options = options | RegexOptions::Singleline;
        }
        if options_string.contains('w') {
            options = options | RegexOptions::IgnorePatternWhitespace;
        }
        if options_string.contains('x') {
            options = options | RegexOptions::ExplicitCapture;
        }

        Ok(Value::Regex(Regex::new(pattern, options)))
    }

    fn read_type(&mut self) -> Result<StoredType, Error> {
        self.read_count(1)?;
        Ok(StoredType(self.reader.read_byte()?))
    }

    fn read_scoped_code(&mut self, options: Options) -> Result<ScopedCode, Error> {
        self.reader.read_int32()?; //length
        self.read_count(4)?;
        let name = self.read_string()?;
        let length = self.reader.read_int32()?;
        self.new_document(length);
        let scope = self.deserialize_value(Some(Type::Object), StoredType(Types::Object as i32 as u8), options)?;
        Ok(ScopedCode { code_string: Some(name), scope })
    }
}

/// `System.Object` as a class: what the original instantiates for an element
/// it reads as `typeof(object)` (the scope of scoped code, a member of an
/// element type that is not in the type map). It has no properties, so the
/// members of its document are read and dropped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object;

bson_class!(Object as "Object" {});

pub mod configuration {
    //! `Metsys.Bson.Configuration`: aliases of properties and the properties
    //! to leave out, per class.
    //!
    //! The original names a property with an expression (`x => x.Name`) and
    //! reads the name from the expression tree. Here the property is named
    //! by its declared name, so the two forms of `Ignore` take the same
    //! argument and `ExpressionHelper` has no counterpart.

    use std::any::TypeId;
    use std::collections::{HashMap, HashSet};
    use std::marker::PhantomData;
    use std::sync::{Mutex, OnceLock};

    pub trait ITypeConfiguration<T: 'static> {
        fn use_alias(&self, expression: &str, alias: &str) -> &dyn ITypeConfiguration<T>;
        fn ignore(&self, expression: &str) -> &dyn ITypeConfiguration<T>;
        /// `Ignore(string name)`.
        fn ignore_name(&self, name: &str) -> &dyn ITypeConfiguration<T>;
        fn ignore_if_null(&self, expression: &str) -> &dyn ITypeConfiguration<T>;
    }

    pub struct TypeConfiguration<T: 'static> {
        configuration: &'static BsonConfiguration,
        marker: PhantomData<fn() -> T>,
    }

    impl<T: 'static> TypeConfiguration<T> {
        pub(crate) fn new(configuration: &'static BsonConfiguration) -> Self {
            TypeConfiguration { configuration, marker: PhantomData }
        }
    }

    impl<T: 'static> ITypeConfiguration<T> for TypeConfiguration<T> {
        fn use_alias(&self, expression: &str, alias: &str) -> &dyn ITypeConfiguration<T> {
            self.configuration.add_map::<T>(expression, alias);
            self
        }

        fn ignore(&self, expression: &str) -> &dyn ITypeConfiguration<T> {
            self.ignore_name(expression)
        }

        fn ignore_name(&self, name: &str) -> &dyn ITypeConfiguration<T> {
            self.configuration.add_ignore::<T>(name);
            self
        }

        fn ignore_if_null(&self, expression: &str) -> &dyn ITypeConfiguration<T> {
            self.configuration.add_ignore_if_null::<T>(expression);
            self
        }
    }

    /// The configuration of the serializer. A class is configured before its
    /// first object is written or read: the properties of a class are loaded
    /// once.
    pub struct BsonConfiguration {
        alias_map: Mutex<HashMap<TypeId, HashMap<String, String>>>,
        ignored: Mutex<HashMap<TypeId, HashSet<String>>>,
        ignored_if_null: Mutex<HashMap<TypeId, HashSet<String>>>,
    }

    impl BsonConfiguration {
        // The original says of its instance "not thread safe"; a static of
        // Rust has to be, so the tables are behind locks.
        pub fn instance() -> &'static BsonConfiguration {
            static INSTANCE: OnceLock<BsonConfiguration> = OnceLock::new();
            INSTANCE.get_or_init(BsonConfiguration::new)
        }

        fn new() -> BsonConfiguration {
            BsonConfiguration {
                alias_map: Mutex::new(HashMap::new()),
                ignored: Mutex::new(HashMap::new()),
                ignored_if_null: Mutex::new(HashMap::new()),
            }
        }

        pub fn for_type<T: 'static>(action: impl FnOnce(&dyn ITypeConfiguration<T>)) {
            action(&TypeConfiguration::<T>::new(BsonConfiguration::instance()));
        }

        pub fn add_map<T: 'static>(&self, property: &str, alias: &str) {
            let mut alias_map = self.alias_map.lock().unwrap();
            alias_map.entry(TypeId::of::<T>()).or_default().insert(property.to_string(), alias.to_string());
        }

        pub fn alias_for(&self, ty: TypeId, property: &str) -> String {
            let alias_map = self.alias_map.lock().unwrap();
            match alias_map.get(&ty).and_then(|map| map.get(property)) {
                Some(value) => value.clone(),
                None => property.to_string(),
            }
        }

        pub fn add_ignore<T: 'static>(&self, name: &str) {
            let mut ignored = self.ignored.lock().unwrap();
            ignored.entry(TypeId::of::<T>()).or_default().insert(name.to_string());
        }

        pub fn is_ignored(&self, ty: TypeId, name: &str) -> bool {
            let ignored = self.ignored.lock().unwrap();
            ignored.get(&ty).is_some_and(|list| list.contains(name))
        }

        pub fn add_ignore_if_null<T: 'static>(&self, name: &str) {
            let mut ignored_if_null = self.ignored_if_null.lock().unwrap();
            ignored_if_null.entry(TypeId::of::<T>()).or_default().insert(name.to_string());
        }

        pub fn is_ignored_if_null(&self, ty: TypeId, name: &str) -> bool {
            let ignored_if_null = self.ignored_if_null.lock().unwrap();
            ignored_if_null.get(&ty).is_some_and(|list| list.contains(name))
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BsonException {
    message: Option<String>,
    inner_exception: Option<Arc<dyn std::error::Error + Send + Sync>>,
}

impl BsonException {
    pub fn new() -> BsonException {
        BsonException::default()
    }

    pub fn with_message(message: impl Into<String>) -> BsonException {
        BsonException { message: Some(message.into()), inner_exception: None }
    }

    pub fn with_inner_exception(
        message: impl Into<String>,
        inner_exception: Arc<dyn std::error::Error + Send + Sync>,
    ) -> BsonException {
        BsonException { message: Some(message.into()), inner_exception: Some(inner_exception) }
    }

    /// `Exception.Message`.
    pub fn message(&self) -> &str {
        self.message.as_deref().unwrap_or("Exception of type 'Metsys.Bson.BsonException' was thrown.")
    }
}

impl fmt::Display for BsonException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for BsonException {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner_exception {
            Some(inner) => Some(&**inner),
            None => None,
        }
    }
}
