use super::{
    IBindableArray, IBindableIndexer, IBindableList, ICommand, INotifyCollectionChanged, INotifyDataErrorInfo,
    INotifyPropertyChanged,
};
use crate::data::core::value_type_id;
use crate::data::core::{ClrPropertyInfo, IPropertyInfo, PropertyKind, ValueType, ValueTypes};
use crate::data::BindingError;
use crate::{AnyValue, BoxedValue};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A method of a model type that bindings can turn into a command.
pub struct ModelMethod {
    pub(crate) name: Box<str>,
    pub(crate) execute: Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>)>,
    pub(crate) can_execute: Option<Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>) -> bool>>,
    pub(crate) depends_on: Vec<Box<str>>,
}

impl ModelMethod {
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The binding metadata of a model type: a type outside the class hierarchy
/// (typically a view model) that bindings with a string path can navigate.
///
/// This is what stands in for reflection: the properties and methods a type
/// declares by name, and how to reach the notification interfaces it
/// implements from an untyped value.
pub struct ModelType {
    value_type: ValueType,
    properties: Vec<Rc<dyn IPropertyInfo>>,
    methods: Vec<Rc<ModelMethod>>,
    as_inpc: Option<fn(&dyn AnyValue) -> Option<&dyn INotifyPropertyChanged>>,
    as_indei: Option<fn(&dyn AnyValue) -> Option<&dyn INotifyDataErrorInfo>>,
    as_list: Option<fn(&dyn AnyValue) -> Option<&dyn IBindableList>>,
    as_indexer: Option<fn(&dyn AnyValue) -> Option<&dyn IBindableIndexer>>,
    as_array: Option<fn(&dyn AnyValue) -> Option<&dyn IBindableArray>>,
    as_incc: Option<fn(&dyn AnyValue) -> Option<&dyn INotifyCollectionChanged>>,
}

impl ModelType {
    pub fn value_type(&self) -> ValueType {
        self.value_type
    }

    /// The declared properties.
    pub fn properties(&self) -> &[Rc<dyn IPropertyInfo>] {
        &self.properties
    }

    /// Finds a declared property by name.
    pub fn find_property(&self, name: &str) -> Option<Rc<dyn IPropertyInfo>> {
        self.properties.iter().find(|p| p.name() == name).cloned()
    }

    /// Finds a declared method by name.
    pub fn find_method(&self, name: &str) -> Option<Rc<ModelMethod>> {
        self.methods.iter().find(|m| &*m.name == name).cloned()
    }

    /// The property change notifications of `value`, if its type declared
    /// them.
    pub fn as_notify_property_changed<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn INotifyPropertyChanged> {
        self.as_inpc.and_then(|f| f(value))
    }

    /// The validation error notifications of `value`, if its type declared
    /// them.
    pub fn as_notify_data_error_info<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn INotifyDataErrorInfo> {
        self.as_indei.and_then(|f| f(value))
    }

    /// Indexed access to `value`, if its type declared itself a list.
    pub fn as_list<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn IBindableList> {
        self.as_list.and_then(|f| f(value))
    }

    /// The indexer of `value`, if its type declared one.
    pub fn as_indexer<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn IBindableIndexer> {
        self.as_indexer.and_then(|f| f(value))
    }

    /// Array access to `value`, if its type declared itself an array.
    pub fn as_array<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn IBindableArray> {
        self.as_array.and_then(|f| f(value))
    }

    /// The collection change notifications of `value`, if its type declared
    /// them or is a notifying list.
    pub fn as_notify_collection_changed<'a>(&self, value: &'a dyn AnyValue) -> Option<&'a dyn INotifyCollectionChanged> {
        match self.as_incc {
            Some(f) => f(value),
            None => self.as_list(value).and_then(|l| l.as_notify_collection_changed()),
        }
    }
}

/// Declares the binding metadata of model type `T`.
pub struct ModelTypeBuilder<T: PartialEq + 'static> {
    model: ModelType,
    _marker: std::marker::PhantomData<fn(&T)>,
}

impl<T: PartialEq + 'static> ModelTypeBuilder<T> {
    fn new() -> Self {
        Self {
            model: ModelType {
                value_type: ValueType::of::<T>(),
                properties: Vec::new(),
                methods: Vec::new(),
                as_inpc: None,
                as_indei: None,
                as_list: None,
                as_indexer: None,
                as_array: None,
                as_incc: None,
            },
            _marker: std::marker::PhantomData,
        }
    }

    /// Declares that the type raises property change notifications.
    pub fn notify_property_changed(mut self) -> Self
    where
        T: INotifyPropertyChanged,
    {
        fn cast<T: INotifyPropertyChanged + 'static>(v: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
            v.downcast_ref::<T>().map(|v| v as &dyn INotifyPropertyChanged)
        }
        self.model.as_inpc = Some(cast::<T>);
        self
    }

    /// Declares that the type reports validation errors.
    pub fn notify_data_error_info(mut self) -> Self
    where
        T: INotifyDataErrorInfo,
    {
        fn cast<T: INotifyDataErrorInfo + 'static>(v: &dyn AnyValue) -> Option<&dyn INotifyDataErrorInfo> {
            v.downcast_ref::<T>().map(|v| v as &dyn INotifyDataErrorInfo)
        }
        self.model.as_indei = Some(cast::<T>);
        self
    }

    /// Declares that the type is a list that binding indexers can index.
    pub fn list(mut self) -> Self
    where
        T: IBindableList,
    {
        fn cast<T: IBindableList + 'static>(v: &dyn AnyValue) -> Option<&dyn IBindableList> {
            v.downcast_ref::<T>().map(|v| v as &dyn IBindableList)
        }
        self.model.as_list = Some(cast::<T>);
        self
    }

    /// Declares that the type has an indexer that binding indexers can use
    /// (`Items[key]`).
    pub fn indexer(mut self) -> Self
    where
        T: IBindableIndexer,
    {
        fn cast<T: IBindableIndexer + 'static>(v: &dyn AnyValue) -> Option<&dyn IBindableIndexer> {
            v.downcast_ref::<T>().map(|v| v as &dyn IBindableIndexer)
        }
        self.model.as_indexer = Some(cast::<T>);
        self
    }

    /// Declares that the type is an array: a fixed-size collection accessed
    /// with integer indexes, one per dimension.
    pub fn array(mut self) -> Self
    where
        T: IBindableArray,
    {
        fn cast<T: IBindableArray + 'static>(v: &dyn AnyValue) -> Option<&dyn IBindableArray> {
            v.downcast_ref::<T>().map(|v| v as &dyn IBindableArray)
        }
        self.model.as_array = Some(cast::<T>);
        self
    }

    /// Declares that the type raises collection change notifications.
    pub fn notify_collection_changed(mut self) -> Self
    where
        T: INotifyCollectionChanged,
    {
        fn cast<T: INotifyCollectionChanged + 'static>(v: &dyn AnyValue) -> Option<&dyn INotifyCollectionChanged> {
            v.downcast_ref::<T>().map(|v| v as &dyn INotifyCollectionChanged)
        }
        self.model.as_incc = Some(cast::<T>);
        self
    }

    /// Declares that the type is a command, so that instances can be bound to
    /// command properties.
    pub fn command(self) -> Self
    where
        T: ICommand,
    {
        ValueTypes::register_boxed_conversion::<T, Rc<dyn ICommand>>(|v| {
            let any: Rc<dyn std::any::Any> = v.clone();
            any.downcast::<T>().ok().map(|c| c as Rc<dyn ICommand>)
        });
        self
    }

    /// Declares a property from a ready-made description.
    pub fn property_info(mut self, property: Rc<dyn IPropertyInfo>) -> Self {
        self.model.properties.push(property);
        self
    }

    /// Declares a read-only property. `K` states how the typed value maps to
    /// an untyped binding value, see [`PropertyKind`].
    pub fn read_only<K: PropertyKind>(self, name: &str, get: impl Fn(&T) -> K::Typed + 'static) -> Self {
        self.property_info(Rc::new(ClrPropertyInfo::read_only::<T, K>(name, get)))
    }

    /// Declares a read-write property.
    pub fn property<K: PropertyKind>(
        self,
        name: &str,
        get: impl Fn(&T) -> K::Typed + 'static,
        set: impl Fn(&T, K::Typed) + 'static,
    ) -> Self {
        self.property_info(Rc::new(ClrPropertyInfo::read_write::<T, K>(name, get, set)))
    }

    /// Declares a read-write property whose setter can reject a value. The
    /// error surfaces as a data validation error on bindings that have data
    /// validation enabled (the equivalent of throwing from a setter).
    pub fn validated_property<K: PropertyKind>(
        self,
        name: &str,
        get: impl Fn(&T) -> K::Typed + 'static,
        set: impl Fn(&T, K::Typed) -> Result<(), BindingError> + 'static,
    ) -> Self {
        self.property_info(Rc::new(ClrPropertyInfo::read_write_validated::<T, K>(name, get, set)))
    }

    /// Declares a method that bindings can use as a command.
    pub fn method(mut self, name: &str, execute: impl Fn(&T, Option<&BoxedValue>) + 'static) -> Self {
        self.model.methods.push(Rc::new(ModelMethod {
            name: name.into(),
            execute: Rc::new(move |o, p| {
                if let Some(o) = o.downcast_ref::<T>() {
                    execute(o, p)
                }
            }),
            can_execute: None,
            depends_on: Vec::new(),
        }));
        self
    }

    /// Declares a method with a "can execute" predicate that is re-queried
    /// when one of the properties named in `depends_on` changes.
    pub fn method_with_can_execute(
        mut self,
        name: &str,
        execute: impl Fn(&T, Option<&BoxedValue>) + 'static,
        can_execute: impl Fn(&T, Option<&BoxedValue>) -> bool + 'static,
        depends_on: &[&str],
    ) -> Self {
        self.model.methods.push(Rc::new(ModelMethod {
            name: name.into(),
            execute: Rc::new(move |o, p| {
                if let Some(o) = o.downcast_ref::<T>() {
                    execute(o, p)
                }
            }),
            can_execute: Some(Rc::new(move |o, p| o.downcast_ref::<T>().is_some_and(|o| can_execute(o, p)))),
            depends_on: depends_on.iter().map(|s| (*s).into()).collect(),
        }));
        self
    }
}

thread_local! {
    static MODELS: RefCell<HashMap<TypeId, Rc<ModelType>>> = RefCell::new(HashMap::new());
}

/// A model type that declares its binding metadata.
///
/// Implement it with [`ferro_model!`](crate::ferro_model); create instances
/// with [`Model::new_model`] (or call [`Model::register`] once per thread
/// before the first binding touches an instance).
pub trait Model: PartialEq + Sized + 'static {
    /// Describes the type's properties, methods and notification interfaces.
    fn describe(builder: ModelTypeBuilder<Self>) -> ModelTypeBuilder<Self>;

    /// Registers the metadata on the current thread, once.
    fn register() {
        if !ModelTypes::is_registered(TypeId::of::<Self>()) {
            ModelTypes::register::<Self>(Self::describe);
        }
    }

    /// Wraps an instance into its shared, bindable form.
    fn new_model(value: Self) -> Rc<Self> {
        Self::register();
        Rc::new(value)
    }
}

/// The per-thread table of model type metadata.
pub struct ModelTypes;

impl ModelTypes {
    /// Registers (or replaces) the metadata of `T`.
    pub fn register<T: PartialEq + 'static>(describe: impl FnOnce(ModelTypeBuilder<T>) -> ModelTypeBuilder<T>) {
        ValueTypes::register_reference::<T>();
        // Reserve the slot first so that `describe` may refer to `T` itself.
        let model = Rc::new(describe(ModelTypeBuilder::new()).model);
        MODELS.with(|m| m.borrow_mut().insert(TypeId::of::<T>(), model));
    }

    /// Registers a list type whose only metadata is indexed access.
    pub(crate) fn register_list<T: IBindableList + PartialEq + 'static>() {
        if !Self::is_registered(TypeId::of::<T>()) {
            Self::register::<T>(|b| b.list());
        }
    }

    /// Registers a collection type whose only metadata is its change notifications: a
    /// collection held by value as a handle (an instantiation of the notifying list that
    /// a typed list declaration names). An indexer of a binding path over such a
    /// collection follows its changes. Unlike [`register`](Self::register) it does not
    /// make the type a reference type: its values are handles.
    pub fn register_notifying_collection<T: INotifyCollectionChanged + PartialEq + 'static>() {
        if !Self::is_registered(TypeId::of::<T>()) {
            let model = Rc::new(ModelTypeBuilder::<T>::new().notify_collection_changed().model);
            MODELS.with(|m| m.borrow_mut().insert(TypeId::of::<T>(), model));
        }
    }

    pub fn is_registered(id: TypeId) -> bool {
        MODELS.with(|m| m.borrow().contains_key(&id))
    }

    /// The metadata of the type of `value`, if it was registered.
    pub fn find(value: &dyn AnyValue) -> Option<Rc<ModelType>> {
        let id = value_type_id(&*value);
        // An accessor dropped while the thread shuts down may ask after the
        // registry is gone: nothing is registered any more then.
        MODELS.try_with(|m| m.borrow().get(&id).cloned()).ok().flatten()
    }
}

/// Declares a model type: gives it identity equality (so shared instances can
/// travel through bindings and be stored in properties) and its binding
/// metadata.
///
/// ```ignore
/// ferro_model!(PersonVm, |b| b
///     .notify_property_changed()
///     .property::<Value<String>>("Name", |vm| vm.name(), |vm, v| vm.set_name(v))
///     .read_only::<Value<i32>>("Age", |vm| vm.age())
///     .method("Save", |vm, _| vm.save()));
/// ```
#[macro_export]
macro_rules! ferro_model {
    ($ty:ty, $describe:expr) => {
        impl ::std::cmp::PartialEq for $ty {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        impl $crate::data::model::Model for $ty {
            fn describe(
                builder: $crate::data::model::ModelTypeBuilder<Self>,
            ) -> $crate::data::model::ModelTypeBuilder<Self> {
                let describe: fn(
                    $crate::data::model::ModelTypeBuilder<Self>,
                ) -> $crate::data::model::ModelTypeBuilder<Self> = $describe;
                describe(builder)
            }
        }
    };
}
