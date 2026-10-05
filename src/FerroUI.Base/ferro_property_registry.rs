use crate::direct_property::DirectPropertyDyn;
use crate::{DirectPropertyBase, FerroObject, FerroProperty, PropertyValue, TypeInfo};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type TypeKey = *const TypeInfo;
type PropertyList = Rc<Vec<&'static FerroProperty>>;

#[derive(Default)]
struct Inner {
    properties: HashMap<u32, &'static FerroProperty>,
    registered: HashMap<TypeKey, HashMap<u32, &'static FerroProperty>>,
    attached: HashMap<TypeKey, HashMap<u32, &'static FerroProperty>>,
    direct: HashMap<TypeKey, HashMap<u32, &'static dyn DirectPropertyDyn>>,
    registered_cache: HashMap<TypeKey, PropertyList>,
    attached_cache: HashMap<TypeKey, PropertyList>,
    direct_cache: HashMap<TypeKey, PropertyList>,
    inherited_cache: HashMap<TypeKey, PropertyList>,
}

/// Tracks registered property definitions.
#[derive(Default)]
pub struct FerroPropertyRegistry {
    inner: RefCell<Inner>,
}

fn key(type_: &'static TypeInfo) -> TypeKey {
    type_ as TypeKey
}

fn collect(
    map: &HashMap<TypeKey, HashMap<u32, &'static FerroProperty>>,
    type_: &'static TypeInfo,
    walk_bases: bool,
    out: &mut Vec<&'static FerroProperty>,
) {
    let mut current = Some(type_);
    while let Some(t) = current {
        if let Some(props) = map.get(&key(t)) {
            let start = out.len();
            out.extend(props.values().copied());
            // Keep a stable, registration-ordered listing per type.
            out[start..].sort_by_key(|p| p.id());
        }
        if !walk_bases {
            break;
        }
        current = t.base_type();
    }
}

impl FerroPropertyRegistry {
    /// The property registry of the current thread.
    pub fn instance() -> &'static FerroPropertyRegistry {
        thread_local! {
            static INSTANCE: &'static FerroPropertyRegistry = Box::leak(Box::default());
        }
        INSTANCE.with(|instance| *instance)
    }

    /// Gets every property known to the registry.
    pub(crate) fn properties(&self) -> Vec<&'static FerroProperty> {
        self.inner.borrow().properties.values().copied().collect()
    }

    /// Gets all non-attached properties registered on a type (and its bases).
    pub fn get_registered(&self, type_: &'static TypeInfo) -> PropertyList {
        if let Some(result) = self.inner.borrow().registered_cache.get(&key(type_)) {
            return result.clone();
        }
        // The static initialisation of the type and its bases registers the
        // properties they declare: a type is listed in full whether or not
        // it has an instance, as when the managed runtime runs the class
        // constructors of the type and its base types.
        type_.ensure_class_init();
        let mut inner = self.inner.borrow_mut();
        let mut list = Vec::new();
        collect(&inner.registered, type_, true, &mut list);
        let result = Rc::new(list);
        inner.registered_cache.insert(key(type_), result.clone());
        result
    }

    /// Gets all attached properties registered on a type (and its bases).
    pub fn get_registered_attached(&self, type_: &'static TypeInfo) -> PropertyList {
        if let Some(result) = self.inner.borrow().attached_cache.get(&key(type_)) {
            return result.clone();
        }
        type_.ensure_class_init();
        let mut inner = self.inner.borrow_mut();
        let mut list = Vec::new();
        collect(&inner.attached, type_, true, &mut list);
        let result = Rc::new(list);
        inner.attached_cache.insert(key(type_), result.clone());
        result
    }

    /// Gets all direct properties registered on a type (and its bases).
    pub fn get_registered_direct(&self, type_: &'static TypeInfo) -> PropertyList {
        if let Some(result) = self.inner.borrow().direct_cache.get(&key(type_)) {
            return result.clone();
        }
        type_.ensure_class_init();
        let mut inner = self.inner.borrow_mut();
        let mut list: Vec<&'static FerroProperty> = Vec::new();
        let mut current = Some(type_);
        while let Some(t) = current {
            if let Some(props) = inner.direct.get(&key(t)) {
                for p in props.values() {
                    let p: &'static dyn DirectPropertyDyn = *p;
                    list.push(p.as_property());
                }
            }
            current = t.base_type();
        }
        let result = Rc::new(list);
        inner.direct_cache.insert(key(type_), result.clone());
        result
    }

    /// Gets all inherited properties registered on a type: registered and
    /// attached properties whose values are inherited.
    pub fn get_registered_inherited(&self, type_: &'static TypeInfo) -> PropertyList {
        if let Some(result) = self.inner.borrow().inherited_cache.get(&key(type_)) {
            return result.clone();
        }
        let registered = self.get_registered(type_);
        let attached = self.get_registered_attached(type_);
        let mut list: Vec<&'static FerroProperty> = Vec::new();
        for p in registered.iter().chain(attached.iter()) {
            if p.inherits() && !list.iter().any(|x| x.id() == p.id()) {
                list.push(p);
            }
        }
        let result = Rc::new(list);
        self.inner.borrow_mut().inherited_cache.insert(key(type_), result.clone());
        result
    }

    /// Gets the properties a type itself declares (registered with the type
    /// as owner: styled, direct, attached and added-owner properties), in
    /// declaration order, without those of its base types.
    pub fn get_declared(&self, type_: &'static TypeInfo) -> Vec<&'static FerroProperty> {
        type_.ensure_class_init();
        let mut list = Vec::new();
        collect(&self.inner.borrow().registered, type_, false, &mut list);
        list
    }

    /// Gets all properties registered on an object's type.
    pub fn get_registered_for(&self, o: &FerroObject) -> PropertyList {
        self.get_registered(o.get_type())
    }

    /// Finds the direct property registered on the type of `o` that shares
    /// the identity of `property`.
    ///
    /// Panics if the property is not registered on the object's type.
    pub fn get_registered_direct_for<T: PropertyValue>(
        &self,
        o: &FerroObject,
        property: &'static DirectPropertyBase<T>,
    ) -> &'static DirectPropertyBase<T> {
        self.find_registered_direct(o, property).unwrap_or_else(|| {
            panic!("Property '{}.{}' not registered on '{}'", property.owner(), property.name(), o.get_type())
        })
    }

    /// Finds the direct property registered on the type of `o` that shares
    /// the identity of `property`.
    pub fn find_registered_direct<T: PropertyValue>(
        &self,
        o: &FerroObject,
        property: &'static DirectPropertyBase<T>,
    ) -> Option<&'static DirectPropertyBase<T>> {
        let object_type = o.get_type();
        if property.owner().is_assignable_from(object_type) {
            // Fast path: the property is owned by the object's class chain. A
            // more derived owner that re-registered it still takes priority.
            if std::ptr::eq(property.owner(), object_type) {
                return Some(property);
            }
        }
        let inner = self.inner.borrow();
        let mut current = Some(object_type);
        while let Some(t) = current {
            if let Some(found) = inner.direct.get(&key(t)).and_then(|m| m.get(&property.id())) {
                let found: &'static dyn DirectPropertyDyn = *found;
                return found.as_any().downcast_ref::<DirectPropertyBase<T>>();
            }
            current = t.base_type();
        }
        None
    }

    /// Finds a registered property on a type by name.
    ///
    /// Panics if `name` contains a `.`: attached properties are not looked up
    /// through this method.
    pub fn find_registered(&self, type_: &'static TypeInfo, name: &str) -> Option<&'static FerroProperty> {
        assert!(!name.contains('.'), "Attached properties not supported.");
        self.get_registered(type_).iter().copied().find(|p| p.name() == name)
    }

    /// Finds a registered property on an object's type by name.
    pub fn find_registered_for(&self, o: &FerroObject, name: &str) -> Option<&'static FerroProperty> {
        self.find_registered(o.get_type(), name)
    }

    /// Finds a registered property by its ID.
    pub fn find_registered_by_id(&self, id: u32) -> Option<&'static FerroProperty> {
        self.inner.borrow().properties.get(&id).copied()
    }

    /// Checks whether a property is registered on a type (or its bases).
    pub fn is_registered(&self, type_: &'static TypeInfo, property: &FerroProperty) -> bool {
        let has = |list: PropertyList| list.iter().any(|p| p.id() == property.id());
        has(self.get_registered(type_)) || has(self.get_registered_attached(type_))
    }

    /// Checks whether a property is registered on an object's type.
    pub fn is_registered_for(&self, o: &FerroObject, property: &FerroProperty) -> bool {
        self.is_registered(o.get_type(), property)
    }

    /// Registers a property on a type.
    ///
    /// You won't usually want to call this method directly; use the
    /// `FerroProperty::register*` functions instead.
    pub fn register<P: Registrable>(&self, type_: &'static TypeInfo, property: &'static P) {
        let base = property.as_registered_property();
        let mut inner = self.inner.borrow_mut();
        inner.registered.entry(key(type_)).or_default().entry(base.id()).or_insert(base);
        if let Some(direct) = property.as_direct() {
            inner.direct.entry(key(type_)).or_default().entry(base.id()).or_insert(direct);
            inner.direct_cache.clear();
        }
        inner.properties.entry(base.id()).or_insert(base);
        inner.registered_cache.clear();
        inner.inherited_cache.clear();
    }

    /// Registers an attached property on a host type.
    pub fn register_attached<P: Registrable>(&self, type_: &'static TypeInfo, property: &'static P) {
        let base = property.as_registered_property();
        assert!(base.is_attached(), "Cannot register a non-attached property as attached.");
        let mut inner = self.inner.borrow_mut();
        inner.attached.entry(key(type_)).or_default().entry(base.id()).or_insert(base);
        inner.properties.entry(base.id()).or_insert(base);
        inner.attached_cache.clear();
        inner.inherited_cache.clear();
    }
}

/// Property kinds that can be registered.
pub trait Registrable: 'static {
    #[doc(hidden)]
    fn as_registered_property(&'static self) -> &'static FerroProperty;
    #[doc(hidden)]
    #[allow(private_interfaces)]
    fn as_direct(&'static self) -> Option<&'static dyn DirectPropertyDyn> {
        None
    }
}

impl<T: PropertyValue> Registrable for crate::StyledProperty<T> {
    fn as_registered_property(&'static self) -> &'static FerroProperty {
        self
    }
}

impl<T: PropertyValue> Registrable for crate::AttachedProperty<T> {
    fn as_registered_property(&'static self) -> &'static FerroProperty {
        self
    }
}

impl<TOwner: crate::ObjectType, T: PropertyValue> Registrable for crate::DirectProperty<TOwner, T> {
    fn as_registered_property(&'static self) -> &'static FerroProperty {
        self
    }

    #[allow(private_interfaces)]
    fn as_direct(&'static self) -> Option<&'static dyn DirectPropertyDyn> {
        let base: &'static DirectPropertyBase<T> = self;
        Some(base)
    }
}
