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
    // A contract (`dyn Trait`): without a `this:` part its instance members
    // receive the unsized trait object, which has no value type of its own.
    ($kind:ident dyn $trait_:path as $name:literal { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@impl $kind dyn $trait_, $name, implied_dyn, { $($body)* });
    };
    ($kind:ident $type_:ty { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@impl $kind $type_, ::std::stringify!($type_), implied, { $($body)* });
    };
    ($kind:ident $type_:ty as $name:literal { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@impl $kind $type_, $name, implied, { $($body)* });
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

    (@this $kind:ident implied_dyn $m:ident, $this:ty) => {};
    (@this $kind:ident $form:ident $m:ident, $this:ty) => {
        $m.this = ::std::option::Option::Some(|| $crate::data::core::ValueType::of::<$this>());
    };

    // The typed functions of the members, as associated functions of the type, and
    // the value type of the type: what a constructor and `Parse` return. It is the
    // instance type when the declaration states `this:`, else the first of its
    // `handles:` (the untyped form), else the declared type (`Rc<dyn Trait>` for a
    // contract).
    (@fns explicit $type_:ty, $this:ty; $($body:tt)*) => {
        $crate::ferro_markup_type!(@fns_impl $type_, $this, $this; $($body)*);
    };
    (@fns $form:ident $type_:ty, $this:ty; $($body:tt)*) => {
        $crate::ferro_markup_type!(@fns_value $form $type_, $this, [$($body)*] $($body)*);
    };
    (@fns_value $form:ident $type_:ty, $this:ty, [$($all:tt)*] handles: [$first:ty $(, $more:ty)* $(,)?] $($rest:tt)*) => {
        $crate::ferro_markup_type!(@fns_impl $type_, $this, $first; $($all)*);
    };
    (@fns_value $form:ident $type_:ty, $this:ty, [$($all:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ferro_markup_type!(@fns_value $form $type_, $this, [$($all)*] $($rest)*);
    };
    (@fns_value implied_dyn $type_:ty, $this:ty, [$($all:tt)*]) => {
        $crate::ferro_markup_type!(@fns_impl $type_, $this, ::std::rc::Rc<$type_>; $($all)*);
    };
    (@fns_value $form:ident $type_:ty, $this:ty, [$($all:tt)*]) => {
        $crate::ferro_markup_type!(@fns_impl $type_, $this, $this; $($all)*);
    };
    (@fns_impl $type_:ty, $this:ty, $value:ty; $($body:tt)*) => {
        impl $type_ {
            /// The value type of the type (`MarkupType::value`).
            #[doc(hidden)]
            pub const __MARKUP_VALUE: $crate::metadata::TypeOf = || $crate::data::core::ValueType::of::<$value>();

            $crate::__ferro_markup_fns!($this, $value; $($body)*);
        }
    };

    (@build $kind:ident $type_:ty, $name:expr, $this:ty, $form:ident, { $($body:tt)* }) => {
        $crate::__ferro_compiler_metadata!($crate::ferro_markup_type!(@fns $form $type_, $this; $($body)*););

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
                    $crate::__ferro_compiler_metadata!(
                        $crate::ferro_markup_type!(@this $kind $form markup, $this);
                        markup.value = ::std::option::Option::Some(<$type_>::__MARKUP_VALUE);
                    );
                    $crate::__ferro_markup_items!(markup, $this; $($body)*);
                    markup
                };
                &MARKUP
            };
        }
    };
    // Finds the `this:` part, wherever it is written; the declared type is
    // the default.
    (@scan $kind:ident $type_:ty, $name:expr, $default:ident, [$($seen:tt)*] this: $this:ty, $($rest:tt)*) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $this, explicit, { $($seen)* $($rest)* });
    };
    (@scan $kind:ident $type_:ty, $name:expr, $default:ident, [$($seen:tt)*] this: $this:ty) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $this, explicit, { $($seen)* });
    };
    (@scan $kind:ident $type_:ty, $name:expr, $default:ident, [$($seen:tt)*] $next:tt $($rest:tt)*) => {
        $crate::ferro_markup_type!(@scan $kind $type_, $name, $default, [$($seen)* $next] $($rest)*);
    };
    (@scan $kind:ident $type_:ty, $name:expr, $default:ident, [$($seen:tt)*]) => {
        $crate::ferro_markup_type!(@build $kind $type_, $name, $type_, $default, { $($seen)* });
    };
    (@impl $kind:ident $type_:ty, $name:expr, $default:ident, { $($body:tt)* }) => {
        $crate::ferro_markup_type!(@scan $kind $type_, $name, $default, [] $($body)*);
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
        $m.constructors = $crate::__ferro_markup_pool!(constructors { [] $($constructors)* });
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
                emit_get: $crate::__ferro_compiler_metadata!(@value $crate::__ferro_markup_emit_accessor!(get $name; $($accessors)*)),
                emit_set: $crate::__ferro_compiler_metadata!(@value $crate::__ferro_markup_emit_accessor!(set $name; $($accessors)*)),
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
        $m.methods = $crate::__ferro_markup_pool!(methods { $this; [] $($methods)* });
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
                emit_get: $crate::__ferro_compiler_metadata!(@value $crate::__ferro_markup_emit_accessor!(static_get $name; $($accessors)*)),
                emit_set: $crate::__ferro_compiler_metadata!(@value $crate::__ferro_markup_emit_accessor!(static_set $name; $($accessors)*)),
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
    ([$($pool:ident)*] [$($done:expr,)*]) => {
        &[$($done),*]
    };
    ([$index:ident $($pool:ident)*] [$($done:expr,)*] ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructors!([$($pool)*] [$($done,)*
            $crate::__ferro_markup_constructor!($index [] ($new) [] [] $($parameters)*),
        ] $($($rest)*)?)
    };
    ([$index:ident $($pool:ident)*] [$($done:expr,)*] try ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructors!([$($pool)*] [$($done,)*
            $crate::__ferro_markup_constructor!($index [try] ($new) [] [] $($parameters)*),
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
    ($index:ident $try_:tt ($new:expr) [$($parameter:ty,)*] [$($info:expr,)*]) => {
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
            emit: $crate::__ferro_compiler_metadata!(@value ::std::option::Option::Some($crate::metadata::MarkupEmit {
                function: ::std::concat!("__markup_new", ::std::stringify!($index)),
                fallible: $crate::__ferro_markup_is_try!($try_),
            })),
        }
    };
    ($index:ident $try_:tt $new:tt [$($parameter:ty,)*] [$($info:expr,)*]
        $name:ident : $type_:ty [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor!($index $try_ $new [$($parameter,)* $type_,] [$($info,)*
            $crate::metadata::MarkupParameter {
                name: ::std::option::Option::Some(::std::stringify!($name)),
                attributes: $crate::__ferro_markup_attributes!([] $($attributes)*),
            },
        ] $($($rest)*)?)
    };
    ($index:ident $try_:tt $new:tt [$($parameter:ty,)*] [$($info:expr,)*]
        $name:ident : $type_:ty $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor!($index $try_ $new [$($parameter,)* $type_,] [$($info,)*
            $crate::metadata::MarkupParameter {
                name: ::std::option::Option::Some(::std::stringify!($name)),
                attributes: &[],
            },
        ] $($($rest)*)?)
    };
    // The positional form: all parameters are types.
    ($index:ident $try_:tt $new:tt [] [] $($type_:ty),+ $(,)?) => {
        $crate::__ferro_markup_constructor!($index $try_ $new [$($type_,)+] [])
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
    ([$($pool:ident)*] $this:ty; [$($done:expr,)*]) => {
        &[$($done),*]
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [try] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [try] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [try] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [static] [] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [try] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [try] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [try] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [] $name ($($parameter),*) ($($return_)?) ($call $(:: $segment)*) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [] $name ($($parameter),*) ($($return_)?) (($($call)*)) [$($attributes)*]
        ),] $($($rest)*)?)
    };

    ([$index:ident $($pool:ident)*] $this:ty; [$($done:expr,)*]
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_methods!([$($pool)*] $this; [$($done,)* $crate::__ferro_markup_method!(
            $index $this; [] [] $name ($($parameter),*) ($($return_)?) ($call) []
        ),] $($($rest)*)?)
    };
}

/// The typed functions of `methods: [..]` (see `MarkupEmit`), in declaration order: the
/// same order and pool of indices as [`__ferro_markup_methods!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_method_fns {
    ([$($pool:ident)*] $this:ty;) => {};

    ([$index:ident $($pool:ident)*] $this:ty;
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [try] $this; $name ($($return_)?) ($call $(:: $segment)*) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [try] $this; $name ($($return_)?) (($($call)*)) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        static try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [try] $this; $name ($($return_)?) ($call) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [] $this; $name ($($return_)?) ($call $(:: $segment)*) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [] $this; $name ($($return_)?) (($($call)*)) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        static fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] [] $this; $name ($($return_)?) ($call) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [try] $this; $name ($($return_)?) ($call $(:: $segment)*) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [try] $this; $name ($($return_)?) (($($call)*)) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        try fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [try] $this; $name ($($return_)?) ($call) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:ident $(:: $segment:ident)* [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [] $this; $name ($($return_)?) ($call $(:: $segment)*) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => ($($call:tt)*) [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [] $this; $name ($($return_)?) (($($call)*)) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };

    ([$index:ident $($pool:ident)*] $this:ty;
        fn $name:ident ($($parameter:ty),* $(,)?) $(-> $return_:ty)? => $call:expr  $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_method_fn!(
            $index [] [] $this; $name ($($return_)?) ($call) [] [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
        $crate::__ferro_markup_method_fns!([$($pool)*] $this; $($($rest)*)?);
    };
}

/// A pool of indices for the members of one declaration that have no
/// unique name (overloaded methods): the data of the metadata and the typed
/// functions both take the indices from here, in declaration order.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_pool {
    (methods { $($arguments:tt)* }) => {
        $crate::__ferro_markup_methods!([_0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _10 _11 _12 _13 _14 _15 _16 _17 _18 _19 _20 _21 _22 _23 _24 _25 _26 _27 _28 _29 _30 _31 _32 _33 _34 _35 _36 _37 _38 _39 _40 _41 _42 _43 _44 _45 _46 _47 _48 _49 _50 _51 _52 _53 _54 _55 _56 _57 _58 _59 _60 _61 _62 _63 _64 _65 _66 _67 _68 _69 _70 _71 _72 _73 _74 _75 _76 _77 _78 _79 _80 _81 _82 _83 _84 _85 _86 _87 _88 _89 _90 _91 _92 _93 _94 _95 _96 _97 _98 _99 _100 _101 _102 _103 _104 _105 _106 _107 _108 _109 _110 _111 _112 _113 _114 _115 _116 _117 _118 _119 _120 _121 _122 _123 _124 _125 _126 _127 _128 _129 _130 _131 _132 _133 _134 _135 _136 _137 _138 _139 _140 _141 _142 _143 _144 _145 _146 _147 _148 _149 _150 _151 _152 _153 _154 _155 _156 _157 _158 _159] $($arguments)*)
    };
    (method_fns { $($arguments:tt)* }) => {
        $crate::__ferro_markup_method_fns!([_0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _10 _11 _12 _13 _14 _15 _16 _17 _18 _19 _20 _21 _22 _23 _24 _25 _26 _27 _28 _29 _30 _31 _32 _33 _34 _35 _36 _37 _38 _39 _40 _41 _42 _43 _44 _45 _46 _47 _48 _49 _50 _51 _52 _53 _54 _55 _56 _57 _58 _59 _60 _61 _62 _63 _64 _65 _66 _67 _68 _69 _70 _71 _72 _73 _74 _75 _76 _77 _78 _79 _80 _81 _82 _83 _84 _85 _86 _87 _88 _89 _90 _91 _92 _93 _94 _95 _96 _97 _98 _99 _100 _101 _102 _103 _104 _105 _106 _107 _108 _109 _110 _111 _112 _113 _114 _115 _116 _117 _118 _119 _120 _121 _122 _123 _124 _125 _126 _127 _128 _129 _130 _131 _132 _133 _134 _135 _136 _137 _138 _139 _140 _141 _142 _143 _144 _145 _146 _147 _148 _149 _150 _151 _152 _153 _154 _155 _156 _157 _158 _159] $($arguments)*);
    };
    (constructors { $($arguments:tt)* }) => {
        $crate::__ferro_markup_constructors!([_0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _10 _11 _12 _13 _14 _15 _16 _17 _18 _19 _20 _21 _22 _23 _24 _25 _26 _27 _28 _29 _30 _31 _32 _33 _34 _35 _36 _37 _38 _39 _40 _41 _42 _43 _44 _45 _46 _47 _48 _49 _50 _51 _52 _53 _54 _55 _56 _57 _58 _59 _60 _61 _62 _63 _64 _65 _66 _67 _68 _69 _70 _71 _72 _73 _74 _75 _76 _77 _78 _79 _80 _81 _82 _83 _84 _85 _86 _87 _88 _89 _90 _91 _92 _93 _94 _95 _96 _97 _98 _99 _100 _101 _102 _103 _104 _105 _106 _107 _108 _109 _110 _111 _112 _113 _114 _115 _116 _117 _118 _119 _120 _121 _122 _123 _124 _125 _126 _127 _128 _129 _130 _131 _132 _133 _134 _135 _136 _137 _138 _139 _140 _141 _142 _143 _144 _145 _146 _147 _148 _149 _150 _151 _152 _153 _154 _155 _156 _157 _158 _159] $($arguments)*)
    };
    (constructor_fns { $($arguments:tt)* }) => {
        $crate::__ferro_markup_constructor_fns!([_0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _10 _11 _12 _13 _14 _15 _16 _17 _18 _19 _20 _21 _22 _23 _24 _25 _26 _27 _28 _29 _30 _31 _32 _33 _34 _35 _36 _37 _38 _39 _40 _41 _42 _43 _44 _45 _46 _47 _48 _49 _50 _51 _52 _53 _54 _55 _56 _57 _58 _59 _60 _61 _62 _63 _64 _65 _66 _67 _68 _69 _70 _71 _72 _73 _74 _75 _76 _77 _78 _79 _80 _81 _82 _83 _84 _85 _86 _87 _88 _89 _90 _91 _92 _93 _94 _95 _96 _97 _98 _99 _100 _101 _102 _103 _104 _105 _106 _107 _108 _109 _110 _111 _112 _113 _114 _115 _116 _117 _118 _119 _120 _121 _122 _123 _124 _125 _126 _127 _128 _129 _130 _131 _132 _133 _134 _135 _136 _137 _138 _139 _140 _141 _142 _143 _144 _145 _146 _147 _148 _149 _150 _151 _152 _153 _154 _155 _156 _157 _158 _159] $($arguments)*);
    };
}

/// Whether a member is declared with `try`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_is_try {
    ([]) => {
        false
    };
    ([try]) => {
        true
    };
}

/// The `MarkupEmit` of an accessor of a property: `get` / `set` /
/// `static_get` / `static_set`, the name, the accessors.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_emit_accessor {
    ($which:ident $name:ident;) => {
        ::std::option::Option::None
    };
    (get $name:ident; get: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_get_", $name, false)
    };
    (get $name:ident; try_get: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_get_", $name, true)
    };
    (set $name:ident; set: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_set_", $name, false)
    };
    (set $name:ident; try_set: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_set_", $name, true)
    };
    (static_get $name:ident; get: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_static_get_", $name, false)
    };
    (static_get $name:ident; try_get: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_static_get_", $name, true)
    };
    (static_set $name:ident; set: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_static_set_", $name, false)
    };
    (static_set $name:ident; try_set: $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!(@emit "__markup_static_set_", $name, true)
    };
    (@emit $prefix:literal, $name:ident, $fallible:literal) => {
        ::std::option::Option::Some($crate::metadata::MarkupEmit {
            function: ::std::concat!($prefix, ::std::stringify!($name)),
            fallible: $fallible,
        })
    };
    ($which:ident $name:ident; $other:ident : $accessor:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_emit_accessor!($which $name; $($($rest)*)?)
    };
}

/// The typed functions of a declaration (see `MarkupEmit`), as associated
/// functions of the declared type: written into an `impl` block of the type
/// by [`ferro_markup_type!`](crate::ferro_markup_type) and
/// [`ferro_class_info!`](crate::ferro_class_info). `this, value; parts..`:
/// the instance type, the type of a value of the type (what `Parse`
/// returns: the type itself, `Rc<dyn Trait>` for a contract) and the parts
/// of the declaration body, as `__ferro_markup_items!` reads them.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_fns {
    ($this:ty, $value:ty;) => {};

    ($this:ty, $value:ty; properties: [
        $($name:ident : $type_:ty { $($accessors:tt)* } $([$($attributes:tt)*])?),* $(,)?
    ] $(, $($rest:tt)*)?) => {
        $($crate::__ferro_markup_property_fns!($this, $type_, $name; $($accessors)*);)*
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };

    ($this:ty, $value:ty; static_properties: [
        $($name:ident : $type_:ty { $($accessors:tt)* } $([$($attributes:tt)*])?),* $(,)?
    ] $(, $($rest:tt)*)?) => {
        $($crate::__ferro_markup_static_property_fns!($type_, $name; $($accessors)*);)*
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };

    ($this:ty, $value:ty; methods: [$($methods:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_pool!(method_fns { $this; $($methods)* });
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };

    // The other parts declare no typed function.
    ($this:ty, $value:ty; namespace: $namespace:literal $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; handles: [$($handle:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; base: $base:ty $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; interfaces: [$($interface:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; generic: $definition:literal [$($argument:ty),* $(,)?] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; content: $content:ident $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; attributes: [$($attributes:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; constructors: [$($constructors:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_pool!(constructor_fns { $value; $($constructors)* });
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; parse: $parse:expr $(, $($rest:tt)*)?) => {
        /// The typed function of the `Parse(string)` of the type (see `MarkupEmit`).
        #[doc(hidden)]
        #[allow(private_interfaces, clippy::all)]
        #[inline]
        pub fn __markup_parse(text: ::std::string::String) -> ::std::result::Result<$value, $crate::metadata::MarkupInvokeError> {
            $crate::metadata::markup_result(($parse)(text.as_str()))
        }
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; indexers: [$($indexers:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; property_attributes: [$($attributes:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; notify_property_changed: $object:ty $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; type_info: $type_info:ty $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; fields: [$($fields:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
    ($this:ty, $value:ty; events: [$($events:tt)*] $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_fns!($this, $value; $($($rest)*)?);
    };
}

/// The typed accessor functions of an instance property.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_property_fns {
    ($this:ty, $type_:ty, $name:ident;) => {};
    ($this:ty, $type_:ty, $name:ident; get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_get_ $name>](this: &$this) -> $type_ {
                ($get)(this)
            }
        }
        $crate::__ferro_markup_property_fns!($this, $type_, $name; $($($rest)*)?);
    };
    ($this:ty, $type_:ty, $name:ident; try_get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_get_ $name>](
                this: &$this,
            ) -> ::std::result::Result<$type_, $crate::metadata::MarkupInvokeError> {
                $crate::metadata::markup_result(($get)(this))
            }
        }
        $crate::__ferro_markup_property_fns!($this, $type_, $name; $($($rest)*)?);
    };
    ($this:ty, $type_:ty, $name:ident; set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_set_ $name>](this: &$this, value: $type_) {
                let _ = ($set)(this, value);
            }
        }
        $crate::__ferro_markup_property_fns!($this, $type_, $name; $($($rest)*)?);
    };
    ($this:ty, $type_:ty, $name:ident; try_set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_set_ $name>](
                this: &$this,
                value: $type_,
            ) -> ::std::result::Result<(), $crate::metadata::MarkupInvokeError> {
                $crate::metadata::markup_result(($set)(this, value)).map(|_| ())
            }
        }
        $crate::__ferro_markup_property_fns!($this, $type_, $name; $($($rest)*)?);
    };
}

/// The typed accessor functions of a static property.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_static_property_fns {
    ($type_:ty, $name:ident;) => {};
    ($type_:ty, $name:ident; get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_static_get_ $name>]() -> $type_ {
                ($get)()
            }
        }
        $crate::__ferro_markup_static_property_fns!($type_, $name; $($($rest)*)?);
    };
    ($type_:ty, $name:ident; try_get: $get:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_static_get_ $name>]() -> ::std::result::Result<$type_, $crate::metadata::MarkupInvokeError> {
                $crate::metadata::markup_result(($get)())
            }
        }
        $crate::__ferro_markup_static_property_fns!($type_, $name; $($($rest)*)?);
    };
    ($type_:ty, $name:ident; set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_static_set_ $name>](value: $type_) {
                let _ = ($set)(value);
            }
        }
        $crate::__ferro_markup_static_property_fns!($type_, $name; $($($rest)*)?);
    };
    ($type_:ty, $name:ident; try_set: $set:expr $(, $($rest:tt)*)?) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_static_set_ $name>](
                value: $type_,
            ) -> ::std::result::Result<(), $crate::metadata::MarkupInvokeError> {
                $crate::metadata::markup_result(($set)(value)).map(|_| ())
            }
        }
        $crate::__ferro_markup_static_property_fns!($type_, $name; $($($rest)*)?);
    };
}

/// The typed function of one method: `index [static]? [try]? this; Name
/// (return)? (callable) [arguments] [names] (parameters)`: the parameters are
/// paired with names from the pool of names, then the function is written.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_method_fn {
    ($index:ident $static_:tt $try_:tt $this:ty; $name:ident ($($return_:ty)?) ($call:expr)
        [$($argument:ident : $parameter:ty,)*] [$($names:ident)*] ()
    ) => {
        $crate::__ferro_markup_method_fn!(@emit $index $static_ $try_ $this; $name ($($return_)?) ($call)
            [$($argument : $parameter,)*]);
    };
    ($index:ident $static_:tt $try_:tt $this:ty; $name:ident ($($return_:ty)?) ($call:expr)
        [$($done:tt)*] [$next:ident $($names:ident)*] ($parameter:ty $(, $rest:ty)*)
    ) => {
        $crate::__ferro_markup_method_fn!($index $static_ $try_ $this; $name ($($return_)?) ($call)
            [$($done)* $next : $parameter,] [$($names)*] ($($rest),*));
    };
    (@emit $index:ident [] $try_:tt $this:ty; $name:ident ($($return_:ty)?) ($call:expr)
        [$($argument:ident : $parameter:ty,)*]
    ) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_ $name $index>](
                this: &$this $(, $argument: $parameter)*
            ) -> $crate::__ferro_markup_fn_return!($try_ ($($return_)?)) {
                $crate::__ferro_markup_fn_body!($try_ ($($return_)?) ($call)(this $(, $argument)*))
            }
        }
    };
    (@emit $index:ident [static] $try_:tt $this:ty; $name:ident ($($return_:ty)?) ($call:expr)
        [$($argument:ident : $parameter:ty,)*]
    ) => {
        $crate::__paste! {
            #[doc(hidden)]
            #[allow(non_snake_case, private_interfaces, clippy::all)]
            #[inline]
            pub fn [<__markup_ $name $index>](
                $($argument: $parameter),*
            ) -> $crate::__ferro_markup_fn_return!($try_ ($($return_)?)) {
                $crate::__ferro_markup_fn_body!($try_ ($($return_)?) ($call)($($argument),*))
            }
        }
    };
}

/// The typed functions of `constructors: [..]` (see `MarkupEmit`), in
/// declaration order: the same order and pool of indices as
/// [`__ferro_markup_constructors!`]. `value` is the type a constructor returns.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_constructor_fns {
    ([$($pool:ident)*] $value:ty;) => {};
    ([$index:ident $($pool:ident)*] $value:ty; ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructor_fn!($index [] $value; ($new) [] $($parameters)*);
        $crate::__ferro_markup_constructor_fns!([$($pool)*] $value; $($($rest)*)?);
    };
    ([$index:ident $($pool:ident)*] $value:ty; try ($($parameters:tt)*) => $new:expr $(, $($rest:tt)*)?) => {
        $crate::__ferro_markup_constructor_fn!($index [try] $value; ($new) [] $($parameters)*);
        $crate::__ferro_markup_constructor_fns!([$($pool)*] $value; $($($rest)*)?);
    };
}

/// The typed function of one constructor: its parameter types are
/// collected from the positional or the named form, then the function is
/// written as a static method `new` returning `value`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_constructor_fn {
    ($index:ident $try_:tt $value:ty; ($new:expr) [$($parameter:ty,)*]) => {
        $crate::__ferro_markup_method_fn!(
            $index [static] $try_ $value; new ($value) ($new) []
            [a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15] ($($parameter),*)
        );
    };
    ($index:ident $try_:tt $value:ty; $new:tt [$($parameter:ty,)*]
        $name:ident : $type_:ty [$($attributes:tt)*] $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor_fn!($index $try_ $value; $new [$($parameter,)* $type_,] $($($rest)*)?);
    };
    ($index:ident $try_:tt $value:ty; $new:tt [$($parameter:ty,)*]
        $name:ident : $type_:ty $(, $($rest:tt)*)?
    ) => {
        $crate::__ferro_markup_constructor_fn!($index $try_ $value; $new [$($parameter,)* $type_,] $($($rest)*)?);
    };
    ($index:ident $try_:tt $value:ty; $new:tt [] $($type_:ty),+ $(,)?) => {
        $crate::__ferro_markup_constructor_fn!($index $try_ $value; $new [$($type_,)+]);
    };
}

/// The return type of a typed function: `[try]? (return)?`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_fn_return {
    ([] ()) => { () };
    ([] ($return_:ty)) => { $return_ };
    ([try] ()) => { ::std::result::Result<(), $crate::metadata::MarkupInvokeError> };
    ([try] ($return_:ty)) => { ::std::result::Result<$return_, $crate::metadata::MarkupInvokeError> };
}

/// The body of a typed function: `[try]? (return)? call`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_fn_body {
    ([] () $($call:tt)*) => {{
        let _ = $($call)*;
    }};
    ([] ($return_:ty) $($call:tt)*) => {
        $($call)*
    };
    ([try] () $($call:tt)*) => {
        $crate::metadata::markup_result($($call)*).map(|_| ())
    };
    ([try] ($return_:ty) $($call:tt)*) => {
        $crate::metadata::markup_result($($call)*)
    };
}

/// One `MarkupMethod`: `this; [static]? [try]? Name (parameters) (return)? (callable) [attributes]`.
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_markup_method {
    ($index:ident $this:ty; [] $try_:tt $name:ident ($($parameter:ty),*) ($($return_:ty)?) ($call:expr) [$($attributes:tt)*]) => {
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
            emit: $crate::__ferro_compiler_metadata!(@value ::std::option::Option::Some($crate::metadata::MarkupEmit {
                function: ::std::concat!("__markup_", ::std::stringify!($name), ::std::stringify!($index)),
                fallible: $crate::__ferro_markup_is_try!($try_),
            })),
        }
    };
    ($index:ident $this:ty; [static] $try_:tt $name:ident ($($parameter:ty),*) ($($return_:ty)?) ($call:expr) [$($attributes:tt)*]) => {
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
            emit: $crate::__ferro_compiler_metadata!(@value ::std::option::Option::Some($crate::metadata::MarkupEmit {
                function: ::std::concat!("__markup_", ::std::stringify!($name), ::std::stringify!($index)),
                fallible: $crate::__ferro_markup_is_try!($try_),
            })),
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

/// The table of public Rust paths of the registered types of a crate, from
/// its generated `rust_paths.rs` (`scripts/rust_paths.py`, called by
/// `scripts/generate_markup_types.py`).
///
/// ```ignore
/// ferro_rust_paths! {
///     classes: [crate::Border, crate::primitives::Popup],
///     types: [crate::Dock, crate::GridLength],
///     contracts: [crate::templates::IDataTemplate],
/// }
/// ```
///
/// Every entry is a crate-relative public path; an instantiation of a
/// generic type (`generics`, `generic_contracts` for a trait) is the type
/// with every path absolute, and the path text the generator rendered for
/// it (checked from outside the crate by the XAML test crate). It defines
/// `CLASS_RUST_PATHS: &[(&TypeInfo, &str)]` (the runtime type of each
/// class) and `MARKUP_RUST_PATHS: &[(&MarkupType, &str, bool)]` (the
/// metadata of each type, of `dyn Trait` for a contract, and whether the
/// path names a trait), each with the path as another crate names it
/// (`crate` replaced by the name of the crate), and `TYPE_RUST_PATHS: &[(fn() -> TypeId, &str)]`
/// (the non-generic types by the `TypeId` of the type itself, for
/// [`register_type_rust_paths`](super::register_type_rust_paths)). The item and
/// the text come from the same tokens, so the text names exactly the
/// registered type. The crate passes both tables to
/// [`TypeInfo::register_rust_paths`](crate::TypeInfo::register_rust_paths)
/// and [`MarkupType::register_rust_paths`](super::MarkupType::register_rust_paths).
#[cfg(feature = "compiler-metadata")]
#[macro_export]
macro_rules! ferro_rust_paths {
    (
        classes: [$(crate $(:: $class:ident)+),* $(,)?],
        types: [$(crate $(:: $type_:ident)+),* $(,)?],
        contracts: [$(crate $(:: $contract:ident)+),* $(,)?],
        generics: [$(($generic:ty, $generic_path:literal)),* $(,)?],
        generic_contracts: [$(($generic_contract:path, $generic_contract_path:literal)),* $(,)?] $(,)?
    ) => {
        /// The public Rust paths of the classes of the type table.
        pub(crate) const CLASS_RUST_PATHS: &[(&'static $crate::TypeInfo, &'static str)] = &[$((
            <crate $(:: $class)+ as $crate::StaticType>::TYPE,
            ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($class))+),
        )),*];

        /// The public Rust paths of the types, by the `TypeId` of the type itself.
        pub(crate) const TYPE_RUST_PATHS: &[(fn() -> ::std::any::TypeId, &'static str)] = &[
            $((
                || ::std::any::TypeId::of::<crate $(:: $class)+>(),
                ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($class))+),
            ),)*
            $((
                || ::std::any::TypeId::of::<crate $(:: $type_)+>(),
                ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($type_))+),
            ),)*
            $((
                || ::std::any::TypeId::of::<dyn crate $(:: $contract)+>(),
                ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($contract))+),
            ),)*
        ];

        /// The public Rust paths of the markup types of the type lists.
        pub(crate) const MARKUP_RUST_PATHS: &[(&'static $crate::metadata::MarkupType, &'static str, bool)] = &[
            $((
                <crate $(:: $type_)+ as $crate::metadata::MarkupTyped>::MARKUP,
                ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($type_))+),
                false,
            ),)*
            $((
                <dyn crate $(:: $contract)+ as $crate::metadata::MarkupTyped>::MARKUP,
                ::std::concat!(::std::env!("CARGO_CRATE_NAME") $(, "::", ::std::stringify!($contract))+),
                true,
            ),)*
            $((<$generic as $crate::metadata::MarkupTyped>::MARKUP, $generic_path, false),)*
            $((<dyn $generic_contract as $crate::metadata::MarkupTyped>::MARKUP, $generic_contract_path, true),)*
        ];

        /// Records the public Rust paths of this crate for the emitter of Rust source.
        pub(crate) fn register_rust_paths() {
            $crate::TypeInfo::register_rust_paths(CLASS_RUST_PATHS);
            $crate::metadata::register_type_rust_paths(TYPE_RUST_PATHS);
            $crate::metadata::MarkupType::register_rust_paths(MARKUP_RUST_PATHS);
        }
    };
}

/// Without the `compiler-metadata` feature: `register_rust_paths()` records
/// nothing and the tables do not exist (see the other definition).
#[cfg(not(feature = "compiler-metadata"))]
#[macro_export]
macro_rules! ferro_rust_paths {
    ($($tokens:tt)*) => {
        /// Records the public Rust paths of this crate: nothing without the
        /// `compiler-metadata` feature of the base crate.
        pub(crate) fn register_rust_paths() {}
    };
}

/// The tokens with the `compiler-metadata` feature of this crate, nothing
/// without it: what only the emitter of Rust source uses (the typed
/// functions of declared members, the metadata that names them). `@value
/// expr` is the value of a [`CompilerMetadata`](crate::metadata::CompilerMetadata)
/// slot: `expr` with the feature, the zero-sized stand-in without it.
#[cfg(feature = "compiler-metadata")]
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_compiler_metadata {
    (@value $value:expr) => {
        $value
    };
    ($($tokens:tt)*) => {
        $($tokens)*
    };
}

/// Without the `compiler-metadata` feature: nothing (see the other
/// definition).
#[cfg(not(feature = "compiler-metadata"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __ferro_compiler_metadata {
    (@value $value:expr) => {
        $crate::metadata::NotRecorded::new()
    };
    ($($tokens:tt)*) => {};
}
