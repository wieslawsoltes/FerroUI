//! ferroui-system
//!
//! The parts of the base class library of .NET that FerroUI and its reusable libraries share, in
//! Rust: thread-local variables that work on every target, events, change notification and the
//! cancellation of work. Nothing here depends on the user interface; the base
//! library of FerroUI re-exports every item at the path it had there.
//!
//! The modules follow the namespaces of .NET where an item ports one of its types; the items of the
//! namespace `System` are at the root (`IDisposable`, `IObservable<T>`, `IObserver<T>`,
//! `IProgress<T>`).
//!
//! - [`component_model`]: `System.ComponentModel` (`INotifyPropertyChanged`).
//! - [`collections::generic`]: `System.Collections.Generic` (`Dictionary` with the order of
//!   enumeration of .NET).
//! - [`collections::object_model`]: `System.Collections.ObjectModel` (`ObservableCollection`).
//! - [`collections::specialized`]: `System.Collections.Specialized` (`INotifyCollectionChanged` and
//!   the arguments of a change).
//! - [`threading`]: `System.Threading` (`CancellationToken` and its source) and
//!   `System.Threading.Tasks` (`Task<T>`, `TaskCompletionSource<T>`, futures run on their thread).
//! - [`events`]: the multicast events and the lists of handlers that FerroUI's events are built on.
//! - [`ferro_thread_local!`]: thread-local variables, `std::thread_local!` on every target with
//!   thread-local statics and one table per thread on Android ([`mod@ferro_thread_local`]).

pub mod collections;
pub mod component_model;
pub mod events;
pub mod ferro_thread_local;
mod i_disposable;
mod i_observable;
mod i_progress;
mod runtime_type_hash;
pub mod threading;
pub use runtime_type_hash::{runtime_identity_hash, runtime_type_hash, runtime_type_hash_of};

pub use i_disposable::IDisposable;
pub use i_observable::{IObservable, IObserver, ObservableError};
pub use i_progress::IProgress;

/// Immutable UTF-16 transport for ports whose strings admit unpaired surrogates. Native
/// UTF-8 control APIs still need an explicit encoding bridge; this is not a lossy Display.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Utf16Text(std::sync::Arc<[u16]>);
impl Utf16Text {
    pub fn from_units(units: impl Into<std::sync::Arc<[u16]>>) -> Self {
        let units = units.into();
        if units.is_empty() { Self::default() } else { Self(units) }
    }
    pub fn units(&self) -> &[u16] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn to_utf8(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(&self.0)
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}
impl From<&str> for Utf16Text {
    fn from(value: &str) -> Self {
        if value.is_empty() { Self::default() } else { Self(value.encode_utf16().collect()) }
    }
}

/// Native addressable argument storage. Cloning aliases the destination, never its value.
/// This storage is not a substitute CLR class/type for `T&`; the type system must describe
/// the by-reference signature separately when an invoker exposes it as an original method.
pub struct ByRefCell<T>(std::rc::Rc<std::cell::RefCell<T>>);
impl<T> Clone for ByRefCell<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> PartialEq for ByRefCell<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl<T> Eq for ByRefCell<T> {}
impl<T> ByRefCell<T> {
    pub fn new(value: T) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(value)))
    }
    pub fn set(&self, value: T) {
        let previous = self.0.replace(value);
        // Release the storage borrow before destruction can reenter the caller.
        drop(previous);
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: Clone> ByRefCell<T> {
    pub fn get(&self) -> T {
        self.0.borrow().clone()
    }
}

#[cfg(test)]
mod argument_storage_tests {
    use super::{ByRefCell, Utf16Text};
    use std::{cell::Cell, rc::Rc};
    #[test]
    fn exact_text_shares_storage_and_rejects_only_the_explicit_utf8_bridge() {
        let text = Utf16Text::from_units(vec![97, 0xd800, 0xd83d, 0xde00, 0xdfff]);
        let alias = text.clone();
        assert!(text.ptr_eq(&alias));
        assert_eq!(alias.units(), &[97, 0xd800, 0xd83d, 0xde00, 0xdfff]);
        assert!(alias.to_utf8().is_err());
        assert_eq!(Utf16Text::from("😀").len(), 2);
    }
    #[test]
    fn by_reference_clone_aliases_the_actual_destination_and_preserves_float_bits() {
        let slot = ByRefCell::new(f64::from_bits(0x7ff0000000000001));
        let alias = slot.clone();
        assert!(slot.ptr_eq(&alias));
        assert_eq!(alias.get().to_bits(), 0x7ff0000000000001);
        alias.set(-0.0);
        assert_eq!(slot.get().to_bits(), (-0.0f64).to_bits());
        assert!(slot != ByRefCell::new(-0.0));
    }
    struct Reentrant {
        call: Option<Rc<dyn Fn()>>,
    }
    impl Drop for Reentrant {
        fn drop(&mut self) {
            if let Some(call) = self.call.take() {
                call();
            }
        }
    }
    #[test]
    fn replacement_releases_storage_borrow_before_destroying_the_previous_value() {
        let slot = ByRefCell::new(Reentrant { call: None });
        let alias = slot.clone();
        let called = Rc::new(Cell::new(false));
        let flag = called.clone();
        slot.set(Reentrant {
            call: Some(Rc::new(move || {
                alias.set(Reentrant { call: None });
                flag.set(true);
            })),
        });
        slot.set(Reentrant { call: None });
        assert!(called.get());
    }
}

/// `System.IEquatable<T>`: equality supplied by the implementation, independently of the
/// equality of its object reference. `T` is the native argument representation; reference
/// arguments that admit null use their nullable handle, just as other System contracts do.
/// No blanket PartialEq implementation is supplied: custom equality and floating NaN
/// equality need not be the language's `==` operation.
pub trait IEquatable<T> {
    fn equals(&self, other: T) -> bool;
}

#[cfg(test)]
mod equatable_tests {
    use super::IEquatable;
    use std::rc::Rc;
    struct Modulo(i32);
    impl IEquatable<i32> for Modulo {
        fn equals(&self, other: i32) -> bool {
            self.0.rem_euclid(7) == other.rem_euclid(7)
        }
    }
    #[test]
    fn equatable_contract_dispatches_the_actual_implementation() {
        let value: Rc<dyn IEquatable<i32>> = Rc::new(Modulo(8));
        assert!(value.equals(1));
        assert!(value.equals(-6));
        assert!(!value.equals(2));
    }
}

#[cfg(test)]
mod tests;

/// One .NET `System.Char`: an unsigned UTF-16 code unit, including lone surrogates.
/// Scalar conversion is explicit and fallible. Exact text uses [`Utf16Text`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Char(u16);
impl Char {
    pub const fn from_unit(value: u16) -> Self {
        Self(value)
    }
    pub const fn unit(self) -> u16 {
        self.0
    }
    pub fn to_scalar(self) -> Option<char> {
        char::from_u32(u32::from(self.0))
    }
    pub fn from_utf8(value: &str) -> Option<Self> {
        let mut units = value.encode_utf16();
        match (units.next(), units.next()) {
            (Some(unit), None) => Some(Self(unit)),
            _ => None,
        }
    }
    pub fn from_text(value: &Utf16Text) -> Option<Self> {
        match value.units() {
            [unit] => Some(Self(*unit)),
            _ => None,
        }
    }
    pub fn to_text(self) -> Utf16Text {
        Utf16Text::from_units(vec![self.0])
    }
}
impl From<Char> for u16 {
    fn from(value: Char) -> Self {
        value.0
    }
}
impl TryFrom<char> for Char {
    type Error = std::num::TryFromIntError;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        u16::try_from(u32::from(value)).map(Self)
    }
}
impl TryFrom<Char> for char {
    type Error = std::char::CharTryFromError;
    fn try_from(value: Char) -> Result<Self, Self::Error> {
        char::try_from(u32::from(value.0))
    }
}
/// Shared original-object boxing; the UI layer re-exports this exact protocol.
pub mod boxed_value {
    use std::{
        any::{Any, TypeId},
        fmt,
        rc::Rc,
    };
    /// Implemented for every type that can be a property value. Unlike
    /// [`Any`], it supports equality, so untyped values can themselves be
    /// property values (for example a data context).
    pub trait AnyValue: Any {
        /// The value as [`Any`]. Call [`as_any`](trait.AnyValue.html#method.as_any)
        /// on `dyn AnyValue` instead.
        ///
        /// The trait is implemented for every `PartialEq` type, which includes
        /// `Rc<dyn AnyValue>` itself. The trait methods therefore have names that
        /// cannot be reached by accident on a boxed value: the inherent methods
        /// of `dyn AnyValue` are the API, and they dereference through the box.
        #[doc(hidden)]
        fn any_value_as_any(&self) -> &dyn Any;

        /// Compares with another untyped value: equal if both hold the same type
        /// and the values are equal. Use `==` on `dyn AnyValue` instead.
        #[doc(hidden)]
        fn any_value_eq(&self, other: &dyn AnyValue) -> bool;

        /// The Rust type name of the contained value. Use `type_name` on
        /// `dyn AnyValue` instead.
        #[doc(hidden)]
        fn any_value_type_name(&self) -> &'static str;
    }

    impl<T: PartialEq + 'static> AnyValue for T {
        #[inline]
        fn any_value_as_any(&self) -> &dyn Any {
            self
        }

        fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
            other
                .any_value_as_any()
                .downcast_ref::<T>()
                .is_some_and(|other| value_equals(self, other))
        }

        fn any_value_type_name(&self) -> &'static str {
            std::any::type_name::<T>()
        }
    }

    /// Deviation (DEVIATIONS.md, Property system): the comparer of the reference
    /// in place of `PartialEq` alone.
    ///
    /// Whether two property values are equal as the reference compares them
    /// (`EqualityComparer<T>.Default.Equals`, and `object.Equals` for boxed
    /// values): `==`, and a floating-point value that is not a number equals
    /// another that is not a number, bare or nullable. With `==` alone a value
    /// store whose value is NaN (a `Width` that is not set) reports a change
    /// every time the value is set again.
    #[inline]
    pub fn value_equals<T: PartialEq + 'static>(a: &T, b: &T) -> bool {
        a == b || both_not_a_number(a, b)
    }

    #[inline]
    fn both_not_a_number<T: 'static>(a: &T, b: &T) -> bool {
        let (a, b): (&dyn Any, &dyn Any) = (a, b);
        if let (Some(a), Some(b)) = (a.downcast_ref::<f64>(), b.downcast_ref::<f64>()) {
            return a.is_nan() && b.is_nan();
        }
        if let (Some(a), Some(b)) = (a.downcast_ref::<f32>(), b.downcast_ref::<f32>()) {
            return a.is_nan() && b.is_nan();
        }
        if let (Some(Some(a)), Some(Some(b))) = (
            a.downcast_ref::<Option<f64>>(),
            b.downcast_ref::<Option<f64>>(),
        ) {
            return a.is_nan() && b.is_nan();
        }
        if let (Some(Some(a)), Some(Some(b))) = (
            a.downcast_ref::<Option<f32>>(),
            b.downcast_ref::<Option<f32>>(),
        ) {
            return a.is_nan() && b.is_nan();
        }
        false
    }

    impl dyn AnyValue {
        /// The contained value as [`Any`], for downcasting.
        #[inline]
        pub fn as_any(&self) -> &dyn Any {
            self.any_value_as_any()
        }

        /// The [`TypeId`] of the contained value.
        #[inline]
        pub fn value_type_id(&self) -> TypeId {
            self.any_value_as_any().type_id()
        }

        /// The Rust type name of the contained value.
        #[inline]
        pub fn type_name(&self) -> &'static str {
            self.any_value_type_name()
        }

        /// Whether the contained value is of type `T`.
        #[inline]
        pub fn is<T: 'static>(&self) -> bool {
            self.any_value_as_any().is::<T>()
        }

        /// The contained value, if it is of type `T`.
        #[inline]
        pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
            self.any_value_as_any().downcast_ref::<T>()
        }

        /// Compares with another untyped value: equal if both hold the same type
        /// and the values are equal.
        #[inline]
        pub fn value_eq(&self, other: &dyn AnyValue) -> bool {
            self.any_value_eq(other)
        }
    }

    impl PartialEq for dyn AnyValue {
        fn eq(&self, other: &Self) -> bool {
            self.any_value_eq(other)
        }
    }

    impl fmt::Debug for dyn AnyValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "AnyValue<{}>", self.any_value_type_name())
        }
    }

    /// An untyped, shared property value. The contained value is always exactly
    /// of the property's value type (or one of the marker types
    /// [`UnsetValueType`] and [`DoNothingType`]).
    pub type BoxedValue = Rc<dyn AnyValue>;
}
pub use boxed_value::{value_equals, AnyValue, BoxedValue};

/// UI-independent globalization objects shared by typing engines and native metadata.
pub mod globalization {
    use crate::Utf16Text;
    use bitflags::bitflags;
    use std::{cmp::Ordering, rc::Rc, sync::OnceLock};
    /// The exact UTF16 uppercase operation supplied by an application or platform.
    pub type TextUppercaseCallback = Rc<dyn Fn(&Utf16Text) -> Result<Utf16Text, TextCasingError>>;
    pub type TextCasingService = fn(&str, &Utf16Text) -> Result<Utf16Text, TextCasingError>;
    static CASING: OnceLock<TextCasingService> = OnceLock::new();

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TextCasingError {
        NullArgument,
        ServiceUnavailable,
        UnsupportedCulture(String),
        InvalidUtf16,
        Failed(String),
    }
    impl std::fmt::Display for TextCasingError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NullArgument => {
                    f.write_str("TextInfo.ToUpper requires a non-null str argument")
                }
                Self::ServiceUnavailable => {
                    f.write_str("the host has not supplied exact text casing data")
                }
                Self::UnsupportedCulture(name) => {
                    write!(f, "text casing data is unavailable for culture {name:?}")
                }
                Self::InvalidUtf16 => {
                    f.write_str("the UTF8 casing bridge cannot represent the exact UTF16 result")
                }
                Self::Failed(message) => f.write_str(message),
            }
        }
    }
    impl std::error::Error for TextCasingError {}
    #[derive(Clone)]
    enum Uppercase {
        Service,
        Callback(TextUppercaseCallback),
    }

    crate::ferro_thread_local! {
        static INVARIANT: Rc<TextInfo> = Rc::new(TextInfo::new());
    }

    /// The text conventions of a culture (C# `TextInfo`).
    ///
    /// Only the conventions of the invariant culture are built in: those of
    /// other cultures come from the provider registered with the locator (see
    /// [`ICultureDataProvider`](super::ICultureDataProvider)); a culture without
    /// data gets the conventions of its nearest parent that has data and finally
    /// the invariant ones.
    #[derive(Clone)]
    pub struct TextInfo {
        list_separator: String,
        uppercase: Uppercase,
        culture_name: Rc<str>,
    }

    static CURRENT_INFO: OnceLock<fn() -> Rc<TextInfo>> = OnceLock::new();
    impl TextInfo {
        pub fn install_current_info_reader(read: fn() -> Rc<TextInfo>) -> Result<(), &'static str> {
            let old = CURRENT_INFO.get_or_init(|| read);
            if std::ptr::fn_addr_eq(*old, read) {
                Ok(())
            } else {
                Err("current TextInfo reader already installed")
            }
        }

        /// The conventions of the invariant culture, to be changed with the
        /// setters.
        pub fn new() -> TextInfo {
            TextInfo {
                list_separator: ",".to_owned(),
                uppercase: Uppercase::Service,
                culture_name: Rc::from(""),
            }
        }

        /// A text-info object whose own casing language is independent of the culture that returns it.
        pub fn for_language(name: &str) -> Self {
            Self {
                culture_name: Rc::from(name),
                ..Self::new()
            }
        }
        /// Retains and invokes the original callback; no culture-name projection is involved.
        pub fn with_uppercase_callback(mut self, callback: TextUppercaseCallback) -> Self {
            self.uppercase = Uppercase::Callback(callback);
            self
        }
        pub fn install_casing_service(service: TextCasingService) -> Result<(), &'static str> {
            let installed = CASING.get_or_init(|| service);
            if std::ptr::fn_addr_eq(*installed, service) {
                Ok(())
            } else {
                Err("text casing service already installed")
            }
        }
        /// Source TextInfo.ToUpper(string). Null fails before consulting any data or callback.
        pub fn try_to_upper_text(
            &self,
            text: Option<&Utf16Text>,
        ) -> Result<Utf16Text, TextCasingError> {
            let text = text.ok_or(TextCasingError::NullArgument)?;
            if text.is_empty() {
                return Ok(text.clone());
            }
            match &self.uppercase {
                Uppercase::Service => CASING.get().ok_or(TextCasingError::ServiceUnavailable)?(
                    &self.culture_name,
                    text,
                ),
                Uppercase::Callback(callback) => callback(text),
            }
        }
        /// Explicit checked UTF8 bridge; native markup uses the exact UTF16 operation.
        pub fn try_to_upper(&self, text: &str) -> Result<String, TextCasingError> {
            self.try_to_upper_text(Some(&Utf16Text::from(text)))?
                .to_utf8()
                .map_err(|_| TextCasingError::InvalidUtf16)
        }
        pub fn to_upper(&self, text: &str) -> String {
            self.try_to_upper(text)
                .unwrap_or_else(|error| std::panic::resume_unwind(Box::new(error)))
        }

        /// The conventions of the invariant culture.
        pub fn invariant_info() -> Rc<TextInfo> {
            INVARIANT.with(Rc::clone)
        }

        /// The conventions of the current culture.
        pub fn current_info() -> Rc<TextInfo> {
            CURRENT_INFO
                .get()
                .map_or_else(Self::invariant_info, |read| read())
        }

        /// Gets the string that separates items in a list (C# `ListSeparator`).
        pub fn list_separator(&self) -> &str {
            &self.list_separator
        }

        /// Sets the string that separates items in a list.
        pub fn set_list_separator(&mut self, value: impl Into<String>) {
            self.list_separator = value.into();
        }

        /// The conventions with the given list separator.
        pub fn with_list_separator(mut self, value: impl Into<String>) -> Self {
            self.set_list_separator(value);
            self
        }
    }

    // Source TextInfo.Equals compares culture names; object reference identity is separate.
    impl PartialEq for TextInfo {
        fn eq(&self, other: &Self) -> bool {
            self.culture_name == other.culture_name
        }
    }
    impl Eq for TextInfo {}
    impl std::fmt::Debug for TextInfo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut d = f.debug_struct("TextInfo");
            d.field("list_separator", &self.list_separator);
            d.field("casing_language", &self.culture_name);
            if let Uppercase::Callback(_) = &self.uppercase {
                d.field("casing", &"custom callback");
            }
            d.finish()
        }
    }
    impl Default for TextInfo {
        fn default() -> Self {
            Self::new()
        }
    }

    bitflags! {
        /// The string comparison options to use with [`CompareInfo`].
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct CompareOptions: i32 {
            /// The default option settings for string comparisons.
            const NONE = 0;
            /// The string comparison must ignore case.
            const IGNORE_CASE = 0x1;
            /// The string comparison must use successive UTF-16 code units,
            /// upper-cased with the invariant simple case mapping.
            const ORDINAL_IGNORE_CASE = 0x1000_0000;
            /// The string comparison must use successive UTF-16 code units.
            const ORDINAL = 0x4000_0000;
        }
    }

    /// The culture-sensitive comparison rules of a culture, supplied by a
    /// culture data provider. `options` never has an ordinal flag: the ordinal
    /// comparisons do not reach the rules of a culture.
    pub trait ICompareRules: 'static {
        /// Compares two strings.
        fn compare(&self, a: &str, b: &str, options: CompareOptions) -> Ordering;

        /// Whether `source` starts with `prefix`.
        fn is_prefix(&self, source: &str, prefix: &str, options: CompareOptions) -> bool;

        /// The position of the first occurrence of `value` in `source` in
        /// UTF-16 code units, or -1.
        fn index_of(&self, source: &str, value: &str, options: CompareOptions) -> i32;
    }

    /// Implements a set of methods for culture-sensitive string comparisons:
    /// the comparer of a culture.
    #[derive(Clone)]
    pub struct CompareInfo {
        name: Rc<str>,
        /// The rules of the culture; the built-in invariant rules if `None`.
        rules: Option<Rc<dyn ICompareRules>>,
    }

    impl PartialEq for CompareInfo {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
        }
    }

    impl CompareInfo {
        pub fn for_name(name: &str, rules: Option<Rc<dyn ICompareRules>>) -> Self {
            Self {
                name: Rc::from(name),
                rules,
            }
        }

        /// The comparer of the invariant culture.
        pub fn invariant() -> CompareInfo {
            CompareInfo {
                name: Rc::from(""),
                rules: None,
            }
        }

        /// The name of the culture used for the comparisons.
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Compares two strings using the specified options.
        pub fn compare(&self, a: &str, b: &str, options: CompareOptions) -> Ordering {
            match self.culture_rules(options) {
                Some(rules) => rules.compare(a, b, options),
                None => {
                    let ignore_case = Self::built_in_ignores_case(options);
                    OrdinalUnits::new(a, ignore_case).cmp(OrdinalUnits::new(b, ignore_case))
                }
            }
        }

        /// Determines whether `source` starts with `prefix` using the specified
        /// options.
        pub fn is_prefix(&self, source: &str, prefix: &str, options: CompareOptions) -> bool {
            match self.culture_rules(options) {
                Some(rules) => rules.is_prefix(source, prefix, options),
                None => {
                    let ignore_case = Self::built_in_ignores_case(options);
                    let mut source = OrdinalUnits::new(source, ignore_case);
                    OrdinalUnits::new(prefix, ignore_case).all(|unit| source.next() == Some(unit))
                }
            }
        }

        /// Searches for `value` in `source` using the specified options and
        /// returns the position of its first occurrence in UTF-16 code units,
        /// or -1. An empty value is found at 0.
        pub fn index_of(&self, source: &str, value: &str, options: CompareOptions) -> i32 {
            match self.culture_rules(options) {
                Some(rules) => rules.index_of(source, value, options),
                None => {
                    let ignore_case = Self::built_in_ignores_case(options);
                    let value: Vec<u16> = OrdinalUnits::new(value, ignore_case).collect();
                    if value.is_empty() {
                        return 0;
                    }
                    let source: Vec<u16> = OrdinalUnits::new(source, ignore_case).collect();
                    source
                        .windows(value.len())
                        .position(|window| window == value.as_slice())
                        .map_or(-1, |index| index as i32)
                }
            }
        }

        /// The rules of the culture for a culture-sensitive comparison; `None`
        /// for an ordinal comparison and for a culture without rules.
        fn culture_rules(&self, options: CompareOptions) -> Option<&Rc<dyn ICompareRules>> {
            if options.intersects(CompareOptions::ORDINAL | CompareOptions::ORDINAL_IGNORE_CASE) {
                return None;
            }
            self.rules.as_ref()
        }

        /// Whether the built-in comparison for `options` ignores case.
        fn built_in_ignores_case(options: CompareOptions) -> bool {
            options.intersects(CompareOptions::IGNORE_CASE | CompareOptions::ORDINAL_IGNORE_CASE)
        }

        /// The upper-case form of a character under the invariant simple case
        /// mapping, the mapping of the ordinal comparison that ignores case: one
        /// character maps to one character (no expansions: `ß` stays `ß`), no
        /// culture is involved, and the dotless `ı` (U+0131) and the long `ſ`
        /// (U+017F) keep their identity, as in the reference runtime.
        pub fn to_upper_ordinal(c: char) -> char {
            if c.is_ascii() {
                return c.to_ascii_uppercase();
            }
            match c {
                '\u{0131}' | '\u{017F}' => return c,
                // The Greek letters with ypogegrammeni: their simple upper-case
                // forms are the letters with prosgegrammeni, their full ones are
                // two characters.
                '\u{1F80}'..='\u{1F87}' | '\u{1F90}'..='\u{1F97}' | '\u{1FA0}'..='\u{1FA7}' => {
                    return char::from_u32(c as u32 + 8).unwrap_or(c);
                }
                '\u{1FB3}' => return '\u{1FBC}',
                '\u{1FC3}' => return '\u{1FCC}',
                '\u{1FF3}' => return '\u{1FFC}',
                // Case pairs the reference runtime folds that are newer than the
                // case tables of some toolchains.
                '\u{A7CF}' => return '\u{A7CE}',
                '\u{A7D3}' => return '\u{A7D2}',
                '\u{A7D5}' => return '\u{A7D4}',
                _ => {}
            }
            let mut upper = c.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(upper), None) => upper,
                // A character whose only upper-case form is an expansion has no
                // simple one.
                _ => c,
            }
        }
    }

    /// The UTF-16 code units of a text for an ordinal comparison, upper-cased
    /// character by character when the comparison ignores case.
    struct OrdinalUnits<'a> {
        chars: std::str::Chars<'a>,
        ignore_case: bool,
        pending: Option<u16>,
    }

    impl<'a> OrdinalUnits<'a> {
        fn new(text: &'a str, ignore_case: bool) -> Self {
            Self {
                chars: text.chars(),
                ignore_case,
                pending: None,
            }
        }
    }

    impl Iterator for OrdinalUnits<'_> {
        type Item = u16;

        fn next(&mut self) -> Option<u16> {
            if let Some(unit) = self.pending.take() {
                return Some(unit);
            }
            let c = self.chars.next()?;
            let c = if self.ignore_case {
                CompareInfo::to_upper_ordinal(c)
            } else {
                c
            };
            let mut units = [0u16; 2];
            let units = c.encode_utf16(&mut units);
            if units.len() == 2 {
                self.pending = Some(units[1]);
            }
            Some(units[0])
        }
    }
}
pub use globalization::{
    CompareInfo, CompareOptions, ICompareRules, TextCasingError, TextCasingService, TextInfo,
    TextUppercaseCallback,
};

/* Marvin32 derives from .NET runtime revision
60629d14374c56f1cb51819049ad1fa529307f8d. Exact servicing behavior is separately measured.
The MIT License (MIT)

Copyright (c) .NET Foundation and Contributors

All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
pub fn process_hash_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    static SEED: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *SEED.get_or_init(|| {
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    })
}
#[doc(hidden)]
pub fn hash_culture_text(text: &Utf16Text, seed: u64) -> i32 {
    fn block(a: &mut u32, b: &mut u32) {
        *b ^= *a;
        *a = a.rotate_left(20);
        *a = a.wrapping_add(*b);
        *b = b.rotate_left(9);
        *b ^= *a;
        *a = a.rotate_left(27);
        *a = a.wrapping_add(*b);
        *b = b.rotate_left(19);
    }
    let mut a = seed as u32;
    let mut b = (seed >> 32) as u32;
    let mut pairs = text.units().chunks_exact(2);
    for pair in &mut pairs {
        let word = if cfg!(target_endian = "little") {
            (pair[0] as u32) | ((pair[1] as u32) << 16)
        } else {
            ((pair[0] as u32) << 16) | (pair[1] as u32)
        };
        a = a.wrapping_add(word);
        block(&mut a, &mut b);
    }
    let partial = match pairs.remainder() {
        [] => {
            if cfg!(target_endian = "little") {
                0x80
            } else {
                0x80000000
            }
        }
        [unit] => {
            if cfg!(target_endian = "little") {
                (*unit as u32) | 0x00800000
            } else {
                ((*unit as u32) << 16) | 0x00008000
            }
        }
        _ => unreachable!(),
    };
    a = a.wrapping_add(partial);
    block(&mut a, &mut b);
    block(&mut a, &mut b);
    (a ^ b) as i32
}

mod shared_array;
pub use shared_array::{SharedArray,WeakSharedArray,ArrayIndexOutOfRange};

#[cfg(test)]
mod empty_utf16_reference_tests {
    use super::Utf16Text;
    use std::sync::Arc;
    #[test]
    fn empty_construction_shares_the_original_empty_reference() {
        let empty=Utf16Text::default();
        assert!(empty.ptr_eq(&Utf16Text::from("")));
        assert!(empty.ptr_eq(&Utf16Text::from_units(Vec::<u16>::new())));
        assert!(empty.ptr_eq(&Utf16Text::from_units(Arc::<[u16]>::from([]))));
        assert!(empty.ptr_eq(&std::thread::spawn(Utf16Text::default).join().unwrap()));
    }
    #[test]
    fn nonempty_construction_preserves_original_units_and_allocation() {
        let original:Arc<[u16]>=vec![0xd800,0,0xdfff].into();
        let a=Utf16Text::from_units(original.clone());
        let b=Utf16Text::from_units(original);
        assert!(a.ptr_eq(&b));
        assert_eq!(a.units(),&[0xd800,0,0xdfff]);
        let other=Utf16Text::from_units(vec![0xd800,0,0xdfff]);
        assert_eq!(a,other);
        assert!(!a.ptr_eq(&other));
    }
}
