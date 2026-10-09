use crate::data::{BindingBase, BindingError, BindingExpressionBase, BindingPriority, BindingValue, BindingValueType};
use crate::property_store::{BindingSource, ValueStore};
use crate::reactive::{Disposable, IDisposable, IObservable};
use crate::threading::Dispatcher;
use crate::type_system::{__forwards_to_parent, cast_this, parent_vtable};
use crate::utilities::HandlerList;
use crate::{
    BoxedValue, DirectPropertyBase, FerroProperty, FerroPropertyChangedEventArgs, FerroPropertyRegistry,
    Nullable, ObjectType, PropertyValue, Ref, StaticType, StyledProperty, Subclassable, TypeInfo, Upcast, WeakRef,
};
use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;

type PropertyChangedHandler = dyn Fn(&FerroPropertyChangedEventArgs<'_>);

/// The virtual table of [`FerroObject`].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FerroObjectVTable {
    pub type_info: &'static TypeInfo,
    pub constructed: fn(&FerroObject),
    pub on_property_changed_core: fn(&FerroObject, &FerroPropertyChangedEventArgs<'_>),
    pub on_property_changed: fn(&FerroObject, &FerroPropertyChangedEventArgs<'_>),
    pub update_data_validation: fn(&FerroObject, &'static FerroProperty, BindingValueType, Option<&BindingError>),
}

/// An object with [`FerroProperty`] support: the root of the class hierarchy.
#[repr(C)]
pub struct FerroObject {
    this: OnceCell<Weak<dyn Any>>,
    vtable: Cell<Option<&'static FerroObjectVTable>>,
    values: ValueStore,
    inheritance_parent: RefCell<Option<WeakRef<FerroObject>>>,
    inheritance_children: RefCell<Vec<WeakRef<FerroObject>>>,
    property_changed: HandlerList<PropertyChangedHandler>,
    /// The dispatcher of the thread that owns this object; resolved the first
    /// time it is asked for, so that constructing an object costs nothing.
    dispatcher: OnceCell<Arc<Dispatcher>>,
}

static FERRO_OBJECT_TYPE: TypeInfo = TypeInfo::new("FerroObject", None)
    .with_module_path(module_path!())
    .with_class_init(<FerroObject as ObjectType>::register_class)
    .with_handles(|| [std::any::TypeId::of::<Ref<FerroObject>>(), std::any::TypeId::of::<Option<Ref<FerroObject>>>()])
    .with_interfaces({
        #[allow(unused_imports)]
        use crate::ClassDefaults as _;
        <FerroObject>::__INTERFACES
    })
    .with_markup({
        #[allow(unused_imports)]
        use crate::ClassDefaults as _;
        <FerroObject>::__MARKUP
    });

impl StaticType for FerroObject {
    const TYPE: &'static TypeInfo = &FERRO_OBJECT_TYPE;
}

impl Upcast<FerroObject> for FerroObject {
    #[inline]
    fn upcast(&self) -> &FerroObject {
        self
    }
}

// SAFETY: the root class has no base; its table is the root table.
unsafe impl ObjectType for FerroObject {
    type Parent = FerroObject;
    type VTable = FerroObjectVTable;

    fn vtable() -> &'static FerroObjectVTable {
        static VTABLE: std::sync::OnceLock<FerroObjectVTable> = std::sync::OnceLock::new();
        VTABLE.get_or_init(<FerroObject as Subclassable<FerroObject>>::build_vtable)
    }

    #[inline]
    fn register_class() {
        thread_local! {
            static REGISTERED: Cell<bool> = const { Cell::new(false) };
        }
        if !REGISTERED.replace(true) {
            crate::type_system::__register_class::<FerroObject>();
        }
    }
}

/// The overridable members of [`FerroObject`].
pub trait FerroObjectImpl: ObjectType {
    /// The names of the members this implementation overrides, where it
    /// states them: `None` for an implementation written as a plain `impl`.
    /// Set by [`ferro_impl_classes!`](crate::ferro_impl_classes) and
    /// [`ferro_overrides!`](crate::ferro_overrides), never by hand: the
    /// table of the class is built from it.
    #[doc(hidden)]
    const __OVERRIDES: Option<&'static [&'static str]> = None;

    /// Called once, right after the object has been allocated. This is where
    /// constructor logic that needs a reference to the object runs. Overrides
    /// must call `parent_constructed` first.
    fn constructed(this: &Self) {
        Self::parent_constructed(this)
    }

    /// Called for every property change on the object, including changes of
    /// non-effective (base) values. Overriding this is rarely needed; override
    /// [`on_property_changed`](Self::on_property_changed) instead.
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed_core(this, change)
    }

    /// Called when the effective value of a property changes on the object.
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change)
    }

    /// Called to update the validation state for properties for which data
    /// validation is enabled.
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        Self::parent_update_data_validation(this, property, state, error)
    }
}

/// Access to the base implementations of the [`FerroObjectImpl`] members.
pub trait FerroObjectImplExt: FerroObjectImpl {
    #[inline]
    fn parent_constructed(this: &Self) {
        (parent_vtable::<Self, FerroObjectVTable>().constructed)(this.upcast())
    }

    #[inline]
    fn parent_on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        (parent_vtable::<Self, FerroObjectVTable>().on_property_changed_core)(this.upcast(), change)
    }

    #[inline]
    fn parent_on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        (parent_vtable::<Self, FerroObjectVTable>().on_property_changed)(this.upcast(), change)
    }

    #[inline]
    fn parent_update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        (parent_vtable::<Self, FerroObjectVTable>().update_data_validation)(this.upcast(), property, state, error)
    }
}

impl<T: FerroObjectImpl> FerroObjectImplExt for T {}

impl<T: FerroObjectImpl> Subclassable<T> for FerroObject {
    fn build_vtable() -> FerroObjectVTable {
        // SAFETY (all casts): the table is only attached to objects whose
        // most-derived class is `T`.
        let mut table = FerroObjectVTable {
            type_info: T::TYPE,
            constructed: |this| T::constructed(unsafe { cast_this::<FerroObject, T>(this) }),
            on_property_changed_core: |this, change| {
                T::on_property_changed_core(unsafe { cast_this::<FerroObject, T>(this) }, change)
            },
            on_property_changed: |this, change| {
                T::on_property_changed(unsafe { cast_this::<FerroObject, T>(this) }, change)
            },
            update_data_validation: |this, property, state, error| {
                T::update_data_validation(unsafe { cast_this::<FerroObject, T>(this) }, property, state, error)
            },
        };

        // A member `T` states it does not override is the default, which
        // calls the same slot of the parent table and does nothing else: the
        // slot itself is taken, so that a call reaches the implementation
        // without a forwarding function of every class in between. A change
        // of a property goes through two of these members.
        let forwards =
            |member: &str| __forwards_to_parent::<T, FerroObjectVTable>(<T as FerroObjectImpl>::__OVERRIDES, member);
        if forwards("constructed") {
            table.constructed = parent_vtable::<T, FerroObjectVTable>().constructed;
        }
        if forwards("on_property_changed_core") {
            table.on_property_changed_core = parent_vtable::<T, FerroObjectVTable>().on_property_changed_core;
        }
        if forwards("on_property_changed") {
            table.on_property_changed = parent_vtable::<T, FerroObjectVTable>().on_property_changed;
        }
        if forwards("update_data_validation") {
            table.update_data_validation = parent_vtable::<T, FerroObjectVTable>().update_data_validation;
        }
        table
    }
}

impl FerroObjectImpl for FerroObject {
    fn constructed(_this: &Self) {}

    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.is_effective_value_change() {
            this.on_property_changed(change);
        }
    }

    fn on_property_changed(_this: &Self, _change: &FerroPropertyChangedEventArgs<'_>) {}

    fn update_data_validation(
        _this: &Self,
        _property: &'static FerroProperty,
        _state: BindingValueType,
        _error: Option<&BindingError>,
    ) {
    }
}

impl FerroObject {
    /// The runtime type of this class.
    pub const TYPE: &'static TypeInfo = &FERRO_OBJECT_TYPE;

    /// Creates the class data. Use from the `construct` function of a derived
    /// class; the object becomes usable once passed to
    /// [`instantiate`](crate::instantiate).
    pub fn construct() -> FerroObject {
        FerroObject {
            this: OnceCell::new(),
            vtable: Cell::new(None),
            values: ValueStore::new(),
            inheritance_parent: RefCell::new(None),
            inheritance_children: RefCell::new(Vec::new()),
            property_changed: HandlerList::new(),
            dispatcher: OnceCell::new(),
        }
    }

    /// Returns the [`Dispatcher`] that this object is associated with: the
    /// dispatcher of the thread that owns the object.
    #[inline]
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        // Objects cannot leave their thread, so the dispatcher of the calling
        // thread is the dispatcher of the object.
        self.dispatcher.get_or_init(Dispatcher::current_dispatcher)
    }

    /// Returns whether the calling thread is the thread that owns this
    /// object.
    #[inline]
    pub fn check_access(&self) -> bool {
        self.dispatcher().check_access()
    }

    /// Checks that the calling thread is the thread that owns this object.
    ///
    /// # Panics
    /// Panics when it is not.
    #[inline]
    #[track_caller]
    pub fn verify_access(&self) {
        self.dispatcher().verify_access()
    }

    /// Creates a plain object.
    pub fn new() -> Ref<FerroObject> {
        crate::instantiate(Self::construct())
    }

    /// Attaches the self reference and the virtual table.
    ///
    /// # Safety
    ///
    /// `vtable` must point to the `'static` virtual table of the most-derived
    /// class of the object that `self` is the root part of, and `this` must
    /// be a weak reference to that object.
    #[doc(hidden)]
    pub unsafe fn attach(&self, this: Weak<dyn Any>, vtable: *const ()) {
        let _ = self.this.set(this);
        self.vtable.set(Some(&*(vtable as *const FerroObjectVTable)));
    }

    #[inline]
    fn vt(&self) -> &'static FerroObjectVTable {
        self.vtable.get().expect("object is not instantiated")
    }

    /// Returns the virtual table of the object viewed as class `T`'s table.
    #[doc(hidden)]
    #[inline]
    pub fn vtable_of<T: ObjectType>(this: &T) -> &'static T::VTable {
        let object: &FerroObject = this.upcast();
        let table: *const FerroObjectVTable = object.vt();
        // SAFETY: `this` is the `T` part of a live object, so the object's
        // most-derived class derives from `T` and its table starts with
        // `T::VTable` (`ObjectType` invariant).
        unsafe { &*(table as *const T::VTable) }
    }

    /// Returns a strong reference to the object that `this` is part of.
    #[doc(hidden)]
    #[inline]
    pub fn ref_of<T: ObjectType>(this: &T) -> Ref<T> {
        let object: &FerroObject = this.upcast();
        let ptr = object
            .this
            .get()
            .and_then(Weak::upgrade)
            .expect("object is not instantiated or is being dropped");
        // `this` is the `T` part of the object behind `ptr`.
        Ref::from_rc(ptr)
    }

    /// A strong reference to this object.
    #[inline]
    pub fn to_ref(&self) -> Ref<FerroObject> {
        Self::ref_of(self)
    }

    /// A weak reference to this object.
    #[inline]
    pub fn to_weak(&self) -> WeakRef<FerroObject> {
        WeakRef::from_weak(self.this.get().expect("object is not instantiated").clone())
    }

    /// The runtime type of the object.
    #[inline]
    pub fn get_type(&self) -> &'static TypeInfo {
        match self.vtable.get() {
            Some(table) => table.type_info,
            None => Self::TYPE,
        }
    }

    /// Whether the object is of class `T` or a class derived from it.
    #[inline]
    pub fn is<T: ObjectType>(&self) -> bool {
        T::TYPE.is_assignable_from(self.get_type())
    }

    /// Returns the object viewed as class `T` if it is one.
    #[inline]
    pub fn downcast_ref<T: ObjectType>(&self) -> Option<&T> {
        if self.is::<T>() {
            // SAFETY: the runtime type derives from `T`, so `self` is the root
            // part of an object that has `T` at offset zero.
            Some(unsafe { cast_this::<FerroObject, T>(self) })
        } else {
            None
        }
    }

    #[inline]
    pub(crate) fn values(&self) -> &ValueStore {
        &self.values
    }

    // --- virtual dispatch -------------------------------------------------

    #[inline]
    pub(crate) fn constructed(&self) {
        crate::perf_count_virtual!("FerroObject::constructed");
        (self.vt().constructed)(self)
    }

    #[inline]
    pub(crate) fn on_property_changed_core(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        crate::perf_count_virtual!("FerroObject::on_property_changed_core");
        (self.vt().on_property_changed_core)(self, change)
    }

    /// Invokes the `on_property_changed` virtual member.
    #[inline]
    pub fn on_property_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        crate::perf_count_virtual!("FerroObject::on_property_changed");
        (self.vt().on_property_changed)(self, change)
    }

    #[inline]
    pub(crate) fn on_update_data_validation(
        &self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<BindingError>,
    ) {
        crate::perf_count_virtual!("FerroObject::update_data_validation");
        (self.vt().update_data_validation)(self, property, state, error.as_ref())
    }

    // --- inheritance ------------------------------------------------------

    /// The parent object that inherited property values are inherited from.
    pub fn inheritance_parent(&self) -> Option<Ref<FerroObject>> {
        self.inheritance_parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the parent object that inherited property values are inherited
    /// from.
    pub fn set_inheritance_parent(&self, value: impl Into<Nullable<FerroObject>>) {
        let value = value.into().0;
        let value = value.as_ref();
        let old = self.inheritance_parent();
        let same = match (&old, value) {
            (Some(a), Some(b)) => a.ptr_eq(b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        if let Some(old) = &old {
            old.remove_inheritance_child(self);
        }
        *self.inheritance_parent.borrow_mut() = value.map(Ref::downgrade);
        if let Some(new) = value {
            let mut children = new.inheritance_children.borrow_mut();
            if children.len() == children.capacity() {
                // Before growing, forget children that were dropped without
                // being detached (amortised constant time).
                children.retain(|c| c.upgrade().is_some());
            }
            children.push(self.to_weak());
        }
        self.values.set_inheritance_parent(self, value);
    }

    fn remove_inheritance_child(&self, child: &FerroObject) {
        let child = child.to_ref();
        self.inheritance_children.borrow_mut().retain(|c| !c.points_to(&child));
    }

    /// Calls `f` for each inheritance child, by index over the count at the
    /// start, as the original iterates `GetInheritanceChildren()`: no copy
    /// of the children for every object a change propagates through. A child
    /// that was dropped without being detached is skipped.
    pub(crate) fn for_each_inheritance_child(&self, mut f: impl FnMut(&Ref<FerroObject>)) {
        let count = self.inheritance_children.borrow().len();
        for index in 0..count {
            // Deviation (DEVIATIONS.md, Property system): upstream's
            // `children[i]` throws past the end; here the list holds weak
            // references and `inheritance_children` prunes dropped children,
            // which can shorten it during the loop, so the loop stops instead.
            let child = match self.inheritance_children.borrow().get(index) {
                Some(child) => child.upgrade(),
                None => break,
            };
            if let Some(child) = child {
                f(&child);
            }
        }
    }

    pub(crate) fn inheritance_children(&self) -> Vec<Ref<FerroObject>> {
        let mut children = self.inheritance_children.borrow_mut();
        if children.is_empty() {
            return Vec::new();
        }
        let result: Vec<Ref<FerroObject>> = children.iter().filter_map(WeakRef::upgrade).collect();
        if result.len() != children.len() {
            // Forget children that were dropped without being detached: a
            // weak reference keeps the child's allocation alive.
            children.retain(|c| c.upgrade().is_some());
        }
        result
    }

    // --- styled properties ------------------------------------------------

    /// Clears a property's local value.
    pub fn clear_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>) {
        self.values.clear_value(self, property);
    }

    /// Gets a property value.
    #[inline]
    pub fn get_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> T {
        self.values.get_value(self, property)
    }

    /// Gets a property's base value: the value excluding animated values.
    /// Returns `None` if the value comes from inheritance or the default.
    pub fn get_base_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>) -> Option<T> {
        self.values.get_base_value(property)
    }

    /// Checks whether a property is animating.
    pub fn is_animating(&self, property: &FerroProperty) -> bool {
        self.values.is_animating(property)
    }

    /// Checks whether a styled property has a value assigned to it or a
    /// binding targeting it.
    pub fn is_set(&self, property: &FerroProperty) -> bool {
        self.values.is_set(property)
    }

    /// Sets a property value with local value priority.
    #[inline]
    pub fn set_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>, value: T) {
        match value_marker(&value) {
            Some(ValueMarker::Unset) => self.clear_value(property),
            Some(ValueMarker::DoNothing) => {}
            None => {
                self.values.set_value(self, property, value, BindingPriority::LocalValue);
            }
        }
    }

    /// Sets a property value with the given priority. For priorities other
    /// than local value, disposing the returned handle removes the value.
    pub fn set_value_with_priority<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        value: T,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        Self::validate_priority(priority);
        match value_marker(&value) {
            Some(ValueMarker::Unset) => {
                if priority == BindingPriority::LocalValue {
                    self.clear_value(property);
                }
                None
            }
            Some(ValueMarker::DoNothing) => None,
            None => self.values.set_value(self, property, value, priority),
        }
    }

    /// Sets the value of a property without changing its value source.
    ///
    /// This is used by a component that programmatically sets the value of
    /// one of its own properties without disabling an application's declared
    /// use of the property: the effective value changes, but existing
    /// bindings and styles continue to work. The new value has the
    /// property's current priority.
    pub fn set_current_value<T: PropertyValue>(&self, property: &'static StyledProperty<T>, value: T) {
        match value_marker(&value) {
            Some(ValueMarker::Unset) => self.clear_value(property),
            Some(ValueMarker::DoNothing) => {}
            None => self.values.set_current_value(self, property, value),
        }
    }

    /// Binds a property to a binding. Returns the binding expression which
    /// represents the binding instance on this object.
    pub fn bind_binding(
        &self,
        property: &'static FerroProperty,
        binding: &dyn BindingBase,
    ) -> Rc<dyn BindingExpressionBase> {
        self.bind_binding_with_anchor(property, binding, None)
    }

    /// Binds a property to a binding.
    ///
    /// Some bindings need context to locate their source: a data context,
    /// named elements or resources. If this object is not an element, or is
    /// not yet attached to the tree, `anchor` provides that context.
    pub fn bind_binding_with_anchor(
        &self,
        property: &'static FerroProperty,
        binding: &dyn BindingBase,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        crate::perf_count!(BindingsInstanced);
        let expression = binding.create_instance(self, Some(property), anchor);
        self.values.add_binding_expression(self, property, expression)
    }

    /// Gets a binding to a property of this object: the indexer taking an
    /// [`IndexerDescriptor`](crate::data::IndexerDescriptor)
    /// (`source[!Property]` is `source.indexer(&Property.bind())`).
    pub fn indexer(&self, binding: &crate::data::IndexerDescriptor) -> crate::data::IndexerBinding {
        let property = binding.property.expect("The IndexerDescriptor has no property.");
        crate::data::IndexerBinding::new(self.to_ref(), property, binding.mode)
    }

    /// Binds the property of an
    /// [`IndexerDescriptor`](crate::data::IndexerDescriptor) to a binding:
    /// the setter of the indexer (`target[!Property] = binding` is
    /// `target.bind_indexer(&Property.bind(), &binding)`).
    pub fn bind_indexer(
        &self,
        binding: &crate::data::IndexerDescriptor,
        value: &dyn BindingBase,
    ) -> Rc<dyn BindingExpressionBase> {
        let property = binding.property.expect("The IndexerDescriptor has no property.");
        self.bind_binding(property, value)
    }

    /// The binding expression that is currently active on the property, if
    /// any.
    pub(crate) fn get_expression(&self, property: &'static FerroProperty) -> Option<Rc<dyn BindingExpressionBase>> {
        self.values.get_expression(property)
    }

    /// Binds a property to an observable.
    pub fn bind<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        source: Rc<dyn IObservable<T>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        Self::validate_priority(priority);
        self.values.add_binding(self, property, BindingSource::Typed(source), priority)
    }

    /// Binds a property to an observable of binding values.
    pub fn bind_value<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        source: Rc<dyn IObservable<BindingValue<T>>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        Self::validate_priority(priority);
        self.values.add_binding(self, property, BindingSource::Value(source), priority)
    }

    /// Binds a property to an observable of untyped values.
    pub fn bind_untyped<T: PropertyValue>(
        &self,
        property: &'static StyledProperty<T>,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        Self::validate_priority(priority);
        self.values.add_binding(self, property, BindingSource::Untyped(source), priority)
    }

    /// Coerces the specified property.
    pub fn coerce_value(&self, property: &'static FerroProperty) {
        self.values.coerce_value(self, property);
    }

    // --- direct properties ------------------------------------------------

    /// Gets a direct property value.
    pub fn get_direct_value<T: PropertyValue>(&self, property: &'static DirectPropertyBase<T>) -> T {
        let registered = FerroPropertyRegistry::instance().get_registered_direct_for(self, property);
        registered.invoke_getter(self)
    }

    /// Sets a direct property value.
    pub fn set_direct_value<T: PropertyValue>(&self, property: &'static DirectPropertyBase<T>, value: T) {
        let registered = FerroPropertyRegistry::instance().get_registered_direct_for(self, property);
        registered.invoke_setter(self, BindingValue::new(value));
    }

    /// Resets a direct property to its unset value.
    pub fn clear_direct_value<T: PropertyValue>(&self, property: &'static DirectPropertyBase<T>) {
        let registered = FerroPropertyRegistry::instance().get_registered_direct_for(self, property);
        let unset = registered.get_unset_value_for(self);
        registered.invoke_setter(self, BindingValue::new(unset));
    }

    pub(crate) fn set_direct_value_unchecked<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        value: BindingValue<T>,
    ) {
        let state = value.value_type();
        let error = value.error().cloned();
        match state {
            BindingValueType::UNSET_VALUE | BindingValueType::BINDING_ERROR => {
                let fallback =
                    if value.has_value() { value } else { value.with_value(property.get_unset_value_for(self)) };
                property.invoke_setter(self, fallback);
            }
            BindingValueType::VALUE
            | BindingValueType::BINDING_ERROR_WITH_FALLBACK
            | BindingValueType::DATA_VALIDATION_ERROR
            | BindingValueType::DATA_VALIDATION_ERROR_WITH_FALLBACK => property.invoke_setter(self, value),
            // Do nothing, and the flag combinations that no binding value has.
            _ => {}
        }
        if property.get_metadata_for(self).enable_data_validation() == Some(true) {
            self.on_update_data_validation(property, state, error);
        }
    }

    /// Binds a direct property to an observable.
    pub fn bind_direct<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IDisposable> {
        let property = self.writable_direct(property);
        self.values.add_direct_binding(self, property, BindingSource::Typed(source))
    }

    /// Binds a direct property to an observable of binding values.
    pub fn bind_direct_value<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        source: Rc<dyn IObservable<BindingValue<T>>>,
    ) -> Rc<dyn IDisposable> {
        let property = self.writable_direct(property);
        self.values.add_direct_binding(self, property, BindingSource::Value(source))
    }

    /// Binds a direct property to an observable of untyped values.
    pub fn bind_direct_untyped<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn IDisposable> {
        let property = self.writable_direct(property);
        self.values.add_direct_binding(self, property, BindingSource::Untyped(source))
    }

    fn writable_direct<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
    ) -> &'static DirectPropertyBase<T> {
        let property = FerroPropertyRegistry::instance().get_registered_direct_for(self, property);
        assert!(!property.is_read_only(), "The property {} is readonly.", property.name());
        property
    }

    /// Sets the backing field of a direct property, raising a change
    /// notification if the value changed. Returns true if it changed.
    pub fn set_and_raise<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        field: &RefCell<T>,
        value: T,
    ) -> bool {
        if *field.borrow() == value {
            return false;
        }
        let old = field.replace(value.clone());
        self.raise_property_changed(property, Some(&old), &value, BindingPriority::LocalValue, true);
        true
    }

    /// [`set_and_raise`](Self::set_and_raise) for `Copy` values stored in a
    /// [`Cell`].
    pub fn set_and_raise_cell<T: PropertyValue + Copy>(
        &self,
        property: &'static DirectPropertyBase<T>,
        field: &Cell<T>,
        value: T,
    ) -> bool {
        let old = field.get();
        if old == value {
            return false;
        }
        field.set(value);
        self.raise_property_changed(property, Some(&old), &value, BindingPriority::LocalValue, true);
        true
    }

    // --- untyped access ---------------------------------------------------

    /// Gets a property value as an untyped value.
    pub fn get_value_untyped(&self, property: &'static FerroProperty) -> BoxedValue {
        property.routes().route_get_value(self)
    }

    /// Sets a property from an untyped value, which must hold exactly the
    /// property's value type or the unset marker.
    pub fn set_value_untyped(
        &self,
        property: &'static FerroProperty,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        property.routes().route_set_value(self, value, priority)
    }

    /// [`set_current_value`](Self::set_current_value) with an untyped value.
    pub fn set_current_value_untyped(&self, property: &'static FerroProperty, value: &dyn Any) {
        property.routes().route_set_current_value(self, value)
    }

    /// Clears a property's local value.
    pub fn clear_value_untyped(&self, property: &'static FerroProperty) {
        property.routes().route_clear_value(self)
    }

    /// Binds a property to an observable of untyped values.
    pub fn bind_property_untyped(
        &self,
        property: &'static FerroProperty,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        property.routes().route_bind(self, source, priority)
    }

    // --- change notification ----------------------------------------------

    /// Subscribes to property changes on this object. Disposing the returned
    /// handle unsubscribes.
    pub fn property_changed(
        &self,
        handler: impl Fn(&FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.property_changed.add(Rc::new(handler));
        let weak = self.to_weak();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.property_changed.remove(token);
            }
        })
    }

    /// The number of subscribers to property changes on this object (a
    /// diagnostics member, used by tests to verify that subscriptions are
    /// released).
    pub fn property_changed_subscriber_count(&self) -> usize {
        self.property_changed.len()
    }

    /// Raises a property changed notification.
    pub fn raise_property_changed<T: PropertyValue>(
        &self,
        property: &'static FerroProperty,
        old_value: Option<&T>,
        new_value: &T,
        priority: BindingPriority,
        is_effective_value: bool,
    ) {
        crate::perf_count!(PropertyChangesRaised);
        let e = FerroPropertyChangedEventArgs::new(
            self,
            property,
            old_value.map(|v| v as &dyn Any),
            new_value,
            priority,
            is_effective_value,
        );

        self.on_property_changed_core(&e);

        if is_effective_value {
            crate::perf_count!(PropertyChangesEffective);
            property.notify_changed(&e);
            if !self.property_changed.is_empty() {
                crate::perf_count!(PropertyChangesWithObjectListeners);
                for (_, handler) in self.property_changed.snapshot().iter() {
                    handler(&e);
                }
            }
        }
    }

    /// Raises a property changed notification for a direct property.
    pub fn raise_direct_property_changed<T: PropertyValue>(
        &self,
        property: &'static DirectPropertyBase<T>,
        old_value: &T,
        new_value: &T,
    ) {
        self.raise_property_changed(property, Some(old_value), new_value, BindingPriority::LocalValue, true);
    }

    fn validate_priority(priority: BindingPriority) {
        if priority < BindingPriority::Animation || priority >= BindingPriority::Inherited {
            panic!("Invalid priority {priority:?}");
        }
    }
}

impl fmt::Debug for FerroObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.get_type().name())
    }
}

/// A marker held by a value of an untyped ("any value") property type.
enum ValueMarker {
    Unset,
    DoNothing,
}

/// Detects the unset and do-nothing markers in a value of an untyped
/// property type (`BoxedValue` or `Option<BoxedValue>`). For every other
/// value type this folds to a constant `None`.
#[inline]
fn value_marker<T: 'static>(value: &T) -> Option<ValueMarker> {
    let any: &dyn Any = value;
    let boxed = match any.downcast_ref::<BoxedValue>() {
        Some(boxed) => boxed,
        None => any.downcast_ref::<Option<BoxedValue>>()?.as_ref()?,
    };
    if boxed.is::<crate::UnsetValueType>() {
        Some(ValueMarker::Unset)
    } else if boxed.is::<crate::DoNothingType>() {
        Some(ValueMarker::DoNothing)
    } else {
        None
    }
}
