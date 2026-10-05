//! Markup metadata: what markup (and other untyped code) needs to know
//! about a type and cannot derive from the property system.
//!
//! The managed original answers these questions with reflection: which
//! property holds the content of an element, which plain (non-registered)
//! properties, collections and events a type has, how to construct it with
//! arguments, how to parse it from text, which attributes it carries. Here a
//! type states them once, as constant data, with the declaration forms of
//! [`ferro_class_info!`](crate::ferro_class_info) (classes),
//! [`ferro_markup_type!`](crate::ferro_markup_type) (every other type) and
//! [`ferro_markup_enum!`](crate::ferro_markup_enum) (enumerations).
//!
//! Everything in this module is plain `'static` data and function pointers:
//! declaring metadata costs nothing at run time until something reads it,
//! and nothing is registered before `main`.
//!
//! # Values
//!
//! Members are invoked with untyped values ([`MarkupValue`]): `None` is
//! null, `Some(box)` holds a value in its untyped form, the form
//! [`ValueTypes`] produces for bindings (the contents of a nullable, the
//! object itself for a reference type, the handle for a class instance).
//! An invoker converts each argument to the declared Rust parameter type
//! with the assignability casts of [`ValueTypes`] (identity, base class and
//! interface handles, wrapping into `Option<T>`, boxing into the "any value"
//! type); conversions that change the value (parsing, numeric conversion)
//! are the caller's business.

use crate::data::core::{ValueType, ValueTypes};
use crate::{BoxedValue, TypeInfo};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::{OnceLock, RwLock};

/// Names a Rust type lazily. Type identities cannot be computed in constant
/// data, so metadata stores a function that returns them.
pub type TypeOf = fn() -> ValueType;

/// An untyped value passed to and returned from an invoker: `None` is null.
pub type MarkupValue = Option<BoxedValue>;

/// An untyped member: a constructor, method, property accessor or event
/// subscription. For instance members the first argument is the instance.
pub type MarkupInvoke = fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError>;

/// Why an untyped member could not be invoked.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkupInvokeError {
    /// The member was called with the wrong number of arguments (including
    /// the instance).
    ArgumentCount { expected: usize, actual: usize },
    /// An argument is not assignable to the declared parameter type.
    Argument { index: usize, expected: &'static str, actual: String },
    /// The member itself failed (the equivalent of an exception thrown by
    /// the member, for example a parse error).
    Failed(String),
}

impl fmt::Display for MarkupInvokeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArgumentCount { expected, actual } => {
                write!(f, "Expected {expected} argument(s), got {actual}.")
            }
            Self::Argument { index, expected, actual } => {
                write!(f, "Argument {index}: a value of type '{actual}' is not assignable to '{expected}'.")
            }
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for MarkupInvokeError {}

/// The declared method a [`MarkupDelegate`] was created from: what the
/// `Target` and `Method` of a delegate are in the managed original.
struct MarkupDelegateMethod {
    target: MarkupValue,
    declaring_type: &'static MarkupType,
    method: &'static MarkupMethod,
}

/// An untyped callback (the equivalent of a delegate instance): what an
/// event subscription declared in metadata receives as its handler, and what
/// a binding to a method produces.
#[derive(Clone)]
pub struct MarkupDelegate {
    call: Rc<dyn Fn(&[MarkupValue]) -> MarkupValue>,
    method: Option<Rc<MarkupDelegateMethod>>,
}

impl MarkupDelegate {
    pub fn new(f: impl Fn(&[MarkupValue]) -> MarkupValue + 'static) -> Self {
        Self { call: Rc::new(f), method: None }
    }

    /// The delegate of a method declared in metadata, bound to `target`
    /// (`method.CreateDelegate(type, target)` in the managed original).
    /// `target` is the instance for an instance method and ignored for a
    /// static one; `declaring_type` is the type whose metadata lists
    /// `method`. The delegate keeps the target alive.
    ///
    /// [`invoke`](Self::invoke) calls the method with the arguments given
    /// (after the instance) and discards a failure;
    /// [`try_invoke`](Self::try_invoke) reports it.
    pub fn for_method(target: MarkupValue, declaring_type: &'static MarkupType, method: &'static MarkupMethod) -> Self {
        let target = if method.is_static { None } else { target };
        let bound = target.clone();
        Self {
            call: Rc::new(move |arguments| invoke_method(method, &bound, arguments).ok().flatten()),
            method: Some(Rc::new(MarkupDelegateMethod { target, declaring_type, method })),
        }
    }

    /// Calls the callback.
    pub fn invoke(&self, arguments: &[MarkupValue]) -> MarkupValue {
        (self.call)(arguments)
    }

    /// Calls the callback. For the delegate of a declared method
    /// ([`for_method`](Self::for_method)) a failure of the call (wrong
    /// arguments, a failing member) is returned.
    pub fn try_invoke(&self, arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
        match &self.method {
            Some(bound) => invoke_method(bound.method, &bound.target, arguments),
            None => Ok((self.call)(arguments)),
        }
    }

    /// The instance the delegate of a declared instance method is bound to
    /// (the `Target` of the managed original). `None` for a static method
    /// and for a delegate that was not created from a declared method.
    pub fn target(&self) -> MarkupValue {
        self.method.as_ref().and_then(|bound| bound.target.clone())
    }

    /// The declared method the delegate was created from (the `Method` of
    /// the managed original), if it was created with
    /// [`for_method`](Self::for_method).
    pub fn method(&self) -> Option<&'static MarkupMethod> {
        self.method.as_ref().map(|bound| bound.method)
    }

    /// The type whose metadata declares [`method`](Self::method).
    pub fn declaring_type(&self) -> Option<&'static MarkupType> {
        self.method.as_ref().map(|bound| bound.declaring_type)
    }
}

fn invoke_method(
    method: &'static MarkupMethod,
    target: &MarkupValue,
    arguments: &[MarkupValue],
) -> Result<MarkupValue, MarkupInvokeError> {
    if method.is_static {
        return (method.invoke)(arguments);
    }
    let mut all = Vec::with_capacity(arguments.len() + 1);
    all.push(target.clone());
    all.extend(arguments.iter().cloned());
    (method.invoke)(&all)
}

impl PartialEq for MarkupDelegate {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.call, &other.call)
    }
}

impl fmt::Debug for MarkupDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MarkupDelegate")
    }
}

/// The name of the delegate type, as the managed original prints a delegate
/// (`System.Action`, ``System.Action`1[System.Int32]``,
/// ``System.Func`2[System.Object,System.Boolean]``) for the delegate of a
/// declared method; `System.Delegate` otherwise.
impl fmt::Display for MarkupDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(method) = self.method() else {
            return f.write_str("System.Delegate");
        };
        let mut types: Vec<String> =
            method.parameters.iter().map(|parameter| ValueTypes::type_full_name(parameter())).collect();
        let definition = match method.return_type {
            Some(return_type) => {
                types.push(ValueTypes::type_full_name(return_type()));
                "System.Func"
            }
            None => "System.Action",
        };
        match types.len() {
            0 => f.write_str(definition),
            count => write!(f, "{definition}`{count}[{}]", types.join(",")),
        }
    }
}

/// Converts a typed value to its untyped form: null for `()` and for an
/// empty nullable, the contents of a nullable, the object itself for a
/// reference type, the value unchanged if it is already untyped.
pub fn into_markup_value<T: PartialEq + 'static>(value: T) -> MarkupValue {
    {
        let any: &dyn Any = &value;
        if let Some(untyped) = any.downcast_ref::<BoxedValue>() {
            return Some(untyped.clone());
        }
        if let Some(untyped) = any.downcast_ref::<Option<BoxedValue>>() {
            return untyped.clone();
        }
        if any.is::<()>() {
            return None;
        }
    }
    let boxed: BoxedValue = Rc::new(value);
    match ValueTypes::try_cast(&boxed, ValueType::of::<Option<BoxedValue>>()) {
        Some(untyped) => untyped.downcast_ref::<Option<BoxedValue>>().cloned().flatten(),
        None => Some(boxed),
    }
}

/// Converts an untyped value to the Rust type `T` with the assignability
/// casts of [`ValueTypes`]. `None` if the value is not assignable to `T`.
pub fn from_markup_value<T: Clone + 'static>(value: &MarkupValue) -> Option<T> {
    let target = TypeId::of::<T>();
    if target == TypeId::of::<Option<BoxedValue>>() {
        let any: &dyn Any = value;
        return any.downcast_ref::<T>().cloned();
    }
    match value {
        Some(boxed) => {
            if let Some(exact) = boxed.downcast_ref::<T>() {
                return Some(exact.clone());
            }
            if target == TypeId::of::<BoxedValue>() {
                let any: &dyn Any = boxed;
                return any.downcast_ref::<T>().cloned();
            }
            let cast = ValueTypes::try_cast(boxed, ValueType::of::<T>())?;
            cast.downcast_ref::<T>().cloned()
        }
        None => {
            let null = ValueTypes::try_convert(None, ValueType::of::<T>())??;
            null.downcast_ref::<T>().cloned()
        }
    }
}

/// Maps the error of a fallible member to [`MarkupInvokeError::Failed`]; see
/// the `try` forms of [`ferro_markup_type!`](crate::ferro_markup_type).
pub fn markup_result<T, E: fmt::Display>(result: Result<T, E>) -> Result<T, MarkupInvokeError> {
    result.map_err(|error| MarkupInvokeError::Failed(error.to_string()))
}

/// The arguments of an untyped call, read in order by a generated invoker.
pub struct MarkupArguments<'a> {
    arguments: &'a [MarkupValue],
    next: usize,
}

impl<'a> MarkupArguments<'a> {
    /// Checks the number of arguments.
    pub fn new(arguments: &'a [MarkupValue], expected: usize) -> Result<Self, MarkupInvokeError> {
        if arguments.len() != expected {
            return Err(MarkupInvokeError::ArgumentCount { expected, actual: arguments.len() });
        }
        Ok(Self { arguments, next: 0 })
    }

    /// The next argument, as a `T`.
    pub fn next<T: Clone + 'static>(&mut self) -> Result<T, MarkupInvokeError> {
        let index = self.next;
        self.next += 1;
        let value = &self.arguments[index];
        from_markup_value::<T>(value).ok_or_else(|| MarkupInvokeError::Argument {
            index,
            expected: std::any::type_name::<T>(),
            actual: match value {
                Some(value) => value.type_name().to_string(),
                None => "null".to_string(),
            },
        })
    }
}

/// What kind of type a [`MarkupType`] describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkupTypeKind {
    /// A reference type: a class of the object model or a plain shared type.
    Class,
    /// A value type.
    Struct,
    /// An enumeration; [`MarkupType::enum_members`] lists its members.
    Enum,
    /// A contract (a trait used through a handle).
    Interface,
    /// A type that cannot be instantiated and only owns static members.
    Static,
}

/// A constant argument of a [`MarkupAttribute`].
#[derive(Clone, Copy)]
pub enum MarkupAttributeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'static str),
    Type(TypeOf),
    /// An array argument (`new[] { ",", " " }` of the managed original).
    Array(&'static [MarkupAttributeValue]),
}

impl fmt::Debug for MarkupAttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value:?}"),
            Self::Str(value) => write!(f, "{value:?}"),
            Self::Type(type_) => write!(f, "type({})", type_()),
            Self::Array(items) => f.debug_list().entries(items.iter()).finish(),
        }
    }
}

impl PartialEq for MarkupAttributeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Type(a), Self::Type(b)) => a() == b(),
            (Self::Array(a), Self::Array(b)) => a == b,
            _ => false,
        }
    }
}

/// Turns a literal of an attribute declaration into a
/// [`MarkupAttributeValue`] in constant context; see
/// [`ferro_markup_type!`](crate::ferro_markup_type).
#[doc(hidden)]
pub struct MarkupLiteral<T>(pub T);

impl MarkupLiteral<&'static str> {
    pub const fn value(self) -> MarkupAttributeValue {
        MarkupAttributeValue::Str(self.0)
    }
}

impl MarkupLiteral<bool> {
    pub const fn value(self) -> MarkupAttributeValue {
        MarkupAttributeValue::Bool(self.0)
    }
}

impl MarkupLiteral<i64> {
    pub const fn value(self) -> MarkupAttributeValue {
        MarkupAttributeValue::Int(self.0)
    }
}

impl MarkupLiteral<f64> {
    pub const fn value(self) -> MarkupAttributeValue {
        MarkupAttributeValue::Float(self.0)
    }
}

impl MarkupLiteral<char> {
    pub const fn value(self) -> MarkupAttributeValue {
        MarkupAttributeValue::Int(self.0 as i64)
    }
}

/// An attribute of a type or member: the data of a custom attribute of the
/// managed original (`[DependsOn("Property")]`,
/// `[TemplateContent(TemplateResultType = typeof(..))]`).
///
/// `name` is the attribute name without the `Attribute` suffix; the names of
/// the attributes the framework itself understands are the constants of
/// [`attributes`](super::attributes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkupAttribute {
    pub name: &'static str,
    /// The constructor arguments.
    pub arguments: &'static [MarkupAttributeValue],
    /// The named arguments.
    pub properties: &'static [(&'static str, MarkupAttributeValue)],
}

impl MarkupAttribute {
    /// The named argument `name`.
    pub fn property(&self, name: &str) -> Option<MarkupAttributeValue> {
        self.properties.iter().find(|(n, _)| *n == name).map(|(_, value)| *value)
    }
}

/// The name and attributes of a parameter of a [`MarkupConstructor`]
/// (`[InheritDataTypeFrom(..)] FerroProperty property` of the managed
/// original).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkupParameter {
    /// The parameter name, when the declaration states it.
    pub name: Option<&'static str>,
    pub attributes: &'static [MarkupAttribute],
}

impl MarkupConstructor {
    /// The declared name of parameter `index`.
    pub fn parameter_name(&self, index: usize) -> Option<&'static str> {
        self.parameter_info.get(index).and_then(|parameter| parameter.name)
    }

    /// The declared attributes of parameter `index`.
    pub fn parameter_attributes(&self, index: usize) -> &'static [MarkupAttribute] {
        self.parameter_info.get(index).map_or(&[], |parameter| parameter.attributes)
    }
}

/// A constructor with arguments. (The parameterless constructor of a class
/// is its [`TypeInfo::default_constructor`]; other types list it here with
/// no parameters.)
#[derive(Clone, Copy)]
pub struct MarkupConstructor {
    pub parameters: &'static [TypeOf],
    /// What the declaration states about each parameter beyond its type
    /// (its name and attributes), by index. Empty when the constructor is
    /// declared in the positional form `(A, B) => callable`, otherwise as
    /// long as [`parameters`](Self::parameters).
    pub parameter_info: &'static [MarkupParameter],
    /// Creates the instance from the arguments.
    pub invoke: MarkupInvoke,
}

/// A property that is not a registered property: a plain property with a
/// getter and/or a setter. Collections are read-only properties whose type
/// has an `Add` method.
#[derive(Clone, Copy)]
pub struct MarkupProperty {
    pub name: &'static str,
    pub type_: TypeOf,
    /// Reads the property: `(instance) -> value`.
    pub get: Option<MarkupInvoke>,
    /// Writes the property: `(instance, value)`.
    pub set: Option<MarkupInvoke>,
    pub attributes: &'static [MarkupAttribute],
    /// Adds the TYPED element of this property to a compiled binding path
    /// built at run time (what `CompiledBindingPathBuilder::typed_property`
    /// adds in generated code), so that a binding whose path is this single
    /// property is a typed binding expression. Generated by the declaration
    /// forms from the accessors; the hook returns `None` (and the caller
    /// uses the untyped element) unless the instance type of the declaration
    /// is a shared reference type `Rc<S>` (notifying or not). `None` for a static
    /// property and for a property with a fallible setter. See
    /// [`typed_path`](super::typed_path).
    pub typed_path_element: Option<super::typed_path::TypedPathElement>,
}

/// An indexer (`this[..]` of the managed original): a property with index
/// parameters. Binding paths reach it with `Path[0]` / `Path[key]`.
#[derive(Clone, Copy)]
pub struct MarkupIndexer {
    /// The types of the index parameters, in order.
    pub parameters: &'static [TypeOf],
    /// The type of the values of the indexer.
    pub type_: TypeOf,
    /// Reads the value: `(instance, index..) -> value`.
    pub get: Option<MarkupInvoke>,
    /// Writes the value: `(instance, index.., value)`.
    pub set: Option<MarkupInvoke>,
    pub attributes: &'static [MarkupAttribute],
}

/// A method: an instance method (`Add`, a command method, an event handler)
/// or a static one (`Parse`, a factory method, an attached property
/// accessor).
#[derive(Clone, Copy)]
pub struct MarkupMethod {
    pub name: &'static str,
    pub is_static: bool,
    /// The parameter types, without the instance.
    pub parameters: &'static [TypeOf],
    /// `None` for a method that returns nothing.
    pub return_type: Option<TypeOf>,
    /// Calls the method: `(instance, arguments..)`, or `(arguments..)` for
    /// a static method.
    pub invoke: MarkupInvoke,
    pub attributes: &'static [MarkupAttribute],
}

/// A static field of the managed original: a `static readonly` field or a
/// constant (`Button.ClickEvent`, `ObjectConverters.IsNull`). A static
/// PROPERTY (`static T Name { get; }`: `Brushes.Red`) is declared in
/// [`MarkupType::static_properties`].
#[derive(Clone, Copy)]
pub struct MarkupField {
    pub name: &'static str,
    pub type_: TypeOf,
    pub get: fn() -> MarkupValue,
    pub attributes: &'static [MarkupAttribute],
}

/// An event that is not a routed event: handlers are attached through
/// `add`.
#[derive(Clone, Copy)]
pub struct MarkupEvent {
    pub name: &'static str,
    /// The types of the arguments the event passes to its handlers
    /// (sender first, where the event has one).
    pub arguments: &'static [TypeOf],
    /// Subscribes a handler: `(instance, handler)`, where the handler is a
    /// [`MarkupDelegate`] called with the event arguments in untyped form.
    pub add: MarkupInvoke,
}

/// A member of an enumeration.
#[derive(Clone, Copy)]
pub struct MarkupEnumMember {
    pub name: &'static str,
    /// The numeric value (the bits, for a flags enumeration).
    pub value: i64,
    /// The member as a value of the enumeration type.
    pub get: fn() -> BoxedValue,
}

/// States that a type is an instantiation of a generic type
/// (`FerroList<T>`): the name of the definition with its arity, as the
/// managed original spells it (``FerroList`1``), and the type arguments.
#[derive(Clone, Copy)]
pub struct MarkupGeneric {
    pub definition: &'static str,
    pub arguments: &'static [TypeOf],
}

/// The markup metadata of a type. Built by the declaration macros as
/// constant data; see the [module documentation](self).
#[derive(Clone, Copy)]
pub struct MarkupType {
    /// The type name, without namespace, as markup spells it.
    pub name: &'static str,
    /// The dotted namespace, when it is not the namespace registered for
    /// [`module_path`](Self::module_path) (types that mirror runtime library
    /// types state `"System"` here).
    pub explicit_namespace: Option<&'static str>,
    /// The Rust module that declares the type (`module_path!()`).
    pub module_path: &'static str,
    pub kind: MarkupTypeKind,
    /// The Rust types values of this type are held in, the untyped
    /// (canonical) form first: `[Ref<T>, Option<Ref<T>>]` for a class,
    /// `[T, Rc<T>, Option<Rc<T>>]` for a reference type, `[T]` for a
    /// value type (its nullable form `Option<T>` is a type of its own, as
    /// in the managed original), `[Rc<dyn I>, Option<Rc<dyn I>>]` for a
    /// contract. Empty for static types.
    pub handles: &'static [TypeOf],
    /// The nullable form of a value type or enumeration (`Option<T>`), which
    /// is a type of its own (the `Nullable<T>` of the managed original).
    /// `None` for every other kind of type.
    pub nullable: Option<TypeOf>,
    /// The class of the object model this metadata belongs to, if any.
    pub type_info: Option<fn() -> &'static TypeInfo>,
    /// The base type (by one of its handles). Classes of the object model
    /// leave this empty: their base is [`TypeInfo::base_type`].
    pub base: Option<TypeOf>,
    /// The contracts the type implements (by their handles), in addition
    /// to the ones of its base type.
    pub interfaces: &'static [TypeOf],
    pub generic: Option<MarkupGeneric>,
    /// The name of the content property (the `[Content]` attribute of the
    /// managed original). Inherited by deriving types.
    pub content_property: Option<&'static str>,
    /// Converts text to a value of the type: `(text) -> value`. The
    /// `Parse(string)` method of the managed original.
    pub parse: Option<MarkupInvoke>,
    pub constructors: &'static [MarkupConstructor],
    pub properties: &'static [MarkupProperty],
    /// The indexers the type declares (not the ones of its base types), in
    /// declaration order.
    pub indexers: &'static [MarkupIndexer],
    /// Attributes of REGISTERED properties (styled, direct, attached) the
    /// type declares or owns, by property name. Registered properties need
    /// no declaration; this only carries what the property system does not
    /// know about them (`[DependsOn]`, `[ResolveByName]`,
    /// `[InheritDataTypeFromItems]`, `[TemplateContent]`, ...).
    pub property_attributes: &'static [(&'static str, &'static [MarkupAttribute])],
    pub methods: &'static [MarkupMethod],
    /// The static fields and constants (`static readonly` / `const` of the
    /// managed original).
    pub fields: &'static [MarkupField],
    /// The static properties (`static T Name { get; set; }` of the managed
    /// original): the accessors take no instance, `get` is `() -> value`
    /// and `set` is `(value)`.
    pub static_properties: &'static [MarkupProperty],
    pub events: &'static [MarkupEvent],
    pub enum_members: &'static [MarkupEnumMember],
    /// Whether an enumeration is a set of flags.
    pub is_flags: bool,
    /// Converts a numeric value to a value of the enumeration: the cast
    /// `(TEnum)value` of the managed original. For a plain enumeration the
    /// value must be the value of a member; for a set of flags any
    /// combination of the bits of its members.
    pub enum_from_value: Option<fn(i64) -> Option<BoxedValue>>,
    pub attributes: &'static [MarkupAttribute],
    /// Views an untyped value of this type as a source of property change
    /// notifications (the `INotifyPropertyChanged` of the managed original),
    /// for types that are not classes of the object model: view models.
    /// Bindings resolved through metadata (reflection bindings, compiled
    /// bindings built at run time) subscribe through it; `None` means the
    /// properties of the type are read once.
    pub notify_property_changed:
        Option<fn(&dyn crate::AnyValue) -> Option<&dyn crate::data::model::INotifyPropertyChanged>>,
}

impl MarkupType {
    /// Metadata that states nothing but the name and kind of the type.
    pub const fn new(name: &'static str, kind: MarkupTypeKind, module_path: &'static str) -> Self {
        Self {
            name,
            explicit_namespace: None,
            module_path,
            kind,
            handles: &[],
            nullable: None,
            type_info: None,
            base: None,
            interfaces: &[],
            generic: None,
            content_property: None,
            parse: None,
            constructors: &[],
            properties: &[],
            indexers: &[],
            property_attributes: &[],
            methods: &[],
            fields: &[],
            static_properties: &[],
            events: &[],
            enum_members: &[],
            is_flags: false,
            enum_from_value: None,
            attributes: &[],
            notify_property_changed: None,
        }
    }

    /// The dotted, markup-facing namespace of the type.
    pub fn namespace(&self) -> &'static str {
        match self.explicit_namespace {
            Some(namespace) => namespace,
            None => TypeInfo::namespace_of_module(self.module_path),
        }
    }

    /// The namespace-qualified name of the type.
    pub fn full_name(&self) -> String {
        match self.namespace() {
            "" => self.name.to_string(),
            namespace => format!("{namespace}.{}", self.name),
        }
    }

    /// The untyped (canonical) handle type of the type, if it has values.
    pub fn handle(&self) -> Option<ValueType> {
        self.handles.first().map(|handle| handle())
    }

    /// The plain property `name` declared by this type (not by its base
    /// types).
    pub fn find_property(&self, name: &str) -> Option<&'static MarkupProperty> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// The indexer this type declares (not its base types) that takes
    /// `parameter_count` index arguments.
    pub fn find_indexer(&self, parameter_count: usize) -> Option<&'static MarkupIndexer> {
        self.indexers.iter().find(|indexer| indexer.parameters.len() == parameter_count)
    }

    /// The registered metadata of the base type ([`base`](Self::base)): for
    /// a class of the object model the metadata of the nearest base class
    /// that has any, otherwise the registered type that the base handle
    /// belongs to. This is how untyped code walks the hierarchy of a type,
    /// as reflection walks base classes.
    pub fn base_type(&self) -> Option<&'static MarkupType> {
        if let Some(type_info) = self.type_info {
            let mut current = type_info().base_type();
            while let Some(class) = current {
                if let Some(markup) = class.markup() {
                    return Some(markup);
                }
                current = class.base_type();
            }
        }
        Self::find_by_handle(self.base?().id())
    }

    /// The attributes this type declares for its registered property `name`.
    pub fn find_property_attributes(&self, name: &str) -> &'static [MarkupAttribute] {
        self.property_attributes.iter().find(|(n, _)| *n == name).map_or(&[], |(_, attributes)| *attributes)
    }

    /// The methods named `name` declared by this type.
    pub fn find_methods<'a>(&self, name: &'a str) -> impl Iterator<Item = &'static MarkupMethod> + 'a {
        self.methods.iter().filter(move |m| m.name == name)
    }

    /// The static field `name` declared by this type.
    pub fn find_field(&self, name: &str) -> Option<&'static MarkupField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The static property `name` declared by this type.
    pub fn find_static_property(&self, name: &str) -> Option<&'static MarkupProperty> {
        self.static_properties.iter().find(|p| p.name == name)
    }

    /// The event `name` declared by this type.
    pub fn find_event(&self, name: &str) -> Option<&'static MarkupEvent> {
        self.events.iter().find(|e| e.name == name)
    }

    /// The attribute `name` of the type.
    pub fn find_attribute(&self, name: &str) -> Option<&'static MarkupAttribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    /// The enumeration member `name`; `ignore_case` as the managed
    /// `Enum.Parse(.., ignoreCase)`.
    pub fn find_enum_member(&self, name: &str, ignore_case: bool) -> Option<&'static MarkupEnumMember> {
        self.enum_members
            .iter()
            .find(|m| if ignore_case { m.name.eq_ignore_ascii_case(name) } else { m.name == name })
    }

    /// Adds a type to the process-wide table of types with markup metadata,
    /// making it discoverable with [`find`](Self::find) and
    /// [`find_by_handle`](Self::find_by_handle). Registering a type again
    /// does nothing.
    ///
    /// A crate registers its types from its `register_types()` function.
    /// The metadata of a class of the object model needs no registration to
    /// be reachable from its runtime type ([`TypeInfo::markup`]).
    pub fn register(type_: &'static MarkupType) {
        Self::register_all(&[type_]);
    }

    /// Registers several types; see [`register`](Self::register).
    pub fn register_all(types: &[&'static MarkupType]) {
        let mut registry = registry().write().unwrap_or_else(|e| e.into_inner());
        for type_ in types {
            registry.insert(type_);
        }
    }

    /// Finds a registered type by namespace and name.
    pub fn find(namespace: &str, name: &str) -> Option<&'static MarkupType> {
        let candidates: Vec<&'static MarkupType> = {
            let registry = registry().read().unwrap_or_else(|e| e.into_inner());
            registry.by_name.get(name)?.clone()
        };
        candidates.into_iter().find(|t| t.namespace() == namespace)
    }

    /// Finds the registered type that values of the Rust type `handle` are
    /// values of (any of its [`handles`](Self::handles)).
    pub fn find_by_handle(handle: TypeId) -> Option<&'static MarkupType> {
        let registry = registry().read().unwrap_or_else(|e| e.into_inner());
        registry.by_handle.get(&handle).copied()
    }

    /// Finds the registered value type or enumeration whose nullable form
    /// ([`nullable`](Self::nullable)) is the Rust type `handle`.
    pub fn find_by_nullable_handle(handle: TypeId) -> Option<&'static MarkupType> {
        let registry = registry().read().unwrap_or_else(|e| e.into_inner());
        registry.by_nullable_handle.get(&handle).copied()
    }

    /// Finds the registered metadata that belongs to the runtime type
    /// `type_` ([`type_info`](Self::type_info)): the metadata of a static
    /// type declared with `ferro_markup_type!(static X { type_info: X, .. })`.
    /// (The metadata of a class is its [`TypeInfo::markup`].)
    pub fn find_by_type_info(type_: &'static TypeInfo) -> Option<&'static MarkupType> {
        if let Some(markup) = type_.markup() {
            return Some(markup);
        }
        let registry = registry().read().unwrap_or_else(|e| e.into_inner());
        registry.by_type_info.get(&(type_ as *const TypeInfo as usize)).copied()
    }

    /// Every registered type, in registration order.
    pub fn registered_types() -> Vec<&'static MarkupType> {
        registry().read().unwrap_or_else(|e| e.into_inner()).types.clone()
    }
}

impl fmt::Debug for MarkupType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// A Rust type that declares markup metadata with
/// [`ferro_markup_type!`](crate::ferro_markup_type) or
/// [`ferro_markup_enum!`](crate::ferro_markup_enum).
pub trait MarkupTyped {
    /// The markup metadata of the type.
    const MARKUP: &'static MarkupType;
}

#[derive(Default)]
struct MarkupRegistry {
    types: Vec<&'static MarkupType>,
    by_name: HashMap<&'static str, Vec<&'static MarkupType>>,
    by_handle: HashMap<TypeId, &'static MarkupType>,
    by_nullable_handle: HashMap<TypeId, &'static MarkupType>,
    by_type_info: HashMap<usize, &'static MarkupType>,
}

impl MarkupRegistry {
    fn insert(&mut self, type_: &'static MarkupType) {
        let known = self.by_name.get(type_.name).is_some_and(|types| types.iter().any(|t| std::ptr::eq(*t, type_)));
        if known {
            return;
        }
        self.types.push(type_);
        self.by_name.entry(type_.name).or_default().push(type_);
        for handle in type_.handles {
            self.by_handle.entry(handle().id()).or_insert(type_);
        }
        if let Some(type_info) = type_.type_info {
            self.by_type_info.entry(type_info() as *const TypeInfo as usize).or_insert(type_);
        }
        if let Some(nullable) = type_.nullable {
            self.by_nullable_handle.entry(nullable().id()).or_insert(type_);
        }
    }
}

fn registry() -> &'static RwLock<MarkupRegistry> {
    static REGISTRY: OnceLock<RwLock<MarkupRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

/// The names of the attributes the framework understands, for
/// [`MarkupAttribute::name`]. Each is the name of the attribute class of the
/// managed original without its `Attribute` suffix; arguments are the ones
/// of that class.
pub mod attributes {
    /// On a property: its value is content built on demand
    /// (`TemplateResultType` names the type of the built object).
    pub const TEMPLATE_CONTENT: &str = "TemplateContent";
    /// On a property: must be assigned after the named property.
    pub const DEPENDS_ON: &str = "DependsOn";
    /// On a property: a binding assigned in markup is stored as the value
    /// instead of being applied.
    pub const ASSIGN_BINDING: &str = "AssignBinding";
    /// On a type: instances are attached to their parent before their own
    /// properties are set.
    pub const USABLE_DURING_INITIALIZATION: &str = "UsableDuringInitialization";
    /// On a collection type: whitespace between its items is significant.
    pub const WHITESPACE_SIGNIFICANT_COLLECTION: &str = "WhitespaceSignificantCollection";
    /// On a type: whitespace around an element of the type is trimmed.
    pub const TRIM_SURROUNDING_WHITESPACE: &str = "TrimSurroundingWhitespace";
    /// On a type or property: names the converter type used to convert
    /// text.
    pub const TYPE_CONVERTER: &str = "TypeConverter";
    /// On a type: it is the scope of a control template.
    pub const CONTROL_TEMPLATE_SCOPE: &str = "ControlTemplateScope";
    /// On a property: it states the data type of the bindings in scope.
    pub const DATA_TYPE: &str = "DataType";
    /// On a property: the data type of its bindings is inherited from
    /// another property.
    pub const INHERIT_DATA_TYPE_FROM: &str = "InheritDataTypeFrom";
    /// On a property: the data type of its bindings is the item type of a
    /// collection property.
    pub const INHERIT_DATA_TYPE_FROM_ITEMS: &str = "InheritDataTypeFromItems";
    /// On a property: text assigned in markup is a name to resolve in the
    /// name scope.
    pub const RESOLVE_BY_NAME: &str = "ResolveByName";
    /// On a property of an option markup extension: the option it holds.
    pub const MARKUP_EXTENSION_OPTION: &str = "MarkupExtensionOption";
    /// On a property of an option markup extension: the default option.
    pub const MARKUP_EXTENSION_DEFAULT_OPTION: &str = "MarkupExtensionDefaultOption";
    /// On a type or property: how a list is parsed from text (separators,
    /// split options).
    pub const FERRO_LIST: &str = "FerroList";
    /// On a control type: a named part its template must contain.
    pub const TEMPLATE_PART: &str = "TemplatePart";
    /// On a control type: the pseudo-classes it sets.
    pub const PSEUDO_CLASSES: &str = "PseudoClasses";
    /// On any member: it is obsolete (the argument is the message).
    pub const OBSOLETE: &str = "Obsolete";
    /// On any member: it is not stable API.
    pub const UNSTABLE: &str = "Unstable";
    /// On a type: it is private API.
    pub const PRIVATE_API: &str = "PrivateApi";
    /// On a type: implemented only by the framework.
    pub const NOT_CLIENT_IMPLEMENTABLE: &str = "NotClientImplementable";
    /// On a property: the content property (stated with `content:` in the
    /// declaration forms; listed for completeness).
    pub const CONTENT: &str = "Content";
    /// On a constructor parameter or property of a markup extension.
    pub const CONSTRUCTOR_ARGUMENT: &str = "ConstructorArgument";
}
