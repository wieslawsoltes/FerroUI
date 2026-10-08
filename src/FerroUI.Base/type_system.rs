//! Runtime type information and the class model of the object hierarchy.
//!
//! The framework is a deep single-inheritance class hierarchy with virtual
//! members. It is modelled as follows:
//!
//! * A class is a `#[repr(C)]` struct whose first field, `base`, is its base
//!   class. It derefs to the base, so inherited members are reached through
//!   auto-deref with static dispatch and no runtime cost.
//! * Objects live on the heap behind [`Ref<T>`], a reference-counted handle
//!   that can be upcast to any base class for free and downcast with a runtime
//!   check, like an object reference in a managed language.
//! * Virtual members are declared once with [`ferro_class!`]. For each class
//!   that introduces virtuals this generates a `<Class>Impl` trait whose
//!   associated functions are the overridable members. A subclass overrides a
//!   member by implementing the function; members it does not implement fall
//!   through to the nearest base class that does, and an override can call
//!   its base implementation with `parent_<member>`.
//! * Dispatch goes through one per-class table of function pointers. An
//!   object holds a single pointer to the table of its most-derived class.
//!
//! All `unsafe` needed for this lives in this module and in the macros it
//! exports, and rests on two invariants that the macros establish and check
//! at compile time: a class struct starts with its base class at offset zero,
//! and a class's virtual table starts with its base class's virtual table.

use crate::metadata::{MarkupType, TypeOf};
use crate::{FerroObject, FerroProperty};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::{OnceLock, RwLock};

/// Describes a class (or a non-instantiable type that owns attached
/// properties): its name, where it is declared, its base class, how to
/// initialise it and how to create an instance of it.
///
/// A `TypeInfo` is plain constant data, built by [`ferro_class!`] and
/// [`ferro_static_type!`].
pub struct TypeInfo {
    name: &'static str,
    base: Option<&'static TypeInfo>,
    module_path: &'static str,
    class_init: Option<fn()>,
    constructor: Option<fn() -> Ref<FerroObject>>,
    handles: Option<fn() -> [TypeId; 2]>,
    interfaces: &'static [TypeOf],
    markup: Option<&'static MarkupType>,
}

impl TypeInfo {
    pub const fn new(name: &'static str, base: Option<&'static TypeInfo>) -> Self {
        Self {
            name,
            base,
            module_path: "",
            class_init: None,
            constructor: None,
            handles: None,
            interfaces: &[],
            markup: None,
        }
    }

    /// States the Rust module that declares the type (`module_path!()` at
    /// the declaration). The markup-facing [`namespace`](Self::namespace) is
    /// derived from it.
    pub const fn with_module_path(mut self, module_path: &'static str) -> Self {
        self.module_path = module_path;
        self
    }

    /// Attaches the static initialisation of the type: a function that,
    /// once per thread and after doing the same for the base type, registers
    /// what the type declares (handle casts, property definitions) and runs
    /// its static constructor.
    ///
    /// It is what lets code that only has the runtime type (a lookup of a
    /// property by owner type and name) initialise a type that has no
    /// instance yet.
    pub const fn with_class_init(mut self, class_init: fn()) -> Self {
        self.class_init = Some(class_init);
        self
    }

    /// Attaches the default constructor of the class: the function that
    /// creates an instance as its parameterless `new()` does.
    pub const fn with_constructor(mut self, constructor: Option<fn() -> Ref<FerroObject>>) -> Self {
        self.constructor = constructor;
        self
    }

    /// States the Rust types of the handle of the class and of its nullable
    /// form (`Ref<T>`, `Option<Ref<T>>`), which makes the class findable by
    /// the type of a value ([`find_by_handle`](Self::find_by_handle)).
    pub const fn with_handles(mut self, handles: fn() -> [TypeId; 2]) -> Self {
        self.handles = Some(handles);
        self
    }

    /// States the interface handles the class declares
    /// ([`ferro_class_info!`], `interfaces`).
    pub const fn with_interfaces(mut self, interfaces: &'static [TypeOf]) -> Self {
        self.interfaces = interfaces;
        self
    }

    /// Attaches the markup metadata of the class ([`ferro_class_info!`],
    /// `markup`).
    pub const fn with_markup(mut self, markup: Option<&'static MarkupType>) -> Self {
        self.markup = markup;
        self
    }

    /// The class name, without namespace.
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The direct base class, if any.
    #[inline]
    pub fn base_type(&self) -> Option<&'static TypeInfo> {
        self.base
    }

    /// The Rust module that declares the type, as `module_path!()` spells
    /// it (`ferroui_base::media::solid_color_brush`). Empty for a type whose
    /// runtime type was written by hand.
    #[inline]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    /// The dotted, markup-facing namespace of the type (`FerroUI.Media`).
    ///
    /// Namespaces are declared once per module, not per class: a crate maps
    /// its module paths to namespaces with [`TypeInfo::register_namespaces`]
    /// (from its `register_types()` function), and the namespace of a type
    /// is the one registered for the longest prefix of its
    /// [`module_path`](Self::module_path). Empty when no prefix is
    /// registered.
    pub fn namespace(&self) -> &'static str {
        let registry = read_registry();
        registry.namespace_of(self.module_path)
    }

    /// The dotted namespace registered for a Rust module path; see
    /// [`namespace`](Self::namespace).
    pub fn namespace_of_module(module_path: &str) -> &'static str {
        read_registry().namespace_of(module_path)
    }

    /// The namespace-qualified name of the type (`FerroUI.Media.Brush`).
    pub fn full_name(&self) -> String {
        match self.namespace() {
            "" => self.name.to_string(),
            namespace => format!("{namespace}.{}", self.name),
        }
    }

    /// Whether the type has a static initialisation of its own
    /// ([`with_class_init`](Self::with_class_init)).
    #[cfg(feature = "compiler-metadata")]
    pub(crate) fn has_class_init(&self) -> bool {
        self.class_init.is_some()
    }

    /// Ensures the static initialisation of the type and of its base types
    /// has run on the current thread: handle casts and property definitions
    /// are registered (properties in declaration order) and the static
    /// constructors have run, base types first. Does nothing after the first
    /// call on a thread.
    ///
    /// Called when the first instance of a class is created, when a property
    /// accessor of the type is first called and by the property registry when
    /// it is asked about the type; call it directly where the static
    /// constructor of a type must have run before none of these happened.
    #[inline]
    pub fn ensure_class_init(&self) {
        let mut current = Some(self);
        while let Some(type_) = current {
            if let Some(class_init) = type_.class_init {
                // An initialiser initialises its base types itself.
                class_init();
                return;
            }
            current = type_.base;
        }
    }

    /// The default constructor of the class, if it declares one (see
    /// [`ferro_class_info!`]): creates an instance as the parameterless
    /// `new()` of the class does.
    #[inline]
    pub fn default_constructor(&self) -> Option<fn() -> Ref<FerroObject>> {
        self.constructor
    }

    /// Creates an instance with the default constructor of the class.
    /// `None` if the class has none (abstract classes, classes that need
    /// constructor arguments, non-instantiable types).
    pub fn create_instance(&self) -> Option<Ref<FerroObject>> {
        self.constructor.map(|constructor| constructor())
    }

    /// The Rust type of the handle of the class (`Ref<T>`). `None` for types
    /// that are not classes.
    pub fn handle(&self) -> Option<TypeId> {
        self.handles.map(|handles| handles()[0])
    }

    /// The Rust type of the nullable handle of the class (`Option<Ref<T>>`).
    pub fn nullable_handle(&self) -> Option<TypeId> {
        self.handles.map(|handles| handles()[1])
    }

    /// The interface handles the class itself declares with
    /// [`ferro_class_info!`] (not the ones of its base classes).
    #[inline]
    pub fn interfaces(&self) -> &'static [TypeOf] {
        self.interfaces
    }

    /// The markup metadata the class itself declares with
    /// [`ferro_class_info!`]: its content property, plain properties,
    /// methods, events and attributes. `None` for a class that declares
    /// none; the metadata of its base classes still applies to it.
    #[inline]
    pub fn markup(&self) -> Option<&'static MarkupType> {
        self.markup
    }

    /// The name of the content property of the class: the one it declares,
    /// or else the one of the nearest base class that declares one.
    pub fn content_property(&self) -> Option<&'static str> {
        let mut current = Some(self);
        while let Some(type_) = current {
            if let Some(content) = type_.markup.and_then(|m| m.content_property) {
                return Some(content);
            }
            current = type_.base;
        }
        None
    }

    /// The properties registered on the type and its base types (styled,
    /// direct and the attached properties the type owns), each type's in
    /// declaration order, most derived type first. Initialises the type if
    /// needed, so no instance has to exist.
    pub fn properties(&'static self) -> Rc<Vec<&'static FerroProperty>> {
        crate::FerroPropertyRegistry::instance().get_registered(self)
    }

    /// Finds a property registered on the type or its base types by name.
    /// Initialises the type if needed.
    pub fn find_property(&'static self, name: &str) -> Option<&'static FerroProperty> {
        crate::FerroPropertyRegistry::instance().find_registered(self, name)
    }

    /// Adds a type to the process-wide table of known types, making it
    /// discoverable with [`find`](Self::find) and
    /// [`find_by_name`](Self::find_by_name). Registering a type again does
    /// nothing.
    ///
    /// Types register themselves when they are initialised on a thread; a
    /// crate's `register_types()` function registers all of its types up
    /// front so that types that were never used are known too.
    pub fn register(type_: &'static TypeInfo) {
        if read_registry().contains(type_) {
            return;
        }
        write_registry().insert(type_);
    }

    /// Registers several types; see [`register`](Self::register).
    pub fn register_all(types: &[&'static TypeInfo]) {
        let mut registry = write_registry();
        for type_ in types {
            registry.insert(type_);
        }
    }

    /// Declares the namespaces of a crate's modules: pairs of a Rust module
    /// path (`"ferroui_base::media"`) and the dotted namespace of the types
    /// declared in that module and its submodules (`"FerroUI.Media"`). A
    /// longer module path overrides a shorter one, down to a single file.
    pub fn register_namespaces(namespaces: &[(&'static str, &'static str)]) {
        let mut registry = write_registry();
        for &(module_path, namespace) in namespaces {
            match registry.namespaces.iter_mut().find(|(m, _)| *m == module_path) {
                Some(entry) => entry.1 = namespace,
                None => registry.namespaces.push((module_path, namespace)),
            }
        }
    }

    #[cfg(feature = "compiler-metadata")]
    /// Declares the public Rust paths of classes: pairs of a class and the
    /// path another crate names it by (`"ferroui_controls::Border"`). The
    /// declaring module ([`module_path`](Self::module_path)) is often
    /// private, so the path cannot be derived from it; a crate records the
    /// table of its generated `rust_paths.rs` ([`ferro_rust_paths!`](crate::ferro_rust_paths))
    /// in its `register_types()`. The emitter of Rust source reads them;
    /// nothing else does.
    pub fn register_rust_paths(paths: &[(&'static TypeInfo, &'static str)]) {
        let mut registry = write_registry();
        for &(type_, path) in paths {
            registry.rust_paths.insert(type_ as *const TypeInfo as usize, (type_, path));
        }
    }

    #[cfg(feature = "compiler-metadata")]
    /// The public Rust path of the class
    /// ([`register_rust_paths`](Self::register_rust_paths)), if its crate
    /// recorded one.
    pub fn rust_path(&'static self) -> Option<&'static str> {
        read_registry().rust_paths.get(&(self as *const TypeInfo as usize)).map(|entry| entry.1)
    }

    #[cfg(feature = "compiler-metadata")]
    /// Every class with a registered public Rust path, with the path, in no
    /// particular order.
    pub fn all_rust_paths() -> Vec<(&'static TypeInfo, &'static str)> {
        read_registry().rust_paths.values().copied().collect()
    }

    /// Finds a known type by namespace and name.
    pub fn find(namespace: &str, name: &str) -> Option<&'static TypeInfo> {
        let registry = read_registry();
        let candidates = registry.by_name.get(name)?;
        candidates.iter().copied().find(|t| registry.namespace_of(t.module_path) == namespace)
    }

    /// Finds the known class whose handle (`Ref<T>`) or nullable handle
    /// (`Option<Ref<T>>`) is the Rust type `handle`. The flag tells whether
    /// it is the nullable handle.
    pub fn find_by_handle(handle: TypeId) -> Option<(&'static TypeInfo, bool)> {
        read_registry().by_handle.get(&handle).copied()
    }

    /// Finds a known type by name alone. `None` if no type or more than one
    /// type (in different namespaces) has the name.
    pub fn find_by_name(name: &str) -> Option<&'static TypeInfo> {
        let registry = read_registry();
        match registry.by_name.get(name)?.as_slice() {
            [type_] => Some(type_),
            _ => None,
        }
    }

    /// Every known type, in registration order.
    pub fn registered_types() -> Vec<&'static TypeInfo> {
        read_registry().types.clone()
    }

    /// Returns true if `other` is this type or derives from it.
    pub fn is_assignable_from(&self, other: &TypeInfo) -> bool {
        let mut current = Some(other);
        while let Some(t) = current {
            if std::ptr::eq(self, t) {
                return true;
            }
            current = t.base;
        }
        false
    }

    /// Returns true if this type is `other` or derives from it.
    #[inline]
    pub fn is_subclass_of(&self, other: &TypeInfo) -> bool {
        other.is_assignable_from(self)
    }
}

impl PartialEq for TypeInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for TypeInfo {}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self as *const TypeInfo as usize).hash(state)
    }
}

impl fmt::Debug for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// The process-wide table of known types. Runtime types are immutable
/// statics, so unlike the property registry the table is not per thread.
#[derive(Default)]
struct TypeRegistry {
    types: Vec<&'static TypeInfo>,
    by_name: HashMap<&'static str, Vec<&'static TypeInfo>>,
    by_handle: HashMap<TypeId, (&'static TypeInfo, bool)>,
    namespaces: Vec<(&'static str, &'static str)>,
    /// The public Rust paths of classes, by the address of their type.
    #[cfg(feature = "compiler-metadata")]
    rust_paths: HashMap<usize, (&'static TypeInfo, &'static str)>,
}

impl TypeRegistry {
    fn contains(&self, type_: &'static TypeInfo) -> bool {
        self.by_name.get(type_.name).is_some_and(|types| types.iter().any(|t| std::ptr::eq(*t, type_)))
    }

    fn insert(&mut self, type_: &'static TypeInfo) {
        if !self.contains(type_) {
            self.types.push(type_);
            self.by_name.entry(type_.name).or_default().push(type_);
            if let Some(handles) = type_.handles {
                let [handle, nullable] = handles();
                self.by_handle.insert(handle, (type_, false));
                self.by_handle.insert(nullable, (type_, true));
            }
        }
    }

    fn namespace_of(&self, module_path: &str) -> &'static str {
        let mut best: Option<(&'static str, &'static str)> = None;
        for &(prefix, namespace) in &self.namespaces {
            let matches = match module_path.strip_prefix(prefix) {
                Some(rest) => rest.is_empty() || rest.starts_with("::"),
                None => false,
            };
            if matches && best.is_none_or(|(b, _)| prefix.len() > b.len()) {
                best = Some((prefix, namespace));
            }
        }
        best.map_or("", |(_, namespace)| namespace)
    }
}

fn type_registry() -> &'static RwLock<TypeRegistry> {
    static REGISTRY: OnceLock<RwLock<TypeRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

fn read_registry() -> std::sync::RwLockReadGuard<'static, TypeRegistry> {
    type_registry().read().unwrap_or_else(|e| e.into_inner())
}

fn write_registry() -> std::sync::RwLockWriteGuard<'static, TypeRegistry> {
    type_registry().write().unwrap_or_else(|e| e.into_inner())
}

/// A Rust type with an associated runtime [`TypeInfo`].
///
/// Implemented by all classes ([`ferro_class!`]) and by non-instantiable
/// types that own attached properties ([`ferro_static_type!`]).
pub trait StaticType: 'static {
    /// The runtime type.
    const TYPE: &'static TypeInfo;
}

/// What a type declares when it declares nothing: no default constructor,
/// no interface casts, no properties and an empty static constructor.
///
/// The class macros read the declarations of a type through the names below.
/// A type that declares one of them does so with an inherent item of the
/// same name (written by hand for [`static_constructor`](Self::static_constructor),
/// generated by [`ferro_properties!`](crate::ferro_properties) and
/// [`ferro_class_info!`] for the others), which takes precedence over this
/// trait; every other type falls back to it.
pub trait ClassDefaults {
    #[doc(hidden)]
    const __CONSTRUCTOR: Option<fn() -> Ref<FerroObject>> = None;

    #[doc(hidden)]
    const __INTERFACES: &'static [TypeOf] = &[];

    #[doc(hidden)]
    const __MARKUP: Option<&'static MarkupType> = None;

    #[doc(hidden)]
    #[inline]
    fn __register_interfaces() {}

    #[doc(hidden)]
    #[inline]
    fn __register_properties() {}

    /// The static constructor of a type: work done once per thread, after
    /// the properties of the type are registered and before its first
    /// instance exists (class handlers, `affects_render`, metadata
    /// overrides).
    ///
    /// A class declares it as an inherent function in the module of its
    /// `ferro_class!`:
    ///
    /// ```ignore
    /// impl Border {
    ///     fn static_constructor() {
    ///         Self::affects_render::<Border>(&[Self::background_property().as_property()]);
    ///     }
    /// }
    /// ```
    ///
    /// It is never called by hand: the class initialisation calls it. Use
    /// `Border::TYPE.ensure_class_init()` to force the initialisation.
    #[inline]
    fn static_constructor() {}
}

impl<T: ?Sized> ClassDefaults for T {}

/// The part of a class initialisation that is the same for every class:
/// makes the type known by name and registers its handle type `Ref<T>` with
/// the untyped value conversions (as an object handle that casts to the
/// handles of its base classes and to its nullable form). The conversions
/// are registered when the thread first uses untyped conversions.
#[doc(hidden)]
#[cold]
pub fn __register_class<T: ObjectType>() {
    TypeInfo::register(T::TYPE);
    crate::data::core::ValueTypes::register_deferred(crate::data::core::ValueTypes::register_object::<T>);
}

/// Access to the embedded base-class data of an object.
///
/// Implemented by [`ferro_class!`] for every ancestor of a class, so
/// `T: Upcast<U>` reads as "`T` is `U` or derives from it".
pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
}

/// Implemented by every class.
///
/// # Safety
///
/// Implemented only by [`ferro_class!`] (and by hand for the root class).
/// Implementors guarantee that:
///
/// * `Self` is `#[repr(C)]`-compatible with `Parent` at offset zero, i.e. a
///   pointer to `Self` is a valid pointer to every class in its ancestor
///   chain;
/// * `VTable` likewise starts with `Parent::VTable` at offset zero;
/// * values of `Self` are only created through [`instantiate`] (class structs
///   have a private `base` field and `construct`-style constructors that are
///   only meaningful inside `instantiate`).
pub unsafe trait ObjectType: StaticType + Upcast<FerroObject> + Sized {
    /// The base class. The root class is its own parent.
    type Parent: ObjectType;

    /// The virtual table of the class.
    type VTable: Copy + 'static;

    /// The virtual table used by instances whose most-derived class is `Self`.
    fn vtable() -> &'static Self::VTable;

    /// The static initialisation of the class. The first call on a thread,
    /// after doing the same for the base class:
    ///
    /// 1. makes the type known by name ([`TypeInfo::find`]);
    /// 2. registers the handle type `Ref<Self>` and the interface handles the
    ///    class declares ([`ferro_class_info!`]) with the untyped value
    ///    conversions;
    /// 3. registers the properties of the class in declaration order
    ///    ([`ferro_properties!`](crate::ferro_properties));
    /// 4. runs the static constructor of the class
    ///    ([`ClassDefaults::static_constructor`]).
    ///
    /// Later calls only read a thread-local flag.
    ///
    /// Generated by `ferro_class!`. Called by [`instantiate`], so a class
    /// that has an instance on the thread is initialised, and through
    /// [`TypeInfo::ensure_class_init`] by property accessors and the property
    /// registry, for classes that have none.
    fn register_class();
}

/// Implemented by a class `C` for every class `T` that can derive from it:
/// builds `C`'s part of the virtual table with `T`'s overrides.
pub trait Subclassable<T>: ObjectType {
    #[doc(hidden)]
    fn build_vtable() -> Self::VTable;
}

/// Reinterprets a reference to base class data as a reference to the derived
/// class `T`.
///
/// # Safety
///
/// `base` must be the base-class part of an object whose most-derived class
/// is `T` or derives from `T`.
#[doc(hidden)]
#[inline(always)]
pub unsafe fn cast_this<B, T>(base: &B) -> &T {
    &*(base as *const B as *const T)
}

/// Returns the virtual table of `T`'s parent class, viewed as the table type
/// `V` of the class that introduced the member being called.
///
/// Panics if the parent class has no `V`, i.e. when the class that introduced
/// a virtual member did not implement it itself.
#[doc(hidden)]
#[inline]
pub fn parent_vtable<T: ObjectType, V>() -> &'static V {
    // A parent class that is an ancestor of the class owning `V` has a
    // strictly smaller table (the owner adds at least the member being
    // called), so this check is exact. It is constant per instantiation.
    assert!(
        std::mem::size_of::<<T::Parent as ObjectType>::VTable>() >= std::mem::size_of::<V>(),
        "virtual member has no base implementation"
    );
    let table: *const <T::Parent as ObjectType>::VTable = <T::Parent as ObjectType>::vtable();
    // SAFETY: by the size argument above the parent class is the owner of `V`
    // or derives from it, and `ObjectType` guarantees that a class's table
    // starts with the tables of all of its ancestors.
    unsafe { &*(table as *const V) }
}

/// A strong reference to an object of class `T` or a class derived from it.
///
/// Cloning the handle shares the object. Equality and hashing are by object
/// identity.
#[repr(transparent)]
pub struct Ref<T: ObjectType> {
    ptr: Rc<dyn Any>,
    _marker: PhantomData<T>,
}

impl<T: ObjectType> Ref<T> {
    /// Converts the handle to a handle of a base class.
    #[inline]
    pub fn upcast<U: ObjectType>(self) -> Ref<U>
    where
        T: Upcast<U>,
    {
        Ref { ptr: self.ptr, _marker: PhantomData }
    }

    /// Borrows the handle as a handle of a base class, without a copy of the
    /// handle (the borrowing form of [`upcast`](Self::upcast)).
    #[inline]
    pub fn upcast_ref<U: ObjectType>(&self) -> &Ref<U>
    where
        T: Upcast<U>,
    {
        // SAFETY: `Ref` is `repr(transparent)` over its `Rc<dyn Any>`; the
        // marker is zero-sized, so `Ref<T>` and `Ref<U>` have the same layout
        // and the same valid values, and `T: Upcast<U>` makes the object a `U`,
        // which is exactly what `upcast` asserts for an owned handle.
        unsafe { &*(self as *const Ref<T> as *const Ref<U>) }
    }

    /// Returns a handle of class `U` if the object is a `U`.
    #[inline]
    pub fn cast<U: ObjectType>(&self) -> Option<Ref<U>> {
        if self.is::<U>() {
            Some(Ref { ptr: self.ptr.clone(), _marker: PhantomData })
        } else {
            None
        }
    }

    /// Converts the handle to class `U` if the object is a `U`, otherwise
    /// returns the handle unchanged.
    #[inline]
    pub fn downcast<U: ObjectType>(self) -> Result<Ref<U>, Self> {
        if self.is::<U>() {
            Ok(Ref { ptr: self.ptr, _marker: PhantomData })
        } else {
            Err(self)
        }
    }

    /// Whether the object is of class `U` or a class derived from it.
    #[inline]
    pub fn is<U: ObjectType>(&self) -> bool {
        let object: &FerroObject = (**self).upcast();
        U::TYPE.is_assignable_from(object.get_type())
    }

    /// Creates a weak reference to the object.
    #[inline]
    pub fn downgrade(&self) -> WeakRef<T> {
        WeakRef { ptr: Rc::downgrade(&self.ptr), _marker: PhantomData }
    }

    /// Whether two handles refer to the same object.
    #[inline]
    pub fn ptr_eq<U: ObjectType>(&self, other: &Ref<U>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.ptr), Rc::as_ptr(&other.ptr))
    }

    #[inline]
    pub(crate) fn from_rc(ptr: Rc<dyn Any>) -> Self {
        Ref { ptr, _marker: PhantomData }
    }
}

impl<T: ObjectType> Deref for Ref<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: a `Ref<T>` is only created by `instantiate` (for the exact
        // class), by `upcast` (statically an ancestor) and by `cast` /
        // `downcast` / `FerroObject::to_ref` (after a runtime type check), so
        // the allocation holds an object whose class is `T` or derives from
        // it, and `ObjectType` guarantees `T` is at offset zero of it.
        unsafe { &*(Rc::as_ptr(&self.ptr) as *const T) }
    }
}

impl<T: ObjectType> Clone for Ref<T> {
    #[inline]
    fn clone(&self) -> Self {
        Ref { ptr: self.ptr.clone(), _marker: PhantomData }
    }
}

impl<T: ObjectType, U: ObjectType> PartialEq<Ref<U>> for Ref<T> {
    #[inline]
    fn eq(&self, other: &Ref<U>) -> bool {
        self.ptr_eq(other)
    }
}

impl<T: ObjectType> Eq for Ref<T> {}

impl<T: ObjectType> Hash for Ref<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.ptr) as *const () as usize).hash(state)
    }
}

impl<T: ObjectType> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let object: &FerroObject = (**self).upcast();
        fmt::Debug::fmt(object, f)
    }
}

/// A weak reference to an object of class `T` or a class derived from it.
pub struct WeakRef<T: ObjectType> {
    ptr: Weak<dyn Any>,
    _marker: PhantomData<T>,
}

impl<T: ObjectType> WeakRef<T> {
    /// Returns a strong reference if the object is still alive.
    #[inline]
    pub fn upgrade(&self) -> Option<Ref<T>> {
        self.ptr.upgrade().map(|ptr| Ref { ptr, _marker: PhantomData })
    }

    /// Converts the handle to a handle of a base class.
    #[inline]
    pub fn upcast<U: ObjectType>(self) -> WeakRef<U>
    where
        T: Upcast<U>,
    {
        WeakRef { ptr: self.ptr, _marker: PhantomData }
    }

    /// Whether the weak reference points at `object`.
    #[inline]
    pub fn points_to<U: ObjectType>(&self, object: &Ref<U>) -> bool {
        std::ptr::addr_eq(self.ptr.as_ptr(), Rc::as_ptr(&object.ptr))
    }

    /// Whether two weak references point at the same object, alive or not.
    #[inline]
    pub fn ptr_eq<U: ObjectType>(&self, other: &WeakRef<U>) -> bool {
        std::ptr::addr_eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }

    #[inline]
    pub(crate) fn from_weak(ptr: Weak<dyn Any>) -> Self {
        WeakRef { ptr, _marker: PhantomData }
    }
}

impl<T: ObjectType> Clone for WeakRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        WeakRef { ptr: self.ptr.clone(), _marker: PhantomData }
    }
}

/// An optional object reference accepted by setters, convertible from a
/// handle (or a borrowed handle) of any derived class and from `None`.
pub struct Nullable<T: ObjectType>(pub Option<Ref<T>>);

impl<T: ObjectType + Upcast<U>, U: ObjectType> From<Ref<T>> for Nullable<U> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Nullable(Some(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<U>, U: ObjectType> From<&Ref<T>> for Nullable<U> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Nullable(Some(value.clone().upcast()))
    }
}

impl<U: ObjectType> From<Option<Ref<U>>> for Nullable<U> {
    #[inline]
    fn from(value: Option<Ref<U>>) -> Self {
        Nullable(value)
    }
}

/// A non-optional object reference accepted by methods, convertible from a
/// handle (or a borrowed handle) of any derived class.
pub trait IntoRef<U: ObjectType> {
    fn into_ref(self) -> Ref<U>;
}

impl<T: ObjectType + Upcast<U>, U: ObjectType> IntoRef<U> for Ref<T> {
    #[inline]
    fn into_ref(self) -> Ref<U> {
        self.upcast()
    }
}

impl<T: ObjectType + Upcast<U>, U: ObjectType> IntoRef<U> for &Ref<T> {
    #[inline]
    fn into_ref(self) -> Ref<U> {
        self.clone().upcast()
    }
}

/// Moves a constructed class value to the heap, attaches the virtual table of
/// its class and runs the `constructed` virtual member. The class (and its
/// base classes) are initialised first if this is the first instance on the
/// thread; see [`ObjectType::register_class`].
pub fn instantiate<T: ObjectType>(value: T) -> Ref<T> {
    T::register_class();
    let ptr: Rc<dyn Any> = Rc::new(value);
    let result: Ref<T> = Ref { ptr, _marker: PhantomData };
    let object: &FerroObject = (*result).upcast();
    let table: *const T::VTable = T::vtable();
    // SAFETY: `T::VTable` starts with the root class's table (`ObjectType`
    // invariant), and the object's most-derived class is `T`.
    unsafe { object.attach(Rc::downgrade(&result.ptr), table as *const ()) };
    object.constructed();
    result
}

/// Declares a class.
///
/// ```ignore
/// #[repr(C)]
/// pub struct Decorator { base: Control, /* fields */ }
///
/// // A class without new virtual members:
/// ferro_class!(Decorator: Control);
///
/// // A class that introduces virtual members:
/// ferro_class! {
///     Layoutable: Visual, virtuals LayoutableImpl: VisualImpl {
///         /// Measures the control.
///         fn measure_override(this, available_size: Size) -> Size;
///         fn arrange_override(this, final_size: Size) -> Size;
///     }
/// }
/// ```
///
/// Both forms generate `Deref<Target = Base>`, [`Upcast`] to every ancestor,
/// the `TYPE` runtime type and [`ObjectType`], including the static
/// initialisation of the class ([`ObjectType::register_class`]). What the
/// initialisation does is taken from the other declarations of the class,
/// each optional and found by the macro without being named here:
///
/// * `ferro_properties! { impl Decorator { .. } }`, the property definitions;
/// * `fn static_constructor()`, an inherent function declared in the same
///   module ([`ClassDefaults::static_constructor`]);
/// * `ferro_class_info!(Decorator { new: Decorator::new, interfaces: [..] })`,
///   the default constructor and the interface handles the class converts
///   to.
///
/// The second form also generates:
///
/// * `LayoutableVTable`, the table of the new members;
/// * `trait LayoutableImpl`, with one associated function per member taking
///   `this: &Self`. Every class deriving from `Layoutable` implements it
///   (`impl LayoutableImpl for Border {}`) and overrides what it needs;
/// * `LayoutableImplExt::parent_<member>`, to call the base implementation
///   from an override;
/// * `Layoutable::<member>(&self, ..)`, which performs the virtual call.
///
/// The declaring class must implement every member itself; these are the
/// base implementations.
#[macro_export]
macro_rules! ferro_class {
    // What the type declares, in the order of a static initialisation:
    // interface casts, properties (field initialisers), static constructor.
    // Inherent items of the type win over the `ClassDefaults` fallbacks.
    (@class_members $name:ident) => {{
        #[allow(unused_imports)]
        use $crate::ClassDefaults as _;
        $crate::data::core::ValueTypes::register_deferred(<$name>::__register_interfaces);
        <$name>::__register_properties();
        <$name>::static_constructor();
    }};

    (@register_class $name:ident : $base:ident) => {
        #[inline]
        fn register_class() {
            ::std::thread_local! {
                static REGISTERED: ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
            }
            #[cold]
            #[inline(never)]
            fn init() {
                <$base as $crate::ObjectType>::register_class();
                $crate::__register_class::<$name>();
                $crate::ferro_class!(@class_members $name);
            }
            if !REGISTERED.get() {
                // Set first: the initialisation may create instances of the
                // class or call its property accessors.
                REGISTERED.set(true);
                init();
            }
        }
    };

    (@common $name:ident : $base:ident) => {
        const _: () = assert!(
            ::std::mem::offset_of!($name, base) == 0,
            "class structs must be #[repr(C)] with `base` as their first field"
        );

        impl ::std::ops::Deref for $name {
            type Target = $base;
            #[inline]
            fn deref(&self) -> &$base {
                &self.base
            }
        }

        impl<T: ?Sized> $crate::Upcast<T> for $name
        where
            $base: $crate::Upcast<T>,
        {
            #[inline]
            fn upcast(&self) -> &T {
                $crate::Upcast::upcast(&self.base)
            }
        }

        impl $crate::Upcast<$name> for $name {
            #[inline]
            fn upcast(&self) -> &$name {
                self
            }
        }

        impl $crate::StaticType for $name {
            const TYPE: &'static $crate::TypeInfo = {
                static TYPE: $crate::TypeInfo = $crate::TypeInfo::new(
                    stringify!($name),
                    Some(<$base as $crate::StaticType>::TYPE),
                )
                .with_module_path(::std::module_path!())
                .with_class_init(<$name as $crate::ObjectType>::register_class)
                .with_constructor({
                    #[allow(unused_imports)]
                    use $crate::ClassDefaults as _;
                    <$name>::__CONSTRUCTOR
                })
                .with_handles(|| {
                    [
                        ::std::any::TypeId::of::<$crate::Ref<$name>>(),
                        ::std::any::TypeId::of::<::std::option::Option<$crate::Ref<$name>>>(),
                    ]
                })
                .with_interfaces({
                    #[allow(unused_imports)]
                    use $crate::ClassDefaults as _;
                    <$name>::__INTERFACES
                })
                .with_markup({
                    #[allow(unused_imports)]
                    use $crate::ClassDefaults as _;
                    <$name>::__MARKUP
                });
                &TYPE
            };
        }

        impl $name {
            /// The runtime type of this class.
            pub const TYPE: &'static $crate::TypeInfo = <$name as $crate::StaticType>::TYPE;

            /// Returns a strong reference to this object.
            #[inline]
            pub fn to_ref(&self) -> $crate::Ref<$name> {
                $crate::FerroObject::ref_of(self)
            }
        }
    };

    ($name:ident : $base:ident) => {
        $crate::ferro_class!(@common $name: $base);

        // SAFETY: the offset assertion above proves the layout requirement;
        // the table type is the parent's.
        unsafe impl $crate::ObjectType for $name {
            type Parent = $base;
            type VTable = <$base as $crate::ObjectType>::VTable;

            fn vtable() -> &'static Self::VTable {
                static VTABLE: ::std::sync::OnceLock<<$base as $crate::ObjectType>::VTable> =
                    ::std::sync::OnceLock::new();
                VTABLE.get_or_init(<$name as $crate::Subclassable<$name>>::build_vtable)
            }

            $crate::ferro_class!(@register_class $name: $base);
        }

        impl<T> $crate::Subclassable<T> for $name
        where
            $base: $crate::Subclassable<T>,
        {
            #[inline]
            fn build_vtable() -> Self::VTable {
                <$base as $crate::Subclassable<T>>::build_vtable()
            }
        }
    };

    (
        $name:ident : $base:ident, virtuals $impl_trait:ident : $parent_impl:path {
            $(
                $(#[$meta:meta])*
                fn $method:ident(this $(, $arg:ident : $arg_ty:ty)* $(,)?) $(-> $ret:ty)?;
            )*
        }
    ) => {
        $crate::ferro_class!(@common $name: $base);

        $crate::__paste! {
            /// The virtual table of the class.
            #[derive(Clone, Copy)]
            #[repr(C)]
            pub struct [<$name VTable>] {
                pub base: <$base as $crate::ObjectType>::VTable,
                $(pub $method: fn(&$name $(, $arg_ty)*) $(-> $ret)?,)*
            }

            // SAFETY: the offset assertion above proves the layout
            // requirement; the table is `#[repr(C)]` and starts with the
            // parent's table.
            unsafe impl $crate::ObjectType for $name {
                type Parent = $base;
                type VTable = [<$name VTable>];

                fn vtable() -> &'static Self::VTable {
                    static VTABLE: ::std::sync::OnceLock<[<$name VTable>]> = ::std::sync::OnceLock::new();
                    VTABLE.get_or_init(<$name as $crate::Subclassable<$name>>::build_vtable)
                }

                $crate::ferro_class!(@register_class $name: $base);
            }

            /// The overridable members introduced by the class.
            pub trait $impl_trait: $parent_impl + $crate::Upcast<$name> {
                $(
                    $(#[$meta])*
                    #[allow(unused_variables)]
                    fn $method(this: &Self $(, $arg: $arg_ty)*) $(-> $ret)? {
                        <Self as [<$impl_trait Ext>]>::[<parent_ $method>](this $(, $arg)*)
                    }
                )*
            }

            /// Access to the base implementations of the overridable members.
            pub trait [<$impl_trait Ext>]: $impl_trait {
                $(
                    /// Calls the base class implementation of the member.
                    #[inline]
                    fn [<parent_ $method>](this: &Self $(, $arg: $arg_ty)*) $(-> $ret)? {
                        let table = $crate::parent_vtable::<Self, [<$name VTable>]>();
                        (table.$method)($crate::Upcast::<$name>::upcast(this) $(, $arg)*)
                    }
                )*
            }

            impl<T: $impl_trait> [<$impl_trait Ext>] for T {}

            impl<T: $impl_trait> $crate::Subclassable<T> for $name
            where
                $base: $crate::Subclassable<T>,
            {
                fn build_vtable() -> [<$name VTable>] {
                    [<$name VTable>] {
                        base: <$base as $crate::Subclassable<T>>::build_vtable(),
                        $(
                            // SAFETY: this table is only attached to objects
                            // whose most-derived class is `T`.
                            $method: |this $(, $arg)*| T::$method(unsafe { $crate::cast_this::<$name, T>(this) } $(, $arg)*),
                        )*
                    }
                }
            }

            impl $name {
                $(
                    $(#[$meta])*
                    #[inline]
                    pub fn $method(&self $(, $arg: $arg_ty)*) $(-> $ret)? {
                        $crate::perf_count_virtual!(concat!(stringify!($name), "::", stringify!($method)));
                        let table = $crate::FerroObject::vtable_of::<$name>(self);
                        (table.$method)(self $(, $arg)*)
                    }
                )*
            }
        }
    };
}

/// Declares the facts about a class that untyped code (markup, untyped
/// bindings) needs and cannot derive: its default constructor, the
/// interface handles its handle converts to and its markup metadata. Every
/// part is optional; the parts may be written in any order.
///
/// ```ignore
/// ferro_class!(SolidColorBrush: Brush);
/// ferro_class_info!(SolidColorBrush {
///     new: SolidColorBrush::new,
///     interfaces: [Rc<dyn IBrush>],
/// });
///
/// ferro_class!(Panel: Control);
/// ferro_class_info!(Panel {
///     new: Panel::new,
///     markup: {
///         content: Children,
///         properties: [
///             Children: Rc<Controls> { get: Panel::children },
///         ],
///     },
/// });
/// ```
///
/// * `new` is a function without parameters that returns the handle of a new
///   instance. It becomes [`TypeInfo::default_constructor`].
/// * `interfaces` lists the handle types `I` that `Ref<Class>` converts to
///   with `Into<I>`, or, as `I => function`, with the given
///   `fn(Ref<Class>) -> I` (for an interface of another crate, for which
///   the conversion trait cannot be implemented). They are registered with the untyped value conversions
///   when the class is initialised, for the class and every class deriving
///   from it (a more derived class that lists the same interface takes
///   precedence for its own instances), and are listed by
///   [`TypeInfo::interfaces`].
/// * `markup` states what markup needs beyond the registered properties of
///   the class, which need no declaration: the content property, plain
///   (non-registered) properties and collections, constructors with
///   arguments, methods, static values, events and attributes. The parts are
///   the ones of [`ferro_markup_type!`](crate::ferro_markup_type) (without
///   `handles`, `this` and `base`, which follow from the class); instance
///   members are called with `&Ref<Class>`, so `Class::method` paths work as
///   callables. It becomes [`TypeInfo::markup`]: constant data that costs
///   nothing until it is read.
#[macro_export]
macro_rules! ferro_class_info {
    (@new $name:ident $constructor:expr) => {
        impl $name {
            #[doc(hidden)]
            pub const __CONSTRUCTOR: ::std::option::Option<fn() -> $crate::Ref<$crate::FerroObject>> =
                ::std::option::Option::Some(|| $crate::Ref::<$name>::upcast($constructor()));
        }
    };

    (@cast) => {
        |object| ::std::convert::Into::into(object)
    };

    (@cast $cast:expr) => {
        $cast
    };

    (@interfaces $name:ident [$($interface:ty $(=> $cast:expr)?),* $(,)?]) => {
        impl $name {
            #[doc(hidden)]
            pub const __INTERFACES: &'static [$crate::metadata::TypeOf] =
                &[$(|| $crate::data::core::ValueType::of::<$interface>()),*];

            #[doc(hidden)]
            pub fn __register_interfaces() {
                $(
                    $crate::data::core::ValueTypes::register_interface::<$name, $interface>(
                        $crate::ferro_class_info!(@cast $($cast)?),
                    );
                )*
            }
        }
    };

    (@markup $name:ident { $($body:tt)* }) => {
        impl $name {
            $crate::__ferro_markup_functions!($crate::__ferro_markup_fns!($crate::Ref<$name>, $crate::Ref<$name>; $($body)*););

            #[doc(hidden)]
            pub const __MARKUP: ::std::option::Option<&'static $crate::metadata::MarkupType> = {
                static MARKUP: $crate::metadata::MarkupType = {
                    #[allow(unused_mut)]
                    let mut markup = $crate::metadata::MarkupType::new(
                        ::std::stringify!($name),
                        $crate::metadata::MarkupTypeKind::Class,
                        ::std::module_path!(),
                    );
                    markup.handles = &[
                        || $crate::data::core::ValueType::of::<$crate::Ref<$name>>(),
                        || $crate::data::core::ValueType::of::<::std::option::Option<$crate::Ref<$name>>>(),
                    ];
                    markup.type_info = ::std::option::Option::Some(|| <$name as $crate::StaticType>::TYPE);
                    markup.interfaces = {
                        #[allow(unused_imports)]
                        use $crate::ClassDefaults as _;
                        <$name>::__INTERFACES
                    };
                    $crate::__ferro_compiler_metadata!(
                        markup.this = ::std::option::Option::Some(|| $crate::data::core::ValueType::of::<$crate::Ref<$name>>());
                        markup.value = ::std::option::Option::Some(|| $crate::data::core::ValueType::of::<$crate::Ref<$name>>());
                    );
                    $crate::__ferro_markup_items!(markup, $crate::Ref<$name>; $($body)*);
                    markup
                };
                ::std::option::Option::Some(&MARKUP)
            };
        }
    };

    (@parts $name:ident;) => {};

    (@parts $name:ident; new: $constructor:expr $(, $($rest:tt)*)?) => {
        $crate::ferro_class_info!(@new $name $constructor);
        $crate::ferro_class_info!(@parts $name; $($($rest)*)?);
    };

    (@parts $name:ident; interfaces: $interfaces:tt $(, $($rest:tt)*)?) => {
        $crate::ferro_class_info!(@interfaces $name $interfaces);
        $crate::ferro_class_info!(@parts $name; $($($rest)*)?);
    };

    (@parts $name:ident; markup: $markup:tt $(, $($rest:tt)*)?) => {
        $crate::ferro_class_info!(@markup $name $markup);
        $crate::ferro_class_info!(@parts $name; $($($rest)*)?);
    };

    ($name:ident { $($parts:tt)* }) => {
        $crate::ferro_class_info!(@parts $name; $($parts)*);
    };
}

/// Declares the runtime type of a type that is not a class: a type that
/// owns attached properties and cannot be instantiated as an object.
///
/// ```ignore
/// pub struct KeyboardNavigation;
/// ferro_static_type!(KeyboardNavigation);
///
/// ferro_properties! {
///     impl KeyboardNavigation {
///         pub fn tab_index_property() -> AttachedProperty<i32> { .. }
///     }
/// }
/// ```
///
/// Unlike a hand-written `impl StaticType`, the runtime type carries the
/// static initialisation of the type (its `ferro_properties!` block and its
/// `fn static_constructor()`, as for a class), so its properties are found
/// by owner type and name before any accessor was called, and it knows its
/// module, hence its namespace.
#[macro_export]
macro_rules! ferro_static_type {
    ($name:ident) => {
        impl $crate::StaticType for $name {
            const TYPE: &'static $crate::TypeInfo = {
                fn class_init() {
                    ::std::thread_local! {
                        static REGISTERED: ::std::cell::Cell<bool> = const { ::std::cell::Cell::new(false) };
                    }
                    if !REGISTERED.replace(true) {
                        $crate::TypeInfo::register(<$name as $crate::StaticType>::TYPE);
                        $crate::ferro_class!(@class_members $name);
                    }
                }
                static TYPE: $crate::TypeInfo = $crate::TypeInfo::new(stringify!($name), None)
                    .with_module_path(::std::module_path!())
                    .with_class_init(class_init);
                &TYPE
            };
        }

        impl $name {
            /// The runtime type of this type.
            #[allow(dead_code)]
            pub const TYPE: &'static $crate::TypeInfo = <$name as $crate::StaticType>::TYPE;
        }
    };
}

/// Implements class `Impl` traits with no overrides.
///
/// ```ignore
/// ferro_impl_classes!(Border: FerroObjectImpl, StyledElementImpl, VisualImpl);
/// ```
#[macro_export]
macro_rules! ferro_impl_classes {
    ($name:ty : $($trait_:path),+ $(,)?) => {
        $(impl $trait_ for $name {})+
    };
}
