//! What the emitter of Rust source asks of the type system it emits against
//! (docs/porting/xaml.md, 9.5.5: the part of a type system that is "Rust
//! spelling of types and call forms").
//!
//! The transformed tree names types and members through the contracts of the
//! compiler (`IXamlType`, `IXamlMethod`, ..). To write a call the emitter
//! needs more than they state: the Rust type that holds a value
//! ([`TypeKey`]), the public Rust path of a type, the typed function of a
//! declared member, the accessor of a registered property, and which Rust
//! types convert to which. [`EmitTypes`] is that part, asked of whichever
//! type system the documents were transformed with:
//!
//! - the run-time type system, where a Rust type is its `TypeId` and the
//!   answers come from the registries of the process
//!   ([`RuntimeEmitTypes`](super::runtime_types::RuntimeEmitTypes));
//! - the build-time type system over the models of the crates (`ferroui-build`,
//!   `ModelTypeSystem`), where a Rust type is its normalised text and the
//!   answers come from the models.
//!
//! The emitter never compares the two kinds of key with each other: one run
//! emits against one type system.

use std::any::{Any, TypeId};
use std::rc::Rc;

use xamlx::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlType};

/// A Rust type, as the emitter compares the type of a value with the type a
/// member declares: the `TypeId` of the type (the run-time type system), or
/// its normalised text with every alias expanded (the build-time type system).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeKey<'a> {
    Id(TypeId),
    Text(&'a str),
}

/// The Rust types the emitter itself names: the types of the values it writes
/// without a member of the type system stating them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Known {
    /// `String`.
    String,
    /// `Option<String>`.
    OptionString,
    Bool,
    Char,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    /// `Option<BoxedValue>`: a value of type `object`.
    Object,
    /// `Rc<dyn IServiceProvider>`.
    ServiceProvider,
    /// `Option<Rc<dyn IServiceProvider>>`.
    OptionServiceProvider,
    /// `Selector`.
    Selector,
    /// `Option<Selector>`.
    OptionSelector,
    /// `Option<Uri>`.
    OptionUri,
    /// `&'static TypeInfo`.
    Class,
    /// `Option<&'static TypeInfo>`.
    OptionClass,
    /// `ValueType`.
    ValueType,
    /// `Option<ValueType>`.
    OptionValueType,
    /// `TypeId`.
    TypeId,
    /// `Option<TypeId>`.
    OptionTypeId,
    /// `BindingPriority`.
    BindingPriority,
    /// `Rc<DeferredContent>`.
    DeferredContent,
    /// `CompiledBindingPath`.
    CompiledBindingPath,
    /// `&'static FerroProperty`.
    Property,
    /// `Option<&'static FerroProperty>`.
    OptionProperty,
    /// `MarkupDelegate`: the delegate of a method named in markup.
    Delegate,
    /// `Rc<dyn ITypeDescriptorContext>`: the context a type converter converts in.
    TypeDescriptorContext,
    /// `Option<Rc<dyn ITypeDescriptorContext>>`.
    OptionTypeDescriptorContext,
}

/// The Rust type that holds a value of a type of the type system (its handle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle<'a> {
    pub key: TypeKey<'a>,
    /// The handle is the untyped value (`object`).
    pub object: bool,
    /// The Rust type as text, for a diagnostic.
    pub name: &'a str,
}

impl<'a> Handle<'a> {
    pub fn id(&self) -> TypeKey<'a> {
        self.key
    }

    pub fn is_object(&self) -> bool {
        self.object
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}

/// A class of the object model (`ferro_class!`).
pub trait EmitClass {
    /// The markup name (`Border`).
    fn name(&self) -> &str;
    /// The namespace-qualified name.
    fn full_name(&self) -> String;
    /// The public Rust path generated code names the class by, when there is one.
    fn rust_path(&self) -> Option<&str>;
    /// `Ref<Class>`.
    fn handle(&self) -> Option<TypeKey<'_>>;
    /// The class has a constructor without parameters (`new:`).
    fn has_default_constructor(&self) -> bool;
    /// `other` is this class or derives from it.
    fn is_assignable_from(&self, other: &dyn EmitClass) -> bool;
    /// `other` is this class.
    fn same(&self, other: &dyn EmitClass) -> bool;
    fn as_any(&self) -> &dyn Any;
}

/// A member of an enumeration or of a set of flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumMember<'a> {
    pub name: &'a str,
    pub value: i64,
    /// The Rust variant of a member of a plain enumeration.
    pub rust_variant: Option<&'a str>,
}

/// The markup metadata of a type (`ferro_markup_type!`, `ferro_markup_enum!`, the
/// metadata of a class).
pub trait EmitMarkup {
    fn full_name(&self) -> String;
    /// The public Rust path of the type (of the trait, for a contract).
    fn rust_path(&self) -> Option<&str>;
    /// The path names a trait: the type is `dyn Path`.
    fn rust_path_is_trait(&self) -> bool;
    /// The Rust types that hold a value of the type, the first one first.
    fn handles(&self) -> Vec<TypeKey<'_>>;
    /// The first handle is a shared reference (`Rc<T>`).
    fn handle_is_shared(&self) -> bool;
    /// The nullable form of the handle of a value type (`Option<T>`).
    fn nullable(&self) -> Option<TypeKey<'_>>;
    /// The handles of the contracts the declaration lists.
    fn interfaces(&self) -> Vec<TypeKey<'_>>;
    /// The metadata of the base type the declaration names.
    fn base_type(&self) -> Option<&dyn EmitMarkup>;
    /// The type instance members receive (`this:`).
    fn this(&self) -> Option<TypeKey<'_>>;
    /// The type of a value of the type: what constructors and `Parse` return.
    fn value(&self) -> Option<TypeKey<'_>>;
    fn is_flags(&self) -> bool;
    fn enum_members(&self) -> Vec<EnumMember<'_>>;
    /// The members of the set of flags make up `value`.
    fn flags_compose(&self, value: i64) -> bool;
    /// `other` is this metadata.
    fn same(&self, other: &dyn EmitMarkup) -> bool;
    fn as_any(&self) -> &dyn Any;

    /// The first handle.
    fn handle(&self) -> Option<TypeKey<'_>> {
        self.handles().into_iter().next()
    }
}

/// A registered property.
pub trait EmitProperty {
    fn name(&self) -> &str;
    /// The name of the class that registered the property.
    fn owner_name(&self) -> String;
    fn is_direct(&self) -> bool;
    fn is_read_only(&self) -> bool;
    /// The value type.
    fn property_type(&self) -> TypeKey<'_>;
    /// The value type as text, for a diagnostic.
    fn property_type_name(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}

/// The typed function a declaration macro writes for a declared member
/// (`__markup_get_Child`), an associated function of the declared type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmitFunction {
    pub function: String,
    /// The function returns `Result<_, MarkupInvokeError>`.
    pub fallible: bool,
}

/// What a declared method is in its declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclaredKind {
    Getter,
    Setter,
    StaticGetter,
    StaticSetter,
    Method,
    Parse,
}

impl DeclaredKind {
    /// An accessor of a plain property.
    pub fn is_accessor(self) -> bool {
        !matches!(self, DeclaredKind::Method | DeclaredKind::Parse)
    }
}

/// The declaration a method is the projection of.
#[derive(Clone, Debug)]
pub struct DeclaredMethod<'a> {
    pub kind: DeclaredKind,
    pub emit: Option<EmitFunction>,
    /// The Rust type the member returns: the type of the property of a getter, the
    /// return type of a method, the value type of the type for `Parse`.
    pub returns: Option<TypeKey<'a>>,
}

impl DeclaredMethod<'_> {
    pub fn emit(&self) -> Option<&EmitFunction> {
        self.emit.as_ref()
    }
}

/// A method of the type system, as the emitter reads it.
#[derive(Clone)]
pub struct MethodInfo<'a> {
    pub name: String,
    pub is_static: bool,
    pub declaring_type: Option<Rc<dyn IXamlType>>,
    /// The parameters, without the instance.
    pub parameters: Vec<Rc<dyn IXamlType>>,
    /// The declared Rust types of the parameters.
    pub parameter_handles: Vec<Option<Handle<'a>>>,
    declared: Option<DeclaredMethod<'a>>,
    /// The method is one the type system builds (the accessor of a registered property,
    /// a member of a runtime library type), not one a declaration states a callable for.
    pub built: bool,
}

impl<'a> MethodInfo<'a> {
    pub fn new(
        name: String,
        is_static: bool,
        declaring_type: Option<Rc<dyn IXamlType>>,
        parameters: Vec<Rc<dyn IXamlType>>,
        parameter_handles: Vec<Option<Handle<'a>>>,
        declared: Option<DeclaredMethod<'a>>,
        built: bool,
    ) -> Self {
        Self { name, is_static, declaring_type, parameters, parameter_handles, declared, built }
    }

    /// The member of markup metadata the method is the projection of.
    pub fn declared(&self) -> Option<&DeclaredMethod<'a>> {
        self.declared.as_ref()
    }
}

/// A constructor of the type system, as the emitter reads it.
#[derive(Clone)]
pub struct ConstructorInfo<'a> {
    pub declaring_type: Option<Rc<dyn IXamlType>>,
    pub parameters: Vec<Rc<dyn IXamlType>>,
    pub parameter_handles: Vec<Option<Handle<'a>>>,
    /// The constructor is one markup metadata declares; its typed function, when the
    /// declaration generated one.
    pub declared: Option<Option<EmitFunction>>,
    /// The constructor is one the type system builds: the default constructor of a class.
    pub built: bool,
}

/// What a static field holds.
#[derive(Clone)]
pub enum FieldValue<'a> {
    /// A field of markup metadata: its name, its typed function and its type.
    Declared { name: String, emit: Option<EmitFunction>, type_: TypeKey<'a> },
    /// A member of an enumeration.
    EnumMember,
    /// Anything else: the definition of a registered property, or nothing.
    Other,
}

/// A static field of the type system, as the emitter reads it.
#[derive(Clone)]
pub struct FieldInfo<'a> {
    pub declaring_type: Option<Rc<dyn IXamlType>>,
    /// The registered property the field holds the definition of.
    pub property: Option<&'a dyn EmitProperty>,
    pub value: FieldValue<'a>,
}

impl<'a> FieldInfo<'a> {
    pub fn ferro_property(&self) -> Option<&'a dyn EmitProperty> {
        self.property
    }
}

/// The types of the framework whose Rust paths the emitter writes by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameworkType {
    /// `FerroUI.Styling.Setter`.
    Setter,
    /// `FerroUI.Styling.SetterBase`.
    SetterBase,
    /// `FerroUI.Styling.StyleBase`.
    StyleBase,
}

/// The part of a type system the emitter of Rust source reads; see the module.
pub trait EmitTypes {
    /// The key of a Rust type the emitter names.
    fn known(&self, known: Known) -> TypeKey<'_>;
    /// The key of `Option<T>` of a Rust type the emitter names: the nullable form of a
    /// value the emitter writes itself (text, a number, the definition of a property).
    fn option_of_known(&self, known: Known) -> TypeKey<'_>;
    /// Whether `type_` is a type of this type system.
    fn is_own(&self, type_: &dyn IXamlType) -> bool;
    /// The class of the object model `type_` is.
    fn class_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitClass>;
    /// The markup metadata `type_` was projected from.
    fn markup_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup>;
    /// The markup metadata of `type_`: the one it was projected from, or, for a type of
    /// the runtime library that a declaration is registered for, that declaration.
    fn metadata_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup>;
    /// The handle of `type_`.
    fn handle_of(&self, type_: &dyn IXamlType) -> Option<Handle<'_>>;

    /// The class whose handle (`Ref<Class>`, or with `true` `Option<Ref<Class>>`) `key` is.
    fn class_by_handle(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)>;
    /// The markup metadata that declares `key` among its handles.
    fn markup_by_handle(&self, key: TypeKey<'_>) -> Option<&dyn EmitMarkup>;
    /// The class an element reference names (`ElementRef<Class>`, or with `true` its
    /// nullable form).
    fn element_ref_class(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)>;
    /// The Rust name of a primitive type the type system defines itself (`i32`,
    /// `::std::string::String`).
    fn primitive_type_name(&self, key: TypeKey<'_>) -> Option<&'static str>;
    /// A value of the Rust type `from` is a value of `to` through the casts the crates
    /// register (a contract a class implements, a registered cast).
    fn is_assignable(&self, from: TypeKey<'_>, to: TypeKey<'_>) -> bool;
    /// The null of the untyped value conversions is a value of exactly `target` (`None` of
    /// a nullable form).
    fn null_converts_to(&self, target: TypeKey<'_>) -> bool;
    /// The type `key` is the nullable form of.
    fn nullable_inner(&self, key: TypeKey<'_>) -> Option<TypeKey<'_>>;

    /// `class` is a styled element.
    fn is_styled_element(&self, class: &dyn EmitClass) -> bool;
    /// `class` has markup of its own (a document with `x:Class`).
    fn has_class_document(&self, class: &dyn EmitClass) -> bool;
    /// The public Rust path of a type of the framework.
    fn framework_path(&self, type_: FrameworkType) -> Option<String>;

    fn method(&self, method: &dyn IXamlMethod) -> Option<MethodInfo<'_>>;
    fn constructor(&self, constructor: &dyn IXamlConstructor) -> Option<ConstructorInfo<'_>>;
    fn field(&self, field: &dyn IXamlField) -> Option<FieldInfo<'_>>;

    /// The Rust expression that yields the definition of `property`: a call of a public
    /// accessor of it, by the public path of the type whose function the accessor is; of
    /// several, the first in a fixed order (the accessors of `preferred`, the type the
    /// property was resolved on, and of its base types, then the ones of the type that
    /// registered the property and of its base types). `Err` says why there is none.
    fn property_definition(&self, property: &dyn EmitProperty, preferred: Option<&dyn EmitClass>) -> Result<String, String>;

    /// The questions asked since the last call that this type system could not answer,
    /// each as text that names the type or the member. A type system that reads its
    /// answers from declarations cannot answer what only a process decides; it answers
    /// such a question with *no* and records it here, and the host refuses the document
    /// the question was asked for, so that nothing is emitted from a guess. The run-time
    /// type system answers everything.
    fn take_unanswered(&self) -> Vec<String> {
        Vec::new()
    }
}
