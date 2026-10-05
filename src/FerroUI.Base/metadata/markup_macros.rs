//! The declaration forms of markup metadata.
//!
//! The grammar is deliberately rigid: the same declarations are read by the
//! compiler (expanded into constant [`MarkupType`](super::MarkupType) data)
//! and, syntactically, by build tooling.

/// Declares the markup metadata of a type that is not a class of the object
/// model: a value type, a plain shared (reference) type, a contract, or a
/// type of another crate. (Classes state theirs in the `markup` part of
/// [`ferro_class_info!`](crate::ferro_class_info); enumerations use
/// [`ferro_markup_enum!`](crate::ferro_markup_enum).)
///
/// ```ignore
/// ferro_markup_type!(struct Thickness {
///     handles: [Thickness],
///     parse: Thickness::parse,
///     constructors: [
///         (f64) => Thickness::uniform,
///         (f64, f64, f64, f64) => Thickness::new,
///     ],
///     properties: [
///         Left: f64 { get: |t: &Thickness| t.left },
///     ],
/// });
///
/// ferro_markup_type!(class Setter {
///     handles: [Setter, Rc<Setter>, Option<Rc<Setter>>],
///     this: Rc<Setter>,
///     interfaces: [Rc<dyn ISetter>],
///     constructors: [() => Setter::empty],
///     content: Value,
///     properties: [
///         Property: Option<&'static FerroProperty> { get: Setter::property, set: Setter::set_property },
///         Value: Option<BoxedValue> { get: Setter::value, set: Setter::set_value }
///             [AssignBinding, DependsOn("Property")],
///     ],
/// });
///
/// ferro_markup_type!(interface dyn IBrush as "IBrush" {
///     handles: [Rc<dyn IBrush>, Option<Rc<dyn IBrush>>],
/// });
/// ```
///
/// The head is `class`, `struct`, `interface` or `static`, the Rust type
/// and, when the markup name is not the identifier of the type, `as "Name"`.
/// It implements [`MarkupTyped`](super::MarkupTyped) for the type. The body
/// is a list of the parts below, each optional, in any order:
///
/// * `namespace: "System"` — the dotted namespace, when it differs from the
///   one registered for the declaring module.
/// * `handles: [T, ..]` — the Rust types that hold values of the type, the
///   untyped form first (see [`MarkupType::handles`](super::MarkupType::handles)).
///   For a `struct` (and an enumeration) the nullable form `Option<T>` is
///   recorded automatically as [`MarkupType::nullable`](super::MarkupType::nullable).
/// * `this: T` — the type instance members receive (`&T`); by default the
///   declared type itself. May be written anywhere in the list. A callable written `Type::method` works for any
///   handle that dereferences to `Type`.
/// * `base: T` and `interfaces: [T, ..]` — the base type and the contracts,
///   by handle.
/// * `generic: "FerroList`1" [T]` — the generic definition this type
///   instantiates and its arguments.
/// * `content: Name` — the content property.
/// * `parse: path` — a function `fn(&str) -> Result<T, E>`: the `Parse(string)`
///   of the type.
/// * `constructors: [(A, B) => callable, ..]` — the positional form; a
///   constructor whose parameters carry names or attributes in the managed
///   original names them all: `(property: &'static FerroProperty [InheritDataTypeFrom(2)], mode: BindingMode) => callable`
///   ([`MarkupConstructor::parameter_info`](super::MarkupConstructor::parameter_info)).
/// * `properties: [Name: T { get: callable, set: callable } [Attribute(..), ..], ..]`
///   — `get` is called with `(&this)`, `set` with `(&this, value: T)`;
///   either may be omitted, the attribute list too.
/// * `indexers: [(i32) -> T { get: callable, set: callable }, (String) -> T { get: callable }]`
///   — the indexers (`this[..]`): `get` is called with `(&this, index..)`,
///   `set` with `(&this, index.., value: T)`; either may be omitted, an
///   attribute list may follow. Binding paths use them for `Path[0]` and
///   `Path[key]`.
/// * `property_attributes: [Name: [Attribute(..), ..], ..]` — attributes of
///   REGISTERED properties (which are otherwise not declared at all).
/// * `notify_property_changed: T` — the type raises property change
///   notifications: `T` is the Rust type of its untyped form (the first
///   handle) and implements `INotifyPropertyChanged`. Bindings resolved
///   through metadata subscribe through it, for a value held as `T` (the
///   box is the shared object) or as `Rc<T>`.
/// * `type_info: X` — for a `static` type declared with
///   [`ferro_static_type!`](crate::ferro_static_type): links the metadata to
///   its runtime type (found with `MarkupType::find_by_type_info`).
/// * `methods: [fn Name(A, B) -> R => callable, static fn Name(A) => callable, ..]`
///   — an instance method is called with `(&this, a, b)`. An attribute list
///   may follow the callable (`fn CanSave(Option<BoxedValue>) -> bool => Vm::can_save [DependsOn("Name")]`);
///   the callable is then a plain path (`Type::method`), or any expression in parentheses
///   (`=> (|this: &Rc<Vm>, p| this.can_save(p)) [DependsOn("Name")]`). A
///   callable that is a path is never an index expression.
/// * `fields: [Name: T => callable, ..]` — static FIELDS and constants
///   (`static readonly` / `const` of the managed original), read with `()`.
/// * `static_properties: [Name: T { get: callable, set: callable } [Attribute(..)], ..]`
///   — static PROPERTIES (`static T Name { get; set; }`): `get` is called
///   with `()`, `set` with `(value: T)`; `try_get:` / `try_set:` as for
///   instance properties.
/// * `events: [Name(A, B) => callable, ..]` — the callable subscribes a
///   handler: it is called with `(&this, MarkupDelegate)`. The listed types
///   are the parameters of the handler as the managed original declares
///   them and the delegate is invoked with exactly that many untyped
///   values: `Name(Option<BoxedValue>, EventArgs)` for an
///   `EventHandler<EventArgs>` (the adapter passes the instance as sender
///   when the Rust event has none), `Name()` for an event without
///   arguments.
/// * `attributes: [Name, Name("text", 1, true, type(T), Key = value), ..]`
///   — type attributes; arguments are literals (text, integers, floating
///   point numbers, booleans, characters), `null`, `type(T)` or an array of
///   those (`Name(["a", "b"])`, `Separators = [",", " "]`; arrays nest).
///
/// **Fallible members.** A member whose callable returns `Result<T, E>`
/// (`E: Display`) is declared with `try`: `try fn Name(A) -> T => callable`,
/// `static try fn ..`, `try (A) => callable` for a constructor,
/// `try_get:` / `try_set:` for property and indexer accessors and `try Name(A) => callable`
/// for an event subscription. `Err` becomes
/// [`MarkupInvokeError::Failed`](super::MarkupInvokeError::Failed) (the
/// exception of the managed original); the declared types are the `Ok`
/// types.
///
/// Callables are paths or closures; parameter and property types are the
/// exact Rust types the callable takes and returns.
#[macro_export]
macro_rules! ferro_markup_type {
    ($kind:ident $type_:ty { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@impl $kind $type_, ::std::stringify!($type_), { $($body)* });
    };
    ($kind:ident $type_:ty as $name:literal { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@impl $kind $type_, $name, { $($body)* });
    };

    (@nullable struct $m:ident, $type_:ty) => {
        $m.nullable = ::std::option::Option::Some(|| {
            $crate::data::core::ValueType::of::<::std::option::Option<$type_>>()
        });
    };
    (@nullable $kind:ident $m:ident, $type_:ty) => {};

    (@kind class) => { $crate::metadata::MarkupTypeKind::Class };
    (@kind struct) => { $crate::metadata::MarkupTypeKind::Struct };
    (@kind interface) => { $crate::metadata::MarkupTypeKind::Interface };
    (@kind static) => { $crate::metadata::MarkupTypeKind::Static };

    (@build $kind:ident $type_:ty, $name:expr, $this:ty, { $($body:tt)* }) => {
        impl $crate::metadata::MarkupTyped for $type_ {
            const MARKUP: &'static $crate::metadata::MarkupType = {
                static MARKUP: $crate::metadata::MarkupType = {
                    #[allow(unused_mut)]
                    let mut markup = $crate::metadata::MarkupType::new(
                        $name,
                        $crate::ferro_markup_type!(@kind $kind),
                        ::std::module_path!(),
                    );
                    $crate::ferro_markup_type!(@nullable $kind markup, $type_);
                    $crate::__ferro_markup_items!(markup, $this; $($body)*);
                    markup
                };
                &MARKUP
            };
        }
    };
    // Finds the `this:` part, wherever it is written; the declared type is
    // the default.
    (@scan $kind:ident $type_:ty, $name:expr, [$($seen:tt)*] this: $this:ty, $($rest:tt)*) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $this, { $($seen)* $($rest)* });
    };
    (@scan $kind:ident $type_:ty, $name:expr, [$($seen:tt)*] this: $this:ty) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $this, { $($seen)* });
    };
    (@scan $kind:ident $type_:ty, $name:expr, [$($seen:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ferro_markup_type!(@scan $kind $type_, $name, [$($seen)* $next] $($rest)*);
    };
    (@scan $kind:ident $type_:ty, $name:expr, [$($seen:tt)*]) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $type_, { $($seen)* });
    };
    (@impl $kind:ident $type_:ty, $name:expr, { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@scan $kind $type_, $name, [] $($body)*);
    };
}

/// Declares the markup metadata of an enumeration: its members by name.
///
/// ```ignore
/// ferro_markup_enum!(Dock { Left, Bottom, Right, Top });
///
/// // A member whose Rust variant has another name (`Self` is reserved):
/// ferro_markup_enum!(RelativeSourceMode { DataContext, TemplatedParent, Self = Self_, FindAncestor });
///
/// // A set of flags (a `bitflags!` type): members name their constants.
/// ferro_markup_enum!(flags RoutingStrategies {
///     Direct = RoutingStrategies::DIRECT,
///     Tunnel = RoutingStrategies::TUNNEL,
///     Bubble = RoutingStrategies::BUBBLE,
/// });
///
/// // With attributes or another namespace:
/// ferro_markup_enum!(Dock { Left, Bottom, Right, Top }, { namespace: "FerroUI.Controls" });
/// ```
///
/// The members of a plain enumeration are its variants (their numeric value
/// is the discriminant); the members of a flags type are constants with a
/// `bits()` method. The untyped form of a member is a value of the type
/// itself. Implements [`MarkupTyped`](super::MarkupTyped).
#[macro_export]
macro_rules! ferro_markup_enum {
    ($type_:ident { $($member:ident $(= $variant:ident)?),* $(,)? } $(, { $($body:tt)* })?) => {
        impl $crate::metadata::MarkupTyped for $type_ {
            const MARKUP: &'static $crate::metadata::MarkupType = {
                static MARKUP: $crate::metadata::MarkupType = {
                    #[allow(unused_mut)]
                    let mut markup = $crate::metadata::MarkupType::new(
                        ::std::stringify!($type_),
                        $crate::metadata::MarkupTypeKind::Enum,
                        ::std::module_path!(),
                    );
                    markup.handles = &[|| $crate::data::core::ValueType::of::<$type_>()];
                    markup.nullable = ::std::option::Option::Some(|| {
                        $crate::data::core::ValueType::of::<::std::option::Option<$type_>>()
                    });
                    markup.enum_members = &[
                        $($crate::metadata::MarkupEnumMember {
                            name: ::std::stringify!($member),
                            value: $crate::__ferro_markup_enum_variant!($type_, $member $(, $variant)?) as i64,
                            get: || ::std::rc::Rc::new($crate::__ferro_markup_enum_variant!($type_, $member $(, $variant)?)),
                            rust_variant: ::std::option::Option::Some(
                                $crate::__ferro_markup_enum_variant_name!($member $(, $variant)?),
                            ),
                        },)*
                    ];
                    markup.enum_from_value = ::std::option::Option::Some(|value| {
                        $(
                            if value == $crate::__ferro_markup_enum_variant!($type_, $member $(, $variant)?) as i64 {
                                return ::std::option::Option::Some(::std::rc::Rc::new(
                                    $crate::__ferro_markup_enum_variant!($type_, $member $(, $variant)?),
                                ) as $crate::BoxedValue);
                            }
                        )*
                        ::std::option::Option::None
                    });
                    $($crate::__ferro_markup_items!(markup, $type_; $($body)*);)?
                    markup
                };
                &MARKUP
            };
        }
    };
    (flags $type_:ident { $($member:ident = $value:expr),* $(,)? } $(, { $($body:tt)* })?) => {
        impl $crate::metadata::MarkupTyped for $type_ {
            const MARKUP: &'static $crate::metadata::MarkupType = {
                static MARKUP: $crate::metadata::MarkupType = {
                    #[allow(unused_mut)]
                    let mut markup = $crate::metadata::MarkupType::new(
                        ::std::stringify!($type_),
                        $crate::metadata::MarkupTypeKind::Enum,
                        ::std::module_path!(),
                    );
                    markup.is_flags = true;
                    markup.handles = &[|| $crate::data::core::ValueType::of::<$type_>()];
                    markup.nullable = ::std::option::Option::Some(|| {
                        $crate::data::core::ValueType::of::<::std::option::Option<$type_>>()
                    });
                    markup.enum_members = &[
                        $($crate::metadata::MarkupEnumMember {
                            name: ::std::stringify!($member),
                            value: $value.bits() as i64,
                            get: || ::std::rc::Rc::new($value),
                            rust_variant: ::std::option::Option::None,
                        },)*
                    ];
                    markup.enum_from_value = ::std::option::Option::Some(|value| {
                        #[allow(unused_mut)]
                        let mut result = $type_::empty();
                        #[allow(unused_mut)]
                        let mut rest = value;
                        $(
                            {
                                let bits = $value.bits() as i64;
                                if value & bits == bits {
                                    result = result | $value;
                                    rest &= !bits;
                                }
                            }
                        )*
                        if rest != 0 {
                            return ::std::option::Option::None;
                        }
                        ::std::option::Option::Some(::std::rc::Rc::new(result) as $crate::BoxedValue)
                    });
                    $($crate::__ferro_markup_items!(markup, $type_; $($body)*);)?
                    markup
                };
                &MARKUP
            };
        }
    };
}

/// The parts of a metadata declaration, as assignments to the fields of the
/// `MarkupType` under construction.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_items {
    ($m:ident, $this:ty; ) => {};

    ($m:ident, $this:ty; namespace: $namespace:literal $(, $($rest:tt)*)?) => {
        $m.explicit_namespace = ::std::option::Option::Some($namespace);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; handles: [$($handle:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $m.handles = &[$(|| $crate::data::core::ValueType::of::<$handle>()),*];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; base: $base:ty $(, $($rest:tt)*)?) => {
        $m.base = ::std::option::Option::Some(|| $crate::data::core::ValueType::of::<$base>());
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; interfaces: [$($interface:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $m.interfaces = &[$(|| $crate::data::core::ValueType::of::<$interface>()),*];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; generic: $definition:literal [$($argument:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $m.generic = ::std::option::Option::Some($crate::metadata::MarkupGeneric {
            definition: $definition,
            arguments: &[$(|| $crate::data::core::ValueType::of::<$argument>()),*],
        });
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; content: $content:ident $(, $($rest:tt)*)?) => {
        $m.content_property = ::std::option::Option::Some(::std::stringify!($content));
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; attributes: [$($attributes:tt)*] $(, $($rest:tt)*)?) => {
        $m.attributes = $crate::__ferro_markup_attributes!([] $($attributes)*);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; constructors: [$($constructors:tt)*] $(, $($rest:tt)*)?) => {
        $m.constructors = $crate::__ferro_markup_constructors!([] $($constructors)*);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; parse: $parse:expr $(, $($rest:tt)*)?) => {
        $m.parse = ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let text = arguments.next::<::std::string::String>()?;
            match ($parse)(text.as_str()) {
                ::std::result::Result::Ok(value) => {
                    ::std::result::Result::Ok($crate::metadata::into_markup_value(value))
                }
                ::std::result::Result::Err(error) => ::std::result::Result::Err(
                    $crate::metadata::MarkupInvokeError::Failed(::std::string::ToString::to_string(&error)),
                ),
            }
        });
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; properties: [
        $($name:ident : $type_:ty { $($accessors:tt)* } $([$($attributes:tt)*])?),* $(,)?
    ] $(, $($rest:tt)*)?) => {
        $m.properties = &[
            $($crate::metadata::MarkupProperty {
                name: ::std::stringify!($name),
                type_: || $crate::data::core::ValueType::of::<$type_>(),
                get: $crate::__ferro_markup_getter!($this, $type_; $($accessors)*),
                set: $crate::__ferro_markup_setter!($this, $type_; $($accessors)*),
                attributes: $crate::__ferro_markup_attributes!([] $($($attributes)*)?),
                typed_path_element: $crate::__ferro_markup_typed_path!(
                    $this, $type_, ::std::stringify!($name); [] [] $($accessors)*
                ),
            },)*
        ];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; indexers: [$($indexers:tt)*] $(, $($rest:tt)*)?) => {
        $m.indexers = $crate::__ferro_markup_indexers!($this; [] $($indexers)*);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; property_attributes: [
        $($name:ident : [$($attributes:tt)*]),* $(,)?
    ] $(, $($rest:tt)*)?) => {
        $m.property_attributes = &[
            $((::std::stringify!($name), $crate::__ferro_markup_attributes!([] $($attributes)*)),)*
        ];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; notify_property_changed: $object:ty $(, $($rest:tt)*)?) => {
        $m.notify_property_changed = ::std::option::Option::Some(|value| {
            // The object itself (the box is the shared object) or its handle.
            match value.downcast_ref::<$object>() {
                ::std::option::Option::Some(object) => ::std::option::Option::Some(
                    object as &dyn $crate::data::model::INotifyPropertyChanged,
                ),
                ::std::option::Option::None => value
                    .downcast_ref::<::std::rc::Rc<$object>>()
                    .map(|object| &**object as &dyn $crate::data::model::INotifyPropertyChanged),
            }
        });
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; type_info: $type_info:ty $(, $($rest:tt)*)?) => {
        $m.type_info = ::std::option::Option::Some(|| <$type_info as $crate::StaticType>::TYPE);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; methods: [$($methods:tt)*] $(, $($rest:tt)*)?) => {
        $m.methods = $crate::__ferro_markup_methods!($this; [] $($methods)*);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; fields: [$($name:ident : $type_:ty => $get:expr),* $(,)?] $(, $($rest:tt)*)?) => {
        $m.fields = &[
            $($crate::metadata::MarkupField {
                name: ::std::stringify!($name),
                type_: || $crate::data::core::ValueType::of::<$type_>(),
                get: || $crate::metadata::into_markup_value::<$type_>(($get)()),
                attributes: &[],
            },)*
        ];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; static_properties: [
        $($name:ident : $type_:ty { $($accessors:tt)* } $([$($attributes:tt)*])?),* $(,)?
    ] $(, $($rest:tt)*)?) => {
        $m.static_properties = &[
            $($crate::metadata::MarkupProperty {
                name: ::std::stringify!($name),
                type_: || $crate::data::core::ValueType::of::<$type_>(),
                get: $crate::__ferro_markup_static_getter!($type_; $($accessors)*),
                set: $crate::__ferro_markup_static_setter!($type_; $($accessors)*),
                attributes: $crate::__ferro_markup_attributes!([] $($($attributes)*)?),
                typed_path_element: ::std::option::Option::None,
            },)*
        ];
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };

    ($m:ident, $this:ty; events: [$($events:tt)*] $(, $($rest:tt)*)?) => {
        $m.events = $crate::__ferro_markup_events!($this; [] $($events)*);
        $crate::__ferro_markup_items!($m, $this; $($($rest)*)?);
    };
}

/// `[Name(A, B) => callable, try Name() => callable, ..]` as a constant slice
/// of `MarkupEvent`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_events {
    ($this:ty; [$($done:expr,)*]) => {
        &[$($done),*]
    };
    ($this:ty; [$($done:expr,)*] $name:ident ($($argument:ty),* $(,)?) => $add:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_events!($this; [$($done,)* $crate::metadata::MarkupEvent {
            name: ::std::stringify!($name),
            arguments: &[$(|| $crate::data::core::ValueType::of::<$argument>()),*],
            add: |arguments| {
                let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
                let this = arguments.next::<$this>()?;
                let handler = arguments.next::<$crate::metadata::MarkupDelegate>()?;
                let _ = ($add)(&this, handler);
                ::std::result::Result::Ok(::std::option::Option::None)
            },
        },] $($($rest)*)?)
    };
    ($this:ty; [$($done:expr,)*] try $name:ident ($($argument:ty),* $(,)?) => $add:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_events!($this; [$($done,)* $crate::metadata::MarkupEvent {
            name: ::std::stringify!($name),
            arguments: &[$(|| $crate::data::core::ValueType::of::<$argument>()),*],
            add: |arguments| {
                let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
                let this = arguments.next::<$this>()?;
                let handler = arguments.next::<$crate::metadata::MarkupDelegate>()?;
                let _ = $crate::metadata::markup_result(($add)(&this, handler))?;
                ::std::result::Result::Ok(::std::option::Option::None)
            },
        },] $($($rest)*)?)
    };
}

/// `[(A, B) => callable, try (A) => callable, (name: A [Attribute(..)], b: B) => callable, ..]`
/// as a constant slice of `MarkupConstructor`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_constructors {
    ([$($done:expr,)*]) => {
        &[$($done),*]
    };
    ([$($done:expr,)*] ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructors!([$($done,)*
            $crate::__ferro_markup_constructor!([] ($new) [] [] $($parameters)*),
        ] $($($rest)*)?)
    };
    ([$($done:expr,)*] try ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructors!([$($done,)*
            $crate::__ferro_markup_constructor!([try] ($new) [] [] $($parameters)*),
        ] $($($rest)*)?)
    };
}

/// One `MarkupConstructor`: `[try]? (callable) [types] [parameter info] parameters..`.
/// A parameter is `Type` (positional form) or `name: Type` with an optional
/// attribute list; a constructor whose parameters are all positional has no
/// parameter info.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_constructor {
    ($try_:tt ($new:expr) [$($parameter:ty,)*] [$($info:expr,)*]) => {
        $crate::metadata::MarkupConstructor {
            parameters: &[$(|| $crate::data::core::ValueType::of::<$parameter>()),*],
            parameter_info: &[$($info),*],
            invoke: |arguments| {
                #[allow(unused_mut, unused_variables)]
                let mut arguments = $crate::metadata::MarkupArguments::new(
                    arguments,
                    <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
                )?;
                $crate::__ferro_markup_constructor_result!(
                    $try_ ($new)($(arguments.next::<$parameter>()?),*)
                )
            },
            emit: ::std::option::Option::Some(::std::stringify!($new)),
        }
    };
    ($try_:tt $new:tt [$($parameter:ty,)*] [$($info:expr,)*]
        $name:ident : $type_:ty [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor!($try_ $new [$($parameter,)* $type_,] [$($info,)*
            $crate::metadata::MarkupParameter {
                name: ::std::option::Option::Some(::std::stringify!($name)),
                attributes: $crate::__ferro_markup_attributes!([] $($attributes)*),
            },
        ] $($($rest)*)?)
    };
    ($try_:tt $new:tt [$($parameter:ty,)*] [$($info:expr,)*]
        $name:ident : $type_:ty $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor!($try_ $new [$($parameter,)* $type_,] [$($info,)*
            $crate::metadata::MarkupParameter {
                name: ::std::option::Option::Some(::std::stringify!($name)),
                attributes: &[],
            },
        ] $($($rest)*)?)
    };
    // The positional form: all parameters are types.
    ($try_:tt $new:tt [] [] $($type_:ty),+ $(,)?) => {
        $crate::__ferro_markup_constructor!($try_ $new [$($type_,)+] [])
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_constructor_result {
    ([] $($call:tt)*) => {
        ::std::result::Result::Ok($crate::metadata::into_markup_value($($call)*))
    };
    ([try] $($call:tt)*) => {
        ::std::result::Result::Ok($crate::metadata::into_markup_value(
            $crate::metadata::markup_result($($call)*)?,
        ))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_enum_variant {
    ($type_:ident, $member:ident) => {
        $type_::$member
    };
    ($type_:ident, $member:ident, $variant:ident) => {
        $type_::$variant
    };
}

/// The identifier of the Rust variant of an enumeration member, as text.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_enum_variant_name {
    ($member:ident) => {
        ::std::stringify!($member)
    };
    ($member:ident, $variant:ident) => {
        ::std::stringify!($variant)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_unit {
    ($type_:ty) => {
        ()
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_getter {
    ($this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let this = arguments.next::<$this>()?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(($get)(&this)))
        })
    };
    ($this:ty, $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let this = arguments.next::<$this>()?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                $crate::metadata::markup_result(($get)(&this))?,
            ))
        })
    };
    ($this:ty, $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_getter!($this, $type_; $($($rest)*)?)
    };
    ($this:ty, $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_getter!($this, $type_; $($($rest)*)?)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_setter {
    ($this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
            let this = arguments.next::<$this>()?;
            let value = arguments.next::<$type_>()?;
            let _ = ($set)(&this, value);
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 2)?;
            let this = arguments.next::<$this>()?;
            let value = arguments.next::<$type_>()?;
            let _ = $crate::metadata::markup_result(($set)(&this, value))?;
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_setter!($this, $type_; $($($rest)*)?)
    };
    ($this:ty, $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_setter!($this, $type_; $($($rest)*)?)
    };
}

/// The typed path hook of a property (`MarkupProperty::typed_path_element`)
/// from its accessors: `this, type, name; [getter] [setter] accessors..`.
/// See `metadata::typed_path`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_typed_path {
    ($this:ty, $type_:ty, $name:expr; [$($get:tt)*] [$($set:tt)*]) => {
        ::std::option::Option::Some(|builder, accepts_null| {
            #[allow(unused_imports)]
            use $crate::metadata::typed_path::{
                TypedPathFallback as _, TypedPathNotifying as _, TypedPathShared as _,
            };
            let get: ::std::option::Option<$crate::metadata::typed_path::TypedPathGetter<$this, $type_>> =
                $crate::__ferro_markup_typed_path!(@getter $this, $type_; $($get)*);
            let set: ::std::option::Option<$crate::metadata::typed_path::TypedPathSetter<$this, $type_>> =
                $crate::__ferro_markup_typed_path!(@setter $this, $type_; $($set)*);
            // The most specific form that applies: a notifying shared type, a shared
            // type, anything else (see `metadata::typed_path`).
            (&&$crate::metadata::typed_path::TypedPathProbe::<$this, $type_>::new())
                .typed_path_element(builder, accepts_null, $name, get, set)
        })
    };
    ($this:ty, $type_:ty, $name:expr; [$($get:tt)*] $set:tt get: $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_typed_path!($this, $type_, $name; [get $new] $set $($($rest)*)?)
    };
    ($this:ty, $type_:ty, $name:expr; [$($get:tt)*] $set:tt try_get: $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_typed_path!($this, $type_, $name; [try_get $new] $set $($($rest)*)?)
    };
    ($this:ty, $type_:ty, $name:expr; $get:tt [$($set:tt)*] set: $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_typed_path!($this, $type_, $name; $get [set $new] $($($rest)*)?)
    };
    // A setter that can fail has no typed form: its failure is reported by
    // the untyped accessors.
    ($this:ty, $type_:ty, $name:expr; $get:tt $set:tt try_set: $new:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::None
    };

    (@getter $this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    (@getter $this:ty, $type_:ty; get $get:expr) => {
        ::std::option::Option::Some(::std::rc::Rc::new(
            |this: &$this| -> ::std::result::Result<$type_, $crate::data::BindingError> {
                ::std::result::Result::Ok(($get)(this))
            },
        ))
    };
    (@getter $this:ty, $type_:ty; try_get $get:expr) => {
        ::std::option::Option::Some(::std::rc::Rc::new(
            |this: &$this| -> ::std::result::Result<$type_, $crate::data::BindingError> {
                ($get)(this).map_err(|error| $crate::data::BindingError::message(::std::string::ToString::to_string(&error)))
            },
        ))
    };
    (@setter $this:ty, $type_:ty;) => {
        ::std::option::Option::None
    };
    (@setter $this:ty, $type_:ty; set $set:expr) => {
        ::std::option::Option::Some(::std::rc::Rc::new(|this: &$this, value: $type_| {
            let _ = ($set)(this, value);
        }))
    };
}

/// The getter of a static property: `() -> value`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_static_getter {
    ($type_:ty;) => {
        ::std::option::Option::None
    };
    ($type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            $crate::metadata::MarkupArguments::new(arguments, 0)?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(($get)()))
        })
    };
    ($type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            $crate::metadata::MarkupArguments::new(arguments, 0)?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                $crate::metadata::markup_result(($get)())?,
            ))
        })
    };
    ($type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_static_getter!($type_; $($($rest)*)?)
    };
    ($type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_static_getter!($type_; $($($rest)*)?)
    };
}

/// The setter of a static property: `(value)`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_static_setter {
    ($type_:ty;) => {
        ::std::option::Option::None
    };
    ($type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let _ = ($set)(arguments.next::<$type_>()?);
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(arguments, 1)?;
            let _ = $crate::metadata::markup_result(($set)(arguments.next::<$type_>()?))?;
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_static_setter!($type_; $($($rest)*)?)
    };
    ($type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_static_setter!($type_; $($($rest)*)?)
    };
}

/// `[fn Name(A, B) -> R => callable, static fn Name(A) => callable [Attribute(..)], ..]`
/// as a constant slice of `MarkupMethod`.
///
/// Each entry is recognised in one step (the list is as deep as it has
/// entries) and handed to `__ferro_markup_method!`. A callable followed by an
/// attribute list is a plain path (`Type::method`) or a parenthesised
/// expression: an expression cannot be followed by `[` in a declaration
/// macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_methods {
    ($this:ty; [$($done:expr,)*]) => {
        &[$($done),*]
    };

    ($this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [try] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [try] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [try] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [static] [] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [try] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [try] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [try] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ($this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!($this; [$($done,)* $crate::__ferro_markup_method!(
            $this; [] [] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };
}

/// One `MarkupMethod`: `this; [static]? [try]? Name (parameters) (return)? (callable) [attributes]`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_method {
    ($this:ty; [] $try_:tt $name:ident ($($parameter:ty),*) ($($return_:ty)?) ($call:expr) [$($attributes:tt)*]) => {
        $crate::metadata::MarkupMethod {
            name: ::std::stringify!($name),
            is_static: false,
            parameters: &[$(|| $crate::data::core::ValueType::of::<$parameter>()),*],
            return_type: $crate::__ferro_markup_return_type!($($return_)?),
            invoke: |arguments| {
                let mut arguments = $crate::metadata::MarkupArguments::new(
                    arguments,
                    1 + <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
                )?;
                let this = arguments.next::<$this>()?;
                $crate::__ferro_markup_method_result!(
                    $try_ ($($return_)?) ($call)(&this $(, arguments.next::<$parameter>()?)*)
                )
            },
            attributes: $crate::__ferro_markup_attributes!([] $($attributes)*),
        }
    };
    ($this:ty; [static] $try_:tt $name:ident ($($parameter:ty),*) ($($return_:ty)?) ($call:expr) [$($attributes:tt)*]) => {
        $crate::metadata::MarkupMethod {
            name: ::std::stringify!($name),
            is_static: true,
            parameters: &[$(|| $crate::data::core::ValueType::of::<$parameter>()),*],
            return_type: $crate::__ferro_markup_return_type!($($return_)?),
            invoke: |arguments| {
                #[allow(unused_mut, unused_variables)]
                let mut arguments = $crate::metadata::MarkupArguments::new(
                    arguments,
                    <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
                )?;
                $crate::__ferro_markup_method_result!(
                    $try_ ($($return_)?) ($call)($(arguments.next::<$parameter>()?),*)
                )
            },
            attributes: $crate::__ferro_markup_attributes!([] $($attributes)*),
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_return_type {
    () => {
        ::std::option::Option::None
    };
    ($return_:ty) => {
        ::std::option::Option::Some(|| $crate::data::core::ValueType::of::<$return_>())
    };
}

/// The result of an invoker from the value of the call: `[try]? (return)? call`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_method_result {
    ([] () $($call:tt)*) => {{
        let _ = $($call)*;
        ::std::result::Result::Ok(::std::option::Option::None)
    }};
    ([] ($return_:ty) $($call:tt)*) => {
        ::std::result::Result::Ok($crate::metadata::into_markup_value::<$return_>($($call)*))
    };
    ([try] () $($call:tt)*) => {{
        let _ = $crate::metadata::markup_result($($call)*)?;
        ::std::result::Result::Ok(::std::option::Option::None)
    }};
    ([try] ($return_:ty) $($call:tt)*) => {
        ::std::result::Result::Ok($crate::metadata::into_markup_value::<$return_>(
            $crate::metadata::markup_result($($call)*)?,
        ))
    };
}

/// `[(A, B) -> T { get: callable, set: callable } [Attribute(..)], ..]` as a
/// constant slice of `MarkupIndexer`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_indexers {
    ($this:ty; [$($done:expr,)*]) => {
        &[$($done),*]
    };
    ($this:ty; [$($done:expr,)*]
        ($($parameter:ty),* $(,)?) -> $type_:ty { $($accessors:tt)* } $([$($attributes:tt)*])? $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_indexers!($this; [$($done,)* $crate::metadata::MarkupIndexer {
            parameters: &[$(|| $crate::data::core::ValueType::of::<$parameter>()),*],
            type_: || $crate::data::core::ValueType::of::<$type_>(),
            get: $crate::__ferro_markup_indexer_getter!($this, ($($parameter),*), $type_; $($accessors)*),
            set: $crate::__ferro_markup_indexer_setter!($this, ($($parameter),*), $type_; $($accessors)*),
            attributes: $crate::__ferro_markup_attributes!([] $($($attributes)*)?),
        },] $($($rest)*)?)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_indexer_getter {
    ($this:ty, ($($parameter:ty),*), $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(
                arguments,
                1 + <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
            )?;
            let this = arguments.next::<$this>()?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                ($get)(&this $(, arguments.next::<$parameter>()?)*),
            ))
        })
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(
                arguments,
                1 + <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
            )?;
            let this = arguments.next::<$this>()?;
            ::std::result::Result::Ok($crate::metadata::into_markup_value::<$type_>(
                $crate::metadata::markup_result(($get)(&this $(, arguments.next::<$parameter>()?)*))?,
            ))
        })
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_indexer_getter!($this, ($($parameter),*), $type_; $($($rest)*)?)
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_indexer_getter!($this, ($($parameter),*), $type_; $($($rest)*)?)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_indexer_setter {
    ($this:ty, ($($parameter:ty),*), $type_:ty;) => {
        ::std::option::Option::None
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(
                arguments,
                2 + <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
            )?;
            let this = arguments.next::<$this>()?;
            let _ = ($set)(&this $(, arguments.next::<$parameter>()?)*, arguments.next::<$type_>()?);
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; try_set: $set:expr $(, $($rest:tt)*)?) => {
        ::std::option::Option::Some(|arguments| {
            let mut arguments = $crate::metadata::MarkupArguments::new(
                arguments,
                2 + <[()]>::len(&[$($crate::__ferro_markup_unit!($parameter)),*]),
            )?;
            let this = arguments.next::<$this>()?;
            let _ = $crate::metadata::markup_result(
                ($set)(&this $(, arguments.next::<$parameter>()?)*, arguments.next::<$type_>()?),
            )?;
            ::std::result::Result::Ok(::std::option::Option::None)
        })
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_indexer_setter!($this, ($($parameter),*), $type_; $($($rest)*)?)
    };
    ($this:ty, ($($parameter:ty),*), $type_:ty; try_get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_indexer_setter!($this, ($($parameter),*), $type_; $($($rest)*)?)
    };
}

/// `Name, Name(args..), ..` as a constant slice of `MarkupAttribute`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_attributes {
    ([$($done:expr,)*]) => {
        &[$($done),*]
    };
    ([$($done:expr,)*] $name:ident ($($arguments:tt)*) $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attributes!([$($done,)* $crate::metadata::MarkupAttribute {
            name: ::std::stringify!($name),
            arguments: $crate::__ferro_markup_attribute_arguments!([] $($arguments)*),
            properties: $crate::__ferro_markup_attribute_properties!([] $($arguments)*),
        },] $($($rest)*)?)
    };
    ([$($done:expr,)*] $name:ident $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attributes!([$($done,)* $crate::metadata::MarkupAttribute {
            name: ::std::stringify!($name),
            arguments: &[],
            properties: &[],
        },] $($($rest)*)?)
    };
}

/// The positional arguments of an attribute declaration.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_attribute_arguments {
    ([$($done:expr,)*]) => {
        &[$($done),*]
    };
    // Named arguments are skipped.
    ([$($done:expr,)*] $key:ident = type($type_:ty) $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = null $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = [$($items:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = $value:literal $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*] $($($rest)*)?)
    };
    // An array: its items are positional arguments themselves.
    ([$($done:expr,)*] [$($items:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*
            $crate::metadata::MarkupAttributeValue::Array(
                $crate::__ferro_markup_attribute_arguments!([] $($items)*)
            ),
        ] $($($rest)*)?)
    };
    ([$($done:expr,)*] type($type_:ty) $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*
            $crate::metadata::MarkupAttributeValue::Type(|| $crate::data::core::ValueType::of::<$type_>()),
        ] $($($rest)*)?)
    };
    ([$($done:expr,)*] null $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*
            $crate::metadata::MarkupAttributeValue::Null,
        ] $($($rest)*)?)
    };
    ([$($done:expr,)*] $value:literal $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_arguments!([$($done,)*
            $crate::metadata::MarkupLiteral($value).value(),
        ] $($($rest)*)?)
    };
}

/// The named arguments of an attribute declaration.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_attribute_properties {
    ([$($done:expr,)*]) => {
        &[$($done),*]
    };
    ([$($done:expr,)*] $key:ident = type($type_:ty) $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)* (
            ::std::stringify!($key),
            $crate::metadata::MarkupAttributeValue::Type(|| $crate::data::core::ValueType::of::<$type_>()),
        ),] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = null $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)* (
            ::std::stringify!($key),
            $crate::metadata::MarkupAttributeValue::Null,
        ),] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = $value:literal $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)* (
            ::std::stringify!($key),
            $crate::metadata::MarkupLiteral($value).value(),
        ),] $($($rest)*)?)
    };
    ([$($done:expr,)*] $key:ident = [$($items:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)* (
            ::std::stringify!($key),
            $crate::metadata::MarkupAttributeValue::Array(
                $crate::__ferro_markup_attribute_arguments!([] $($items)*)
            ),
        ),] $($($rest)*)?)
    };
    // Positional arguments are skipped.
    ([$($done:expr,)*] [$($items:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] type($type_:ty) $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] null $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)*] $($($rest)*)?)
    };
    ([$($done:expr,)*] $value:literal $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_attribute_properties!([$($done,)*] $($($rest)*)?)
    };
}
