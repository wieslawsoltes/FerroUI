//! Runtime knowledge about value types for the untyped parts of the binding
//! system.
//!
//! An untyped value ([`BoxedValue`]) holds exactly one Rust type and offers
//! no way to ask "is this nullable", "can this be shown as text" or "can this
//! be converted to the target property's type". The managed runtime answers
//! those questions through reflection; here they are answered by a per-thread
//! table keyed by [`TypeId`], filled by explicit registration. Typed
//! (compiled) bindings whose source and target types match never consult the
//! table.

use crate::{AnyValue, BoxedValue, FerroObject, ObjectType, PropertyValue, Ref, StyledElement, TypeInfo, Upcast};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

/// Identifies the type of a value: the equivalent of a runtime type handle
/// for values that are not class instances.
#[derive(Clone, Copy)]
pub struct ValueType {
    id: TypeId,
    name: &'static str,
}

impl ValueType {
    /// The value type of `T`.
    #[inline]
    pub fn of<T: 'static>() -> Self {
        Self { id: TypeId::of::<T>(), name: std::any::type_name::<T>() }
    }

    /// Creates a value type from its parts.
    #[inline]
    pub fn new(id: TypeId, name: &'static str) -> Self {
        Self { id, name }
    }

    /// The value type of the contents of an untyped value.
    #[inline]
    pub fn of_value(value: &dyn AnyValue) -> Self {
        Self { id: value_type_id(&*value), name: value.type_name() }
    }

    /// The "any value" type: the type of targets that accept every value
    /// unconverted.
    #[inline]
    pub fn object() -> Self {
        Self::of::<Option<BoxedValue>>()
    }

    #[inline]
    pub fn id(&self) -> TypeId {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    #[inline]
    pub fn is<T: 'static>(&self) -> bool {
        self.id == TypeId::of::<T>()
    }

    /// Whether this is the "any value" type (`Option<BoxedValue>` or
    /// `BoxedValue`).
    #[inline]
    pub fn is_object(&self) -> bool {
        self.is::<Option<BoxedValue>>() || self.is::<BoxedValue>()
    }

    /// Whether this is the string type.
    #[inline]
    pub fn is_string(&self) -> bool {
        self.is::<String>() || self.is::<Option<String>>()
    }
}

impl PartialEq for ValueType {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ValueType {}

impl fmt::Debug for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// The type ID of the contents of an untyped value.
#[inline]
pub(crate) fn value_type_id(value: &dyn AnyValue) -> TypeId {
    <dyn std::any::Any>::type_id(value.as_any())
}

/// A hasher for `TypeId` keys: type IDs are already well distributed.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(b);
        }
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = self.0.rotate_left(32) ^ i;
    }
}

type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

type ConvertFn = Rc<dyn Fn(&BoxedValue) -> Option<BoxedValue>>;
type AnyConvertFn = Rc<dyn Fn(&dyn std::any::Any) -> Option<BoxedValue>>;
type DisplayFn = fn(&dyn AnyValue) -> String;
type UnwrapFn = fn(&dyn AnyValue) -> Option<BoxedValue>;
type NullFn = fn() -> BoxedValue;
type ObjectFn = fn(&dyn AnyValue) -> Option<Ref<FerroObject>>;
type FromObjectFn = fn(Ref<FerroObject>) -> Option<BoxedValue>;
type InterfaceFn = Rc<dyn Fn(&Ref<FerroObject>) -> Option<BoxedValue>>;

#[derive(Default)]
struct Registry {
    display: IdMap<TypeId, DisplayFn>,
    /// `Option<T>` → unwraps to the contained `T`.
    nullable: IdMap<TypeId, UnwrapFn>,
    /// `Option<T>` → a boxed `None`.
    null_values: IdMap<TypeId, NullFn>,
    objects: IdMap<TypeId, (ObjectFn, FromObjectFn, &'static TypeInfo)>,
    /// Model types: values whose box is the shared object itself.
    references: IdMap<TypeId, ()>,
    /// `Rc<T>` of a model type `T` → `T`.
    reference_handles: IdMap<TypeId, TypeId>,
    /// A model type `T` → `Rc<T>`.
    reference_objects: IdMap<TypeId, TypeId>,
    conversions: IdMap<(TypeId, TypeId), ConvertFn>,
    /// The conversions that are assignability casts: the value is the same
    /// value, seen as a wider type.
    casts: IdMap<(TypeId, TypeId), ConvertFn>,
    /// `Option<T>` → the type of the value it unwraps to.
    nullable_inner: IdMap<TypeId, ValueType>,
    /// Conversions applied to borrowed untyped values when a property is
    /// set with a value that is not of its exact type.
    any_conversions: IdMap<(TypeId, TypeId), AnyConvertFn>,
    /// Interface handle type → the classes whose handles convert to it.
    interfaces: IdMap<TypeId, Vec<(&'static TypeInfo, InterfaceFn)>>,
}

/// A registered cast of a reference type that applies to another form of
/// the source or of the target: the cast between the forms of the source
/// (if needed), the registered cast, the cast between the forms of the
/// target (if needed).
type ReferenceCast = (Option<ConvertFn>, ConvertFn, Option<ConvertFn>);

impl Registry {
    /// The other form of a reference type: the handle `Rc<T>` of the object
    /// form `T`, and the reverse.
    fn other_reference_form(&self, id: TypeId) -> Option<TypeId> {
        self.reference_handles.get(&id).or_else(|| self.reference_objects.get(&id)).copied()
    }

    /// The assignability cast registered between any form of the reference
    /// type `from` (the object `A`, its handle `Rc<A>`) and any form of
    /// `to`, whichever forms it was registered for.
    fn reference_cast(&self, from: TypeId, to: TypeId) -> Option<ReferenceCast> {
        let from_other = self.other_reference_form(from);
        let to_other = self.other_reference_form(to);
        if from_other.is_none() && to_other.is_none() {
            return None;
        }
        for source in [Some(from), from_other].into_iter().flatten() {
            for target in [Some(to), to_other].into_iter().flatten() {
                if (source, target) == (from, to) || source == target {
                    continue;
                }
                let Some(cast) = self.casts.get(&(source, target)) else { continue };
                let before = match source == from {
                    true => None,
                    false => Some(self.casts.get(&(from, source))?.clone()),
                };
                let after = match target == to {
                    true => None,
                    false => Some(self.casts.get(&(target, to))?.clone()),
                };
                return Some((before, cast.clone(), after));
            }
        }
        None
    }

    /// The conversion to the interface handle `interface` declared by the
    /// most derived class that `class` is or derives from.
    fn interface_cast(&self, interface: TypeId, class: &'static TypeInfo) -> Option<&InterfaceFn> {
        let mut best: Option<&(&'static TypeInfo, InterfaceFn)> = None;
        for entry in self.interfaces.get(&interface)? {
            if entry.0.is_assignable_from(class) && best.is_none_or(|b| b.0.is_assignable_from(entry.0)) {
                best = Some(entry);
            }
        }
        best.map(|entry| &entry.1)
    }
}

thread_local! {
    static REGISTRY: RefCell<Option<Registry>> = const { RefCell::new(None) };
    /// Registrations made before the table was first used on the thread.
    static DEFERRED: RefCell<Vec<fn()>> = const { RefCell::new(Vec::new()) };
}

/// Registration functions that apply to every thread ([`ValueTypes::register_global`]).
static GLOBAL: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());
/// The length of [`GLOBAL`], readable without the lock.
static GLOBAL_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

thread_local! {
    /// How many of the [`GLOBAL`] registrations ran on the thread.
    static GLOBAL_APPLIED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Runs the process-wide registrations the thread has not run yet.
#[cold]
fn apply_global(applied: usize) {
    let pending: Vec<fn()> = {
        let global = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
        // Set first: a registration uses the table itself.
        GLOBAL_APPLIED.set(global.len());
        global[applied.min(global.len())..].to_vec()
    };
    for register in pending {
        register();
    }
}

fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    let initialized = REGISTRY.with(|r| r.borrow().is_some());
    if !initialized {
        REGISTRY.with(|r| *r.borrow_mut() = Some(Registry::default()));
        register_defaults();
        for register in DEFERRED.take() {
            register();
        }
    }
    let applied = GLOBAL_APPLIED.get();
    if applied != GLOBAL_LEN.load(std::sync::atomic::Ordering::Acquire) {
        apply_global(applied);
    }
    REGISTRY.with(|r| f(r.borrow_mut().as_mut().expect("registry is initialized")))
}

// The insertions of the registration functions. The registration functions
// are instantiated once per registered type; what they do with the table
// does not depend on the type, so it lives here, compiled once.

fn insert_display(id: TypeId, display: DisplayFn) {
    with_registry(|r| {
        r.display.insert(id, display);
    });
}

fn insert_nullable(id: TypeId, unwrap: UnwrapFn, null: NullFn, inner: ValueType) {
    with_registry(|r| {
        r.nullable.insert(id, unwrap);
        r.null_values.insert(id, null);
        r.nullable_inner.insert(id, inner);
    });
}

fn insert_cast(key: (TypeId, TypeId), cast: ConvertFn) {
    with_registry(|r| {
        r.conversions.insert(key, cast.clone());
        r.casts.insert(key, cast);
    });
}

fn insert_conversion(key: (TypeId, TypeId), convert: ConvertFn) {
    with_registry(|r| {
        r.conversions.insert(key, convert);
    });
}

fn insert_object(id: TypeId, as_object: ObjectFn, from_object: FromObjectFn, class: &'static TypeInfo) {
    with_registry(|r| {
        r.objects.insert(id, (as_object, from_object, class));
    });
}

fn insert_interface(interface: TypeId, class: &'static TypeInfo, convert: InterfaceFn) {
    with_registry(|r| {
        let entries = r.interfaces.entry(interface).or_default();
        match entries.iter_mut().find(|e| std::ptr::eq(e.0, class)) {
            Some(entry) => entry.1 = convert,
            None => entries.push((class, convert)),
        }
    });
}

/// The per-thread table of value type knowledge used by untyped bindings.
pub struct ValueTypes;

impl ValueTypes {
    /// Runs `register` (a function that makes registrations with this table)
    /// now if the table is in use on the thread, and otherwise when it is
    /// first used. This is how classes register their handle types: a
    /// thread that never converts untyped values does not build the table.
    pub fn register_deferred(register: fn()) {
        if REGISTRY.with(|r| r.borrow().is_some()) {
            register();
        } else {
            DEFERRED.with(|d| d.borrow_mut().push(register));
        }
    }

    /// Adds a registration function that applies to EVERY thread: it runs on
    /// each thread when the thread next uses the table (the calling thread
    /// included), whether the thread first used the table before or after
    /// this call. Adding the same function again does nothing.
    ///
    /// This is how a crate registers the value types it declares metadata
    /// for (from its `register_types()`): the per-thread table then knows
    /// them on whatever thread untyped values are converted, with no
    /// per-thread call.
    pub fn register_global(register: fn()) {
        let mut global = GLOBAL.lock().unwrap_or_else(|e| e.into_inner());
        if !global.iter().any(|f| std::ptr::fn_addr_eq(*f, register)) {
            global.push(register);
            GLOBAL_LEN.store(global.len(), std::sync::atomic::Ordering::Release);
        }
    }

    /// Registers `T` as convertible to text through its [`fmt::Display`]
    /// implementation (the `ToString()` of the value), and to `String`.
    pub fn register_display<T: fmt::Display + PartialEq + 'static>() {
        fn display<T: fmt::Display + 'static>(value: &dyn AnyValue) -> String {
            value.downcast_ref::<T>().map(T::to_string).unwrap_or_default()
        }
        insert_display(TypeId::of::<T>(), display::<T>);
    }

    /// Registers `Option<T>` as the nullable form of `T`: untyped bindings
    /// treat `None` as null and `Some(v)` as `v`, and convert a `T` (or null)
    /// to `Option<T>` when a target requires it.
    pub fn register_nullable<T: PropertyValue>() {
        fn unwrap<T: PropertyValue>(value: &dyn AnyValue) -> Option<BoxedValue> {
            match value.downcast_ref::<Option<T>>() {
                Some(Some(v)) => Some(Rc::new(v.clone())),
                _ => None,
            }
        }
        fn null<T: PropertyValue>() -> BoxedValue {
            Rc::new(Option::<T>::None)
        }
        insert_nullable(TypeId::of::<Option<T>>(), unwrap::<T>, null::<T>, ValueType::of::<T>());
        Self::register_cast::<T, Option<T>>(|v| Some(v.clone()));
    }

    /// Registers an assignability cast: values of `TFrom` are values of
    /// `TTo` (the same value, seen as a wider type), as opposed to values
    /// that can merely be converted to it. A cast is also a conversion.
    pub fn register_cast<TFrom: 'static, TTo: PropertyValue>(cast: impl Fn(&TFrom) -> TTo + 'static) {
        let cast: ConvertFn =
            Rc::new(move |v: &BoxedValue| v.downcast_ref::<TFrom>().map(|v| Rc::new(cast(v)) as BoxedValue));
        insert_cast((TypeId::of::<TFrom>(), TypeId::of::<TTo>()), cast);
    }

    /// Registers a conversion between two value types. Returning `None`
    /// means the value cannot be converted.
    pub fn register_conversion<TFrom: 'static, TTo: PropertyValue>(convert: impl Fn(&TFrom) -> Option<TTo> + 'static) {
        insert_conversion(
            (TypeId::of::<TFrom>(), TypeId::of::<TTo>()),
            Rc::new(move |v: &BoxedValue| v.downcast_ref::<TFrom>().and_then(&convert).map(|v| Rc::new(v) as BoxedValue)),
        );
    }

    /// Registers the conversion of text to `T` through `parse`, the
    /// equivalent of a type converter from string.
    pub fn register_parse<T: PropertyValue>(parse: fn(&str) -> Option<T>) {
        Self::register_conversion::<String, T>(move |s| parse(s));
    }

    /// Registers the handle type of class `T` (`Ref<T>`), so that untyped
    /// bindings recognise values of it as objects with registered properties
    /// and can convert them to base class handles and nullable handles.
    pub fn register_object<T: ObjectType>() {
        fn as_object<T: ObjectType>(value: &dyn AnyValue) -> Option<Ref<FerroObject>> {
            value.downcast_ref::<Ref<T>>().map(|v| FerroObject::to_ref(Upcast::<FerroObject>::upcast(&**v)))
        }
        fn from_object<T: ObjectType>(value: Ref<FerroObject>) -> Option<BoxedValue> {
            value.cast::<T>().map(|v| Rc::new(v) as BoxedValue)
        }
        insert_object(TypeId::of::<Ref<T>>(), as_object::<T>, from_object::<T>, T::TYPE);
        Self::register_nullable::<Ref<T>>();
        Self::register_conversion::<Ref<FerroObject>, Ref<T>>(|o| o.cast::<T>());
        Self::register_conversion::<Ref<FerroObject>, Option<Ref<T>>>(|o| o.cast::<T>().map(Some));
    }

    /// Registers the element reference type of class `T`
    /// ([`ElementRef<T>`](crate::ElementRef)): the value type of properties
    /// that point at an element they do not own. Handles (`Ref<T>`,
    /// `Option<Ref<T>>`, object handles) convert to it and it converts back
    /// to handles, in bindings and in untyped property access, so that
    /// such a property behaves like one of type `Ref<T>`.
    pub fn register_element_ref<T: ObjectType>() {
        use crate::ElementRef;

        Self::register_nullable::<ElementRef<T>>();
        Self::register_cast::<Ref<T>, ElementRef<T>>(|o| ElementRef::of(o));
        Self::register_cast::<Ref<T>, Option<ElementRef<T>>>(|o| Some(ElementRef::of(o)));
        Self::register_cast::<Option<Ref<T>>, Option<ElementRef<T>>>(|o| ElementRef::from_nullable(o.clone()));
        Self::register_conversion::<Ref<FerroObject>, ElementRef<T>>(|o| o.cast::<T>().map(|o| ElementRef::of(&o)));
        Self::register_conversion::<Ref<FerroObject>, Option<ElementRef<T>>>(|o| {
            o.cast::<T>().map(|o| Some(ElementRef::of(&o)))
        });
        Self::register_conversion::<ElementRef<T>, Ref<T>>(|e| e.get());
        Self::register_cast::<ElementRef<T>, Option<Ref<T>>>(|e| e.get());
        Self::register_cast::<Option<ElementRef<T>>, Option<Ref<T>>>(|e| ElementRef::resolve(e));

        fn any<TFrom: 'static, TTo: PropertyValue>(convert: fn(&TFrom) -> TTo) -> ((TypeId, TypeId), AnyConvertFn) {
            (
                (TypeId::of::<TFrom>(), TypeId::of::<TTo>()),
                Rc::new(move |v: &dyn std::any::Any| v.downcast_ref::<TFrom>().map(|v| Rc::new(convert(v)) as BoxedValue)),
            )
        }
        with_registry(|r| {
            r.any_conversions.extend([
                any::<Ref<T>, Option<ElementRef<T>>>(|o| Some(ElementRef::of(o))),
                any::<Option<Ref<T>>, Option<ElementRef<T>>>(|o| ElementRef::from_nullable(o.clone())),
                any::<ElementRef<T>, Option<ElementRef<T>>>(|e| Some(e.clone())),
            ]);
        });
    }

    /// Converts a borrowed untyped value to the value type `target` of a
    /// property that was given a value of another type, if a conversion for
    /// untyped property access is registered (element references accept
    /// handles this way).
    pub fn try_convert_for_property(value: &dyn std::any::Any, target: TypeId) -> Option<BoxedValue> {
        let key = (<dyn std::any::Any>::type_id(value), target);
        let f = with_registry(|r| r.any_conversions.get(&key).cloned());
        f.and_then(|f| f(value))
    }

    /// Registers the interface handle type `I` (for example
    /// `Rc<dyn IBrush>`) as one that handles of class `T`, and of every
    /// class deriving from it, convert to. The conversion is an
    /// assignability cast, like the cast of a handle to a base class handle,
    /// and also applies to the nullable form `Option<I>`.
    ///
    /// Classes state their interfaces with `ferro_class_info!`, which calls
    /// this when the class is initialised. When several classes of a chain
    /// register the same interface, the most derived one converts.
    pub fn register_interface<T: ObjectType, I: PropertyValue>(cast: fn(Ref<T>) -> I) {
        let convert: InterfaceFn =
            Rc::new(move |o: &Ref<FerroObject>| o.cast::<T>().map(|o| Rc::new(cast(o)) as BoxedValue));
        insert_interface(TypeId::of::<I>(), T::TYPE, convert);
        Self::register_nullable::<I>();
    }

    /// Converts an object to the interface handle type `target` (or its
    /// nullable form), if the class of the object declares the interface.
    fn object_to_interface(object: &Ref<FerroObject>, target: ValueType) -> Option<BoxedValue> {
        let class = object.get_type();
        let (cast, wrap) = with_registry(|r| {
            if let Some(cast) = r.interface_cast(target.id(), class) {
                return (Some(cast.clone()), None);
            }
            let Some(inner) = r.nullable_inner.get(&target.id()) else {
                return (None, None);
            };
            (r.interface_cast(inner.id(), class).cloned(), r.casts.get(&(inner.id(), target.id())).cloned())
        });
        let value = cast?(object)?;
        match wrap {
            Some(wrap) => wrap(&value),
            None if value_type_id(&*value) == target.id() => Some(value),
            None => None,
        }
    }

    /// Registers the conversion of handles of class `T` to handles of its
    /// base class `TBase`.
    pub fn register_upcast<T: ObjectType + Upcast<TBase>, TBase: ObjectType>() {
        Self::register_cast::<Ref<T>, Ref<TBase>>(|o| o.clone().upcast());
        Self::register_cast::<Ref<T>, Option<Ref<TBase>>>(|o| Some(o.clone().upcast()));
    }

    /// Normalises a value read from a property for use in a binding chain:
    /// a nullable (`Option<T>`) value becomes null or the contained value.
    pub fn normalize(value: BoxedValue) -> Option<BoxedValue> {
        let mut value = value;
        loop {
            let id = value_type_id(&*value);
            if id == TypeId::of::<Option<BoxedValue>>() {
                match value.downcast_ref::<Option<BoxedValue>>() {
                    Some(Some(inner)) => {
                        let inner = inner.clone();
                        value = inner;
                        continue;
                    }
                    _ => return None,
                }
            }
            let unwrap = with_registry(|r| r.nullable.get(&id).copied());
            return match unwrap {
                Some(unwrap) => unwrap(&*value),
                None => Some(value),
            };
        }
    }

    /// The text form of a value (its `ToString()`): its registered display
    /// form; without one, the dotted full name of its type (the run-time
    /// class of an object of the class hierarchy, the name the metadata of
    /// the type states) or the name of the member for a value of an
    /// enumeration with metadata; only then the Rust type name. Null is
    /// `"(null)"`.
    pub fn to_display_string(value: Option<&BoxedValue>) -> String {
        match value {
            None => "(null)".to_string(),
            Some(v) => Self::try_to_string(&**v).unwrap_or_else(|| (**v).type_name().to_string()),
        }
    }

    /// The text form of a value, if its type has one: its registered
    /// display form, or else the name [`to_display_string`](Self::to_display_string)
    /// describes for a type that is a class of the object model or has
    /// registered metadata. `None` for a type nothing is known about.
    pub fn try_to_string(value: &dyn AnyValue) -> Option<String> {
        if let Some(s) = value.downcast_ref::<String>() {
            return Some(s.clone());
        }
        if let Some(s) = value.downcast_ref::<&'static str>() {
            return Some((*s).to_string());
        }
        let id = value_type_id(&*value);
        let display = with_registry(|r| r.display.get(&id).copied());
        if let Some(display) = display {
            return Some(display(value));
        }
        if let Some(object) = Self::as_object(value) {
            return Some(object.get_type().full_name());
        }
        let markup = crate::metadata::MarkupType::find_by_handle(id)?;
        if markup.kind == crate::metadata::MarkupTypeKind::Enum {
            if let Some(member) = markup.enum_members.iter().find(|member| (*(member.get)()).value_eq(value)) {
                return Some(member.name.to_string());
            }
        }
        Some(markup.full_name())
    }

    /// The dotted full name of a type, as the managed original prints a
    /// type: the name of the runtime library type a primitive mirrors
    /// (`System.Int32`, `System.String`, `System.Object`), the full name of
    /// the class of a handle, the name the registered metadata of the type
    /// states (for any of its handles), ``System.Nullable`1[..]`` for the
    /// nullable form of a value type; the Rust type name for a type nothing
    /// is known about.
    pub fn type_full_name(type_: ValueType) -> String {
        if type_.is_object() {
            return "System.Object".to_string();
        }
        if let Some(name) = primitive_full_name(type_.id()) {
            return name.to_string();
        }
        let (class, inner) = with_registry(|r| {
            (r.objects.get(&type_.id()).map(|o| o.2), r.nullable_inner.get(&type_.id()).copied())
        });
        if let Some(class) = class {
            return class.full_name();
        }
        if let Some(markup) = crate::metadata::MarkupType::find_by_handle(type_.id()) {
            return markup.full_name();
        }
        if let Some(markup) = crate::metadata::MarkupType::find_by_nullable_handle(type_.id()) {
            return format!("System.Nullable`1[{}]", markup.full_name());
        }
        match inner {
            Some(inner) if inner.is::<String>() => "System.String".to_string(),
            Some(inner) if primitive_full_name(inner.id()).is_some() => {
                format!("System.Nullable`1[{}]", Self::type_full_name(inner))
            }
            Some(inner) => Self::type_full_name(inner),
            None => type_.name().to_string(),
        }
    }

    /// The object form of a value held as the handle `Rc<T>` of a registered
    /// reference type `T` (the box that is the shared object itself); `None`
    /// for every other value.
    pub fn reference_object(value: &BoxedValue) -> Option<BoxedValue> {
        let handle = value_type_id(&**value);
        let cast = with_registry(|r| {
            r.reference_handles.get(&handle).and_then(|object| r.casts.get(&(handle, *object)).cloned())
        })?;
        cast(value)
    }

    /// Views a value as an object of the class hierarchy, if it is a
    /// registered handle type.
    pub fn as_object(value: &dyn AnyValue) -> Option<Ref<FerroObject>> {
        if let Some(o) = value.downcast_ref::<Ref<FerroObject>>() {
            return Some(o.clone());
        }
        let id = value_type_id(&*value);
        let f = with_registry(|r| r.objects.get(&id).copied());
        f.and_then(|f| (f.0)(value))
    }

    /// Registers `T` as a reference (model) type: a type whose instances are
    /// shared objects with identity, boxed as the shared object itself
    /// (`Rc<T>` coerced to a [`BoxedValue`]). Bindings hold such sources
    /// weakly, treat null as a valid value of the type, and convert between
    /// the object and `Rc<T>` / `Option<Rc<T>>` typed properties.
    pub fn register_reference<T: PartialEq + 'static>() {
        fn unwrap<T: PartialEq + 'static>(value: &dyn AnyValue) -> Option<BoxedValue> {
            match value.downcast_ref::<Option<Rc<T>>>() {
                Some(Some(v)) => Some(v.clone() as BoxedValue),
                _ => None,
            }
        }
        fn null<T: PartialEq + 'static>() -> BoxedValue {
            Rc::new(Option::<Rc<T>>::None)
        }
        fn to_rc<T: PartialEq + 'static>(value: &BoxedValue) -> Option<Rc<T>> {
            let any: Rc<dyn std::any::Any> = value.clone();
            any.downcast::<T>().ok()
        }
        let registered = with_registry(|r| r.references.insert(TypeId::of::<T>(), ()).is_some());
        if registered {
            return;
        }
        with_registry(|r| {
            r.nullable.insert(TypeId::of::<Option<Rc<T>>>(), unwrap::<T>);
            r.null_values.insert(TypeId::of::<Option<Rc<T>>>(), null::<T>);
            r.nullable_inner.insert(TypeId::of::<Option<Rc<T>>>(), ValueType::of::<T>());
            r.reference_handles.insert(TypeId::of::<Rc<T>>(), TypeId::of::<T>());
            r.reference_objects.insert(TypeId::of::<T>(), TypeId::of::<Rc<T>>());
            // The object, its handle and its nullable handle are the same
            // reference: these conversions are assignability casts.
            let mut cast = |key: (TypeId, TypeId), f: ConvertFn| {
                r.conversions.insert(key, f.clone());
                r.casts.insert(key, f);
            };
            cast(
                (TypeId::of::<T>(), TypeId::of::<Option<Rc<T>>>()),
                Rc::new(|v: &BoxedValue| to_rc::<T>(v).map(|v| Rc::new(Some(v)) as BoxedValue)),
            );
            cast(
                (TypeId::of::<T>(), TypeId::of::<Rc<T>>()),
                Rc::new(|v: &BoxedValue| to_rc::<T>(v).map(|v| Rc::new(v) as BoxedValue)),
            );
            cast(
                (TypeId::of::<Rc<T>>(), TypeId::of::<T>()),
                Rc::new(|v: &BoxedValue| v.downcast_ref::<Rc<T>>().map(|v| v.clone() as BoxedValue)),
            );
            cast(
                (TypeId::of::<Rc<T>>(), TypeId::of::<Option<Rc<T>>>()),
                Rc::new(|v: &BoxedValue| v.downcast_ref::<Rc<T>>().map(|v| Rc::new(Some(v.clone())) as BoxedValue)),
            );
        });
    }

    /// Whether the type is a registered reference (model) type.
    pub fn is_reference(id: TypeId) -> bool {
        with_registry(|r| r.references.contains_key(&id))
    }

    /// Re-creates the registered handle type `id` from a root handle.
    pub(crate) fn from_object(id: TypeId, object: Ref<FerroObject>) -> Option<BoxedValue> {
        if id == TypeId::of::<Ref<FerroObject>>() {
            return Some(Rc::new(object));
        }
        let f = with_registry(|r| r.objects.get(&id).copied());
        f.and_then(|f| (f.1)(object))
    }

    /// The type a registered nullable form holds: `T` for `Option<T>` and for
    /// `Option<Rc<T>>`, `None` for any other type.
    pub fn nullable_inner(type_: ValueType) -> Option<ValueType> {
        with_registry(|r| r.nullable_inner.get(&type_.id()).copied())
    }

    /// Whether `target` accepts null.
    pub fn accepts_null(target: ValueType) -> bool {
        target.is_object()
            || with_registry(|r| r.null_values.contains_key(&target.id()) || r.references.contains_key(&target.id()))
    }

    /// The boxed null of a nullable target type.
    pub fn null_value(target: ValueType) -> Option<BoxedValue> {
        if target.is::<Option<BoxedValue>>() {
            return Some(Rc::new(Option::<BoxedValue>::None));
        }
        let f = with_registry(|r| r.null_values.get(&target.id()).copied());
        f.map(|f| f())
    }

    /// Looks up a registered conversion.
    pub fn try_convert_registered(value: &BoxedValue, target: ValueType) -> Option<BoxedValue> {
        let key = (value_type_id(&**value), target.id());
        let f = with_registry(|r| r.conversions.get(&key).cloned());
        f.and_then(|f| f(value))
    }

    /// Converts a value to exactly `target`. Returns `None` if no conversion
    /// applies; `Some(None)` (null) is only returned for a null value when
    /// `target` is the untyped "any value" [`BoxedValue`] type or a reference
    /// (model) type.
    ///
    /// The order follows the managed implicit conversion rules: identity,
    /// null to nullable, wrapping into "any value", registered conversions
    /// (which include wrapping into `Option<T>`, numeric conversions, parsing
    /// from text and handle casts), and finally conversion to text.
    pub fn try_convert(value: Option<&BoxedValue>, target: ValueType) -> Option<Option<BoxedValue>> {
        let Some(value) = value else {
            if target.is::<BoxedValue>() || Self::is_reference(target.id()) {
                return Some(None);
            }
            return Self::null_value(target).map(Some);
        };
        let id = value_type_id(&**value);
        if id == target.id() {
            return Some(Some(value.clone()));
        }
        if target.is::<Option<BoxedValue>>() {
            return Some(Some(Rc::new(Some(value.clone()))));
        }
        if target.is::<BoxedValue>() {
            return Some(Some(Rc::new(value.clone())));
        }
        if let Some(v) = Self::try_convert_registered(value, target) {
            return Some(Some(v));
        }
        // A nullable source: convert its contents.
        if with_registry(|r| r.nullable.contains_key(&id)) {
            let inner = Self::normalize(value.clone());
            return Self::try_convert(inner.as_ref(), target);
        }
        // Handles convert through the root handle.
        if let Some(object) = Self::as_object(&**value) {
            if id != TypeId::of::<Ref<FerroObject>>() {
                let root: BoxedValue = Rc::new(object.clone());
                if target.is::<Ref<FerroObject>>() {
                    return Some(Some(root));
                }
                if let Some(v) = Self::try_convert_registered(&root, target) {
                    return Some(Some(v));
                }
            }
            // And to the interface handles their class declares.
            if let Some(v) = Self::object_to_interface(&object, target) {
                return Some(Some(v));
            }
        }
        if target.is::<String>() {
            return Self::try_to_string(&**value).map(|s| Some(Rc::new(s) as BoxedValue));
        }
        if target.is::<Option<String>>() {
            return Self::try_to_string(&**value).map(|s| Some(Rc::new(Some(s)) as BoxedValue));
        }
        // Text converts to a type that states its conversion from text in
        // its markup metadata: the type converter of the managed original.
        Self::try_parse_text(value, target).map(Some)
    }

    /// The class of the object model that `type_` is a handle of: `Ref<T>`
    /// or its nullable form `Option<Ref<T>>` of a registered class `T`.
    pub fn class_of(type_: ValueType) -> Option<&'static TypeInfo> {
        with_registry(|r| {
            let id = r.nullable_inner.get(&type_.id()).map_or(type_.id(), |inner| inner.id());
            r.objects.get(&id).map(|o| o.2)
        })
    }

    /// Converts text to exactly `target` with the conversion from text the
    /// type states in its markup metadata (`parse:`): what the type
    /// converter of the type does in the managed original, where every
    /// conversion of a value to a type (`TypeUtilities.TryConvert`, the
    /// default value converter) consults it. `target` is any handle of a
    /// type with metadata, the nullable form of a value type, or a handle
    /// of a class of the object model (the conversion of the nearest class
    /// that states one applies, as a converter attribute is inherited).
    /// `None` if `value` is not text, the type states no conversion, the
    /// text does not parse or the parsed value is not a value of `target`.
    pub fn try_parse_text(value: &BoxedValue, target: ValueType) -> Option<BoxedValue> {
        use crate::metadata::MarkupType;

        if !value.is::<String>() {
            return None;
        }
        let parse = match Self::class_of(target) {
            Some(class) => {
                let mut current = Some(class);
                let mut found = None;
                while let Some(class) = current {
                    if let Some(parse) = class.markup().and_then(|markup| markup.parse) {
                        found = Some(parse);
                        break;
                    }
                    current = class.base_type();
                }
                found?
            }
            None => {
                let mut current = MarkupType::find_by_handle(target.id())
                    .or_else(|| MarkupType::find_by_nullable_handle(target.id()));
                let mut seen: Vec<&'static MarkupType> = Vec::new();
                let mut found = None;
                while let Some(markup) = current {
                    if seen.iter().any(|known| std::ptr::eq(*known, markup)) {
                        break;
                    }
                    if let Some(parse) = markup.parse {
                        found = Some(parse);
                        break;
                    }
                    seen.push(markup);
                    current = markup.base_type();
                }
                found?
            }
        };
        // A failing conversion is no conversion.
        let parsed = parse(&[Some(value.clone())]).ok()??;
        if ValueType::of_value(&*parsed) == target {
            return Some(parsed);
        }
        Self::try_cast(&parsed, target)
    }

    /// Whether a value of type `from` can be assigned to a location of type
    /// `to` without converting it: the two types are the same, `to` is the
    /// "any value" type, `to` is the nullable form of a type `from` is
    /// assignable to, `from` is a handle of a class derived from the class of
    /// the handle `to`, or a cast between the two is registered
    /// ([`register_cast`](Self::register_cast), [`register_upcast`](Self::register_upcast),
    /// [`register_reference`](Self::register_reference)). A cast registered
    /// for one form of a reference type (the object `A`, its handle `Rc<A>`)
    /// applies to its other forms, and to the forms of its target (`Rc<B>`,
    /// `B`, `Option<Rc<B>>`). Numeric conversions, parsing and conversion to
    /// text are not assignability.
    pub fn is_assignable(from: ValueType, to: ValueType) -> bool {
        if from == to || to.is_object() {
            return true;
        }
        let (direct, from_class, to_class, from_inner, to_inner) = with_registry(|r| {
            (
                r.casts.contains_key(&(from.id(), to.id())) || r.reference_cast(from.id(), to.id()).is_some(),
                r.objects.get(&from.id()).map(|o| o.2),
                r.objects.get(&to.id()).map(|o| o.2),
                r.nullable_inner.get(&from.id()).copied(),
                r.nullable_inner.get(&to.id()).copied(),
            )
        });
        if direct {
            return true;
        }
        if let (Some(from_class), Some(to_class)) = (from_class, to_class) {
            return to_class.is_assignable_from(from_class);
        }
        if let Some(from_class) = from_class {
            // A handle is assignable to the interface handles its class
            // declares.
            if with_registry(|r| r.interface_cast(to.id(), from_class).is_some()) {
                return true;
            }
        }
        match (from_inner, to_inner) {
            // Null stays null, a value is assigned as the contained type.
            (Some(inner), Some(_)) => Self::is_assignable(inner, to),
            (None, Some(inner)) => Self::is_assignable(from, inner),
            _ => false,
        }
    }

    /// Casts a value to exactly `target`, performing only the casts that
    /// [`is_assignable`](Self::is_assignable) describes. Returns `None` if
    /// the type of the value is not assignable to `target`, or if the value
    /// is null and `target` is the non-null "any value" type.
    pub fn try_cast(value: &BoxedValue, target: ValueType) -> Option<BoxedValue> {
        let id = value_type_id(&**value);
        if id == target.id() {
            return Some(value.clone());
        }
        if target.is_object() {
            // The untyped form of the value: null or the contents of a
            // nullable, and the object itself for a model handle.
            let mut untyped = Self::normalize(value.clone());
            if let Some(v) = &untyped {
                let handle = value_type_id(&**v);
                let cast = with_registry(|r| {
                    r.reference_handles.get(&handle).and_then(|object| r.casts.get(&(handle, *object)).cloned())
                });
                if let Some(cast) = cast {
                    untyped = cast(v);
                }
            }
            return if target.is::<Option<BoxedValue>>() {
                Some(Rc::new(untyped))
            } else {
                untyped.map(|v| Rc::new(v) as BoxedValue)
            };
        }
        let (direct, from_class, to_class, from_nullable, to_inner) = with_registry(|r| {
            (
                r.casts.get(&(id, target.id())).cloned(),
                r.objects.get(&id).map(|o| o.2),
                r.objects.get(&target.id()).map(|o| o.2),
                r.nullable_inner.contains_key(&id),
                r.nullable_inner.get(&target.id()).copied(),
            )
        });
        if let Some(cast) = direct {
            return cast(value);
        }
        if let Some((before, cast, after)) = with_registry(|r| r.reference_cast(id, target.id())) {
            let source = match before {
                Some(before) => before(value)?,
                None => value.clone(),
            };
            let result = cast(&source)?;
            return match after {
                Some(after) => after(&result),
                None => Some(result),
            };
        }
        if let (Some(from_class), Some(to_class)) = (from_class, to_class) {
            if !to_class.is_assignable_from(from_class) {
                return None;
            }
            // Handles are cast through the root handle.
            return Self::from_object(target.id(), Self::as_object(&**value)?);
        }
        if from_class.is_some() {
            if let Some(v) = Self::as_object(&**value).and_then(|o| Self::object_to_interface(&o, target)) {
                return Some(v);
            }
        }
        let inner = to_inner?;
        if from_nullable {
            return match Self::normalize(value.clone()) {
                Some(contents) => Self::try_cast(&contents, target),
                None => Self::null_value(target),
            };
        }
        let contents = Self::try_cast(value, inner)?;
        let wrap = with_registry(|r| r.casts.get(&(inner.id(), target.id())).cloned())?;
        wrap(&contents)
    }

    /// Compares two untyped values: nulls are equal to each other, values are
    /// equal if they hold the same type and compare equal (handles compare by
    /// identity).
    pub fn identity_equals(a: Option<&BoxedValue>, b: Option<&BoxedValue>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b) || (**a).value_eq(&**b),
            _ => false,
        }
    }
}

macro_rules! numeric_conversions {
    ($($from:ty),* => $all:tt) => {
        $( numeric_conversions!(@from $from => $all); )*
    };
    (@from $from:ty => [$($to:ty),*]) => {
        $(
            ValueTypes::register_conversion::<$from, $to>(|v| NumericCast::<$to>::cast(*v));
            ValueTypes::register_conversion::<$from, Option<$to>>(|v| NumericCast::<$to>::cast(*v).map(Some));
        )*
    };
}

/// Checked numeric conversion with the managed `Convert.ChangeType` rules:
/// integers convert when in range, floating point values round to the nearest
/// even integer.
trait NumericCast<T> {
    fn cast(self) -> Option<T>;
}

macro_rules! int_to_int {
    ($($from:ty => [$($to:ty),*]);*) => {
        $($(
            impl NumericCast<$to> for $from {
                #[inline]
                fn cast(self) -> Option<$to> { <$to>::try_from(self).ok() }
            }
        )*)*
    };
}

macro_rules! int_to_float {
    ($($from:ty),*) => {
        $(
            impl NumericCast<f64> for $from {
                #[inline]
                fn cast(self) -> Option<f64> { Some(self as f64) }
            }
            impl NumericCast<f32> for $from {
                #[inline]
                fn cast(self) -> Option<f32> { Some(self as f32) }
            }
        )*
    };
}

macro_rules! float_to_int {
    ($($to:ty),*) => {
        $(
            impl NumericCast<$to> for f64 {
                fn cast(self) -> Option<$to> {
                    let rounded = self.round_ties_even();
                    if rounded.is_finite() && rounded >= <$to>::MIN as f64 && rounded <= <$to>::MAX as f64 {
                        Some(rounded as $to)
                    } else {
                        None
                    }
                }
            }
            impl NumericCast<$to> for f32 {
                fn cast(self) -> Option<$to> { NumericCast::<$to>::cast(f64::from(self)) }
            }
        )*
    };
}

int_to_int!(
    i8 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    i16 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    i32 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    i64 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    u8 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    u16 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    u32 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    u64 => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    isize => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize];
    usize => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize]
);
int_to_float!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);
float_to_int!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize);

impl NumericCast<f64> for f64 {
    fn cast(self) -> Option<f64> {
        Some(self)
    }
}
impl NumericCast<f32> for f64 {
    fn cast(self) -> Option<f32> {
        Some(self as f32)
    }
}
impl NumericCast<f64> for f32 {
    fn cast(self) -> Option<f64> {
        Some(f64::from(self))
    }
}
impl NumericCast<f32> for f32 {
    fn cast(self) -> Option<f32> {
        Some(self)
    }
}

macro_rules! primitive {
    ($($ty:ty),*) => {
        $(
            ValueTypes::register_display::<$ty>();
            ValueTypes::register_nullable::<$ty>();
            ValueTypes::register_parse::<$ty>(|s| s.trim().parse::<$ty>().ok());
            ValueTypes::register_conversion::<String, Option<$ty>>(|s| s.trim().parse::<$ty>().ok().map(Some));
        )*
    };
}

/// The name of the runtime library type a primitive mirrors.
fn primitive_full_name(id: TypeId) -> Option<&'static str> {
    macro_rules! names {
        ($($type_:ty => $name:literal),* $(,)?) => {
            $(if id == TypeId::of::<$type_>() { return Some($name); })*
        };
    }
    names!(
        bool => "System.Boolean", i8 => "System.SByte", u8 => "System.Byte", i16 => "System.Int16",
        u16 => "System.UInt16", i32 => "System.Int32", u32 => "System.UInt32", i64 => "System.Int64",
        u64 => "System.UInt64", isize => "System.IntPtr", usize => "System.UIntPtr", f32 => "System.Single",
        f64 => "System.Double", char => "System.Char", String => "System.String", &'static str => "System.String",
    );
    None
}

fn parse_bool(s: &str) -> Option<bool> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("true") {
        Some(true)
    } else if s.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn register_defaults() {
    primitive!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize, f32, f64, char);
    numeric_conversions!(i8, i16, i32, i64, u8, u16, u32, u64, isize, usize, f32, f64
        => [i8, i16, i32, i64, u8, u16, u32, u64, isize, usize, f32, f64]);

    // The managed runtime shows booleans as "True"/"False".
    fn display_bool(value: &dyn AnyValue) -> String {
        match value.downcast_ref::<bool>() {
            Some(true) => "True".to_string(),
            _ => "False".to_string(),
        }
    }
    with_registry(|r| {
        r.display.insert(TypeId::of::<bool>(), display_bool);
    });
    ValueTypes::register_nullable::<bool>();
    ValueTypes::register_parse::<bool>(parse_bool);
    ValueTypes::register_conversion::<String, Option<bool>>(|s| parse_bool(s).map(Some));

    ValueTypes::register_nullable::<String>();
    ValueTypes::register_conversion::<&'static str, String>(|s| Some((*s).to_string()));
    ValueTypes::register_conversion::<&'static str, Option<String>>(|s| Some(Some((*s).to_string())));

    ValueTypes::register_object::<FerroObject>();
    ValueTypes::register_object::<StyledElement>();
    ValueTypes::register_upcast::<StyledElement, FerroObject>();
    // Every class registers its handle type when it is initialised (see
    // `ObjectType::register_class`). The base classes that are used as
    // property types are also registered here, so that a null can be
    // converted to their nullable handle on a thread where the class was
    // not initialised yet. Conversions from a derived handle to a base
    // handle go through the root handle.
    ValueTypes::register_object::<crate::animation::Animatable>();
    ValueTypes::register_object::<crate::Visual>();
    ValueTypes::register_object::<crate::layout::Layoutable>();
    ValueTypes::register_object::<crate::interactivity::Interactive>();
    ValueTypes::register_object::<crate::input::InputElement>();
    ValueTypes::register_object::<crate::media::Brush>();
    ValueTypes::register_object::<crate::media::SolidColorBrush>();
    ValueTypes::register_object::<crate::media::GradientBrush>();
    ValueTypes::register_object::<crate::media::Transform>();
    ValueTypes::register_object::<crate::media::Geometry>();

    ValueTypes::register_display::<crate::metadata::MarkupDelegate>();
    ValueTypes::register_nullable::<crate::metadata::MarkupDelegate>();
    ValueTypes::register_nullable::<Rc<dyn crate::input::ICommand>>();
    ValueTypes::register_nullable::<crate::data::core::plugins::ObservableValue>();

    crate::utilities::Decimal::register_value_type();
}

impl ValueTypes {
    /// Registers a conversion that needs the shared box itself (for reference
    /// types, whose box is the object).
    /// Registers an assignability cast that needs the shared box itself: the
    /// counterpart of [`register_cast`](Self::register_cast) for reference
    /// types, whose box is the object (for example the cast of a reference
    /// object to the handle of a contract it implements). A cast is also a
    /// conversion.
    pub fn register_boxed_cast<TFrom: 'static, TTo: PropertyValue>(cast: impl Fn(&BoxedValue) -> Option<TTo> + 'static) {
        let cast: ConvertFn = Rc::new(move |v: &BoxedValue| cast(v).map(|v| Rc::new(v) as BoxedValue));
        insert_cast((TypeId::of::<TFrom>(), TypeId::of::<TTo>()), cast);
    }

    pub fn register_boxed_conversion<TFrom: 'static, TTo: PropertyValue>(
        convert: impl Fn(&BoxedValue) -> Option<TTo> + 'static,
    ) {
        insert_conversion(
            (TypeId::of::<TFrom>(), TypeId::of::<TTo>()),
            Rc::new(move |v: &BoxedValue| convert(v).map(|v| Rc::new(v) as BoxedValue)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
    use crate::{ferro_markup_enum, ferro_markup_type};

    trait IShape {
        fn sides(&self) -> i32;
    }

    impl PartialEq for dyn IShape {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::addr_eq(self as *const Self, other as *const Self)
        }
    }

    struct ShapeBase {
        sides: i32,
    }

    impl PartialEq for ShapeBase {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    impl IShape for ShapeBase {
        fn sides(&self) -> i32 {
            self.sides
        }
    }

    struct Square {
        base: Rc<ShapeBase>,
    }

    impl PartialEq for Square {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    struct Circle {
        base: Rc<ShapeBase>,
    }

    impl PartialEq for Circle {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    struct Unrelated;

    impl PartialEq for Unrelated {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    fn register_shapes() {
        ValueTypes::register_reference::<ShapeBase>();
        ValueTypes::register_reference::<Square>();
        ValueTypes::register_reference::<Circle>();
        ValueTypes::register_reference::<Unrelated>();
        ValueTypes::register_nullable::<Rc<dyn IShape>>();
        // Registered for the object form, with the box.
        ValueTypes::register_boxed_cast::<Square, Rc<ShapeBase>>(|object| {
            let any: Rc<dyn std::any::Any> = object.clone();
            any.downcast::<Square>().ok().map(|square| square.base.clone())
        });
        ValueTypes::register_cast::<Square, Rc<dyn IShape>>(|square| square.base.clone() as Rc<dyn IShape>);
        // Registered for the handle form.
        ValueTypes::register_cast::<Rc<Circle>, Rc<ShapeBase>>(|circle| circle.base.clone());
        ValueTypes::register_cast::<Rc<Circle>, Rc<dyn IShape>>(|circle| circle.base.clone() as Rc<dyn IShape>);
    }

    fn sides_of(value: &BoxedValue) -> i32 {
        if let Some(base) = value.downcast_ref::<ShapeBase>() {
            return base.sides;
        }
        if let Some(base) = value.downcast_ref::<Rc<ShapeBase>>() {
            return base.sides;
        }
        if let Some(Some(base)) = value.downcast_ref::<Option<Rc<ShapeBase>>>() {
            return base.sides;
        }
        if let Some(shape) = value.downcast_ref::<Rc<dyn IShape>>() {
            return shape.sides();
        }
        if let Some(Some(shape)) = value.downcast_ref::<Option<Rc<dyn IShape>>>() {
            return shape.sides();
        }
        panic!("unexpected type {}", (**value).type_name());
    }

    #[test]
    fn registered_casts_of_reference_types_apply_to_every_form() {
        register_shapes();
        let square = Rc::new(Square { base: Rc::new(ShapeBase { sides: 4 }) });
        let circle = Rc::new(Circle { base: Rc::new(ShapeBase { sides: 0 }) });
        let targets = [
            ValueType::of::<Rc<ShapeBase>>(),
            ValueType::of::<Option<Rc<ShapeBase>>>(),
            ValueType::of::<ShapeBase>(),
            ValueType::of::<Rc<dyn IShape>>(),
            ValueType::of::<Option<Rc<dyn IShape>>>(),
        ];
        let sources: [(BoxedValue, ValueType, i32); 6] = [
            (square.clone(), ValueType::of::<Square>(), 4),
            (Rc::new(square.clone()), ValueType::of::<Rc<Square>>(), 4),
            (Rc::new(Some(square.clone())), ValueType::of::<Option<Rc<Square>>>(), 4),
            (circle.clone(), ValueType::of::<Circle>(), 0),
            (Rc::new(circle.clone()), ValueType::of::<Rc<Circle>>(), 0),
            (Rc::new(Some(circle.clone())), ValueType::of::<Option<Rc<Circle>>>(), 0),
        ];
        for (value, type_, sides) in &sources {
            for target in targets {
                // A nullable handle is assignable to what takes null only.
                let from_nullable = type_.name().contains("Option<");
                if from_nullable && !target.name().contains("Option<") {
                    assert!(!ValueTypes::is_assignable(*type_, target), "{type_} -> {target}");
                    assert!(ValueTypes::try_cast(value, target).is_none(), "{type_} -> {target}");
                    continue;
                }
                assert!(ValueTypes::is_assignable(*type_, target), "{type_} -> {target}");
                let cast = ValueTypes::try_cast(value, target).unwrap_or_else(|| panic!("{type_} -> {target}"));
                assert_eq!(ValueType::of_value(&*cast), target, "{type_} -> {target}");
                assert_eq!(sides_of(&cast), *sides, "{type_} -> {target}");
            }
        }
        // The cast keeps the identity of the base object.
        let cast = ValueTypes::try_cast(&(square.clone() as BoxedValue), ValueType::of::<Rc<ShapeBase>>()).unwrap();
        assert!(Rc::ptr_eq(cast.downcast_ref::<Rc<ShapeBase>>().unwrap(), &square.base));
        // A null nullable handle stays null.
        let null: BoxedValue = Rc::new(Option::<Rc<Square>>::None);
        let cast = ValueTypes::try_cast(&null, ValueType::of::<Option<Rc<ShapeBase>>>()).unwrap();
        assert_eq!(cast.downcast_ref::<Option<Rc<ShapeBase>>>().map(Option::is_none), Some(true));
    }

    #[test]
    fn reference_casts_make_nothing_else_assignable() {
        register_shapes();
        let base = Rc::new(ShapeBase { sides: 3 });
        let unrelated = Rc::new(Unrelated);
        // Not the reverse direction, not unrelated types.
        for target in [ValueType::of::<Rc<Square>>(), ValueType::of::<Square>(), ValueType::of::<Option<Rc<Square>>>()] {
            assert!(!ValueTypes::is_assignable(ValueType::of::<ShapeBase>(), target), "{target}");
            assert!(!ValueTypes::is_assignable(ValueType::of::<Rc<ShapeBase>>(), target), "{target}");
            assert!(ValueTypes::try_cast(&(base.clone() as BoxedValue), target).is_none(), "{target}");
            assert!(ValueTypes::try_cast(&(Rc::new(base.clone()) as BoxedValue), target).is_none(), "{target}");
        }
        for target in [ValueType::of::<Rc<ShapeBase>>(), ValueType::of::<Rc<dyn IShape>>(), ValueType::of::<Circle>()] {
            assert!(!ValueTypes::is_assignable(ValueType::of::<Unrelated>(), target), "{target}");
            assert!(!ValueTypes::is_assignable(ValueType::of::<Rc<Unrelated>>(), target), "{target}");
            assert!(ValueTypes::try_cast(&(unrelated.clone() as BoxedValue), target).is_none(), "{target}");
        }
        assert!(!ValueTypes::is_assignable(ValueType::of::<Square>(), ValueType::of::<Rc<Circle>>()));
        // Conversions are still not assignability.
        assert!(!ValueTypes::is_assignable(ValueType::of::<i32>(), ValueType::of::<f64>()));
        assert!(!ValueTypes::is_assignable(ValueType::of::<String>(), ValueType::of::<i32>()));
        assert!(!ValueTypes::is_assignable(ValueType::of::<Square>(), ValueType::of::<String>()));
        assert!(ValueTypes::try_cast(&(Rc::new(1i32) as BoxedValue), ValueType::of::<f64>()).is_none());
        assert!(ValueTypes::try_cast(&(Rc::new("1".to_string()) as BoxedValue), ValueType::of::<i32>()).is_none());
        // The forms of one type remain assignable to each other.
        assert!(ValueTypes::is_assignable(ValueType::of::<Square>(), ValueType::of::<Rc<Square>>()));
        assert!(ValueTypes::is_assignable(ValueType::of::<Rc<Square>>(), ValueType::of::<Square>()));
    }

    #[test]
    fn reference_object_is_the_object_form_of_a_handle() {
        register_shapes();
        let base = Rc::new(ShapeBase { sides: 3 });
        let handle: BoxedValue = Rc::new(base.clone());
        let object = ValueTypes::reference_object(&handle).expect("the object form");
        assert!(std::ptr::eq(object.downcast_ref::<ShapeBase>().unwrap(), &*base));
        assert!(ValueTypes::reference_object(&object).is_none());
        assert!(ValueTypes::reference_object(&(Rc::new(1i32) as BoxedValue)).is_none());
    }

    pub struct NamedVm;

    impl PartialEq for NamedVm {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    ferro_markup_type!(class NamedVm as "Outer+NamedVm" {
        namespace: "Tests.Display",
        handles: [NamedVm, Rc<NamedVm>, Option<Rc<NamedVm>>],
    });

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct NamedSize(i32);

    ferro_markup_type!(struct NamedSize { namespace: "Tests.Display", handles: [NamedSize] });

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum NamedMode {
        First,
        Second,
    }

    ferro_markup_enum!(NamedMode { First, Second }, { namespace: "Tests.Display" });

    struct Unknown;

    impl PartialEq for Unknown {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    fn register_named() {
        MarkupType::register_all(&[
            <NamedVm as MarkupTyped>::MARKUP,
            <NamedSize as MarkupTyped>::MARKUP,
            <NamedMode as MarkupTyped>::MARKUP,
        ]);
    }

    #[test]
    fn values_without_a_display_form_print_the_full_name_of_their_type() {
        register_named();
        let vm = Rc::new(NamedVm);
        // Both forms of a type with metadata.
        let object: BoxedValue = vm.clone();
        let handle: BoxedValue = Rc::new(vm.clone());
        for value in [&object, &handle] {
            assert_eq!(ValueTypes::to_display_string(Some(value)), "Tests.Display.Outer+NamedVm");
            assert_eq!(ValueTypes::try_to_string(&**value).as_deref(), Some("Tests.Display.Outer+NamedVm"));
        }
        let size: BoxedValue = Rc::new(NamedSize(1));
        assert_eq!(ValueTypes::to_display_string(Some(&size)), "Tests.Display.NamedSize");
        // A value of an enumeration prints its member.
        let mode: BoxedValue = Rc::new(NamedMode::Second);
        assert_eq!(ValueTypes::to_display_string(Some(&mode)), "Second");

        // An object of the class hierarchy: its run-time class, whatever the handle type.
        let element = crate::StyledElement::new();
        let as_element: BoxedValue = Rc::new(element.clone());
        let as_object: BoxedValue = Rc::new(crate::FerroObject::to_ref(&element));
        let expected = crate::StyledElement::TYPE.full_name();
        assert_eq!(ValueTypes::to_display_string(Some(&as_element)), expected);
        assert_eq!(ValueTypes::to_display_string(Some(&as_object)), expected);

        // A registered display form wins; text and null are unchanged.
        assert_eq!(ValueTypes::to_display_string(Some(&(Rc::new(5i32) as BoxedValue))), "5");
        assert_eq!(ValueTypes::to_display_string(Some(&(Rc::new(true) as BoxedValue))), "True");
        assert_eq!(ValueTypes::to_display_string(Some(&(Rc::new("x".to_string()) as BoxedValue))), "x");
        assert_eq!(ValueTypes::to_display_string(None), "(null)");
        let delegate: BoxedValue = Rc::new(MarkupDelegate::new(|_| None));
        assert_eq!(ValueTypes::to_display_string(Some(&delegate)), "System.Delegate");

        // Nothing is known about the type: the Rust type name, and no text form.
        let unknown: BoxedValue = Rc::new(Unknown);
        assert_eq!(ValueTypes::to_display_string(Some(&unknown)), std::any::type_name::<Unknown>());
        assert_eq!(ValueTypes::try_to_string(&*unknown), None);
        assert!(ValueTypes::try_convert(Some(&unknown), ValueType::of::<String>()).is_none());
        // A value with a name converts to text.
        let text = ValueTypes::try_convert(Some(&object), ValueType::of::<String>()).flatten().expect("text");
        assert_eq!(text.downcast_ref::<String>().map(String::as_str), Some("Tests.Display.Outer+NamedVm"));
    }

    #[test]
    fn types_print_their_full_name() {
        register_named();
        let name = |type_: ValueType| ValueTypes::type_full_name(type_);
        assert_eq!(name(ValueType::of::<i32>()), "System.Int32");
        assert_eq!(name(ValueType::of::<String>()), "System.String");
        assert_eq!(name(ValueType::of::<Option<String>>()), "System.String");
        assert_eq!(name(ValueType::of::<bool>()), "System.Boolean");
        assert_eq!(name(ValueType::of::<f64>()), "System.Double");
        assert_eq!(name(ValueType::of::<Option<i32>>()), "System.Nullable`1[System.Int32]");
        assert_eq!(name(ValueType::object()), "System.Object");
        assert_eq!(name(ValueType::of::<BoxedValue>()), "System.Object");
        assert_eq!(name(ValueType::of::<NamedVm>()), "Tests.Display.Outer+NamedVm");
        assert_eq!(name(ValueType::of::<Rc<NamedVm>>()), "Tests.Display.Outer+NamedVm");
        assert_eq!(name(ValueType::of::<Option<Rc<NamedVm>>>()), "Tests.Display.Outer+NamedVm");
        assert_eq!(name(ValueType::of::<NamedSize>()), "Tests.Display.NamedSize");
        assert_eq!(name(ValueType::of::<Option<NamedSize>>()), "System.Nullable`1[Tests.Display.NamedSize]");
        assert_eq!(name(ValueType::of::<Option<NamedMode>>()), "System.Nullable`1[Tests.Display.NamedMode]");
        let element = crate::StyledElement::TYPE.full_name();
        assert_eq!(name(ValueType::of::<Ref<StyledElement>>()), element);
        assert_eq!(name(ValueType::of::<Option<Ref<StyledElement>>>()), element);
        assert_eq!(name(ValueType::of::<Unknown>()), std::any::type_name::<Unknown>());
    }
}
