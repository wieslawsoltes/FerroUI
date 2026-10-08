//! The run-time values of runtime library types that have no Rust
//! counterpart in the framework: `System.Type`, arrays, the deferred content
//! delegate and the type descriptor context.
//!
//! # Arrays
//!
//! An array (`T[]`: an array constant of the compiler, a list parsed at
//! compile time) evaluates to a [`RuntimeArray`]: the element type and the
//! untyped elements. It is converted where it is passed on: a member that
//! declares `Vec<T>` (or `Option<Vec<T>>`) receives the elements in that
//! Rust type if `T` has a registered array form
//! ([`RuntimeArray::register_element::<T>`]; the element types of the
//! runtime library and the base value types the compiler parses are
//! registered by the type system), an untyped target (`object`) receives the
//! `RuntimeArray` itself. A member that declares another collection type
//! for an array is a conversion error that names both types.
//!
//! # Lists
//!
//! A list of the runtime library that markup creates (`List<T>` with
//! `x:TypeArguments`, `ArrayList`) evaluates to a [`RuntimeList`]: the element
//! type and the untyped items. A member that declares a collection handle (the
//! items source of an items control) receives the shared list of the items
//! through the cast the crate of the handle registers for it; an untyped
//! target receives the `RuntimeList` itself.

use std::any::TypeId;
use std::fmt;
use std::rc::Rc;

use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{IServiceProvider, MarkupType, MarkupValue};
use ferroui_base::{BoxedValue, TypeInfo};
use xamlx::exceptions::XamlResult;
use xamlx::type_system::IXamlType;

use super::runtime_type::RuntimeType;

/// A value of `System.Type`: what `x:Type` and text converted to a type
/// evaluate to.
///
/// A member that takes a type in another representation (a class reference
/// `&'static TypeInfo`, markup metadata, a value type) receives it converted;
/// see [`RuntimeTypeValue::to_handle`].
#[derive(Clone)]
pub struct RuntimeTypeValue(Rc<dyn IXamlType>);

impl RuntimeTypeValue {
    pub fn new(type_: Rc<dyn IXamlType>) -> Self {
        Self(type_)
    }

    /// The type, as the type system describes it.
    pub fn type_(&self) -> &Rc<dyn IXamlType> {
        &self.0
    }

    fn runtime(&self) -> Option<&RuntimeType> {
        self.0.as_any().downcast_ref::<RuntimeType>()
    }

    /// The class of the object model the type is, if it is one.
    pub fn type_info(&self) -> Option<&'static TypeInfo> {
        self.runtime().and_then(RuntimeType::type_info)
    }

    /// The markup metadata of the type, if it has any.
    pub fn markup(&self) -> Option<&'static MarkupType> {
        self.runtime().and_then(RuntimeType::markup)
    }

    /// The untyped (canonical) Rust type of the values of the type.
    pub fn handle(&self) -> Option<ValueType> {
        self.runtime().and_then(RuntimeType::handle)
    }

    /// The type in the representation the Rust type `handle` holds types in:
    /// `&'static TypeInfo` (classes of the object model), [`ValueType`] or
    /// [`TypeId`] (the canonical handle of the
    /// values of the type), each also as `Option<_>`. `None` if the type has
    /// no such representation.
    pub fn to_handle(&self, handle: ValueType) -> Option<BoxedValue> {
        fn boxed<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
            Some(Rc::new(value))
        }
        if handle.is::<&'static TypeInfo>() {
            boxed(self.type_info()?)
        } else if handle.is::<Option<&'static TypeInfo>>() {
            boxed(Some(self.type_info()?))
        } else if handle.is::<ValueType>() {
            boxed(self.handle()?)
        } else if handle.is::<Option<ValueType>>() {
            boxed(Some(self.handle()?))
        } else if handle.is::<TypeId>() {
            boxed(self.handle()?.id())
        } else if handle.is::<Option<TypeId>>() {
            boxed(Some(self.handle()?.id()))
        } else if handle.is::<Option<RuntimeTypeValue>>() {
            boxed(Some(self.clone()))
        } else {
            None
        }
    }
}

impl RuntimeTypeValue {
    /// The type as the value an UNTYPED target receives (a resource key, a
    /// setter value, a property of type `object`): the class reference
    /// `&'static TypeInfo` for a class of the object model, the
    /// [`ValueType`] of its canonical handle otherwise, so that type keys
    /// compare equal however they were produced. A type without a run-time
    /// representation stays a `System.Type` value.
    pub fn to_untyped_value(&self) -> BoxedValue {
        if let Some(type_info) = self.type_info() {
            return Rc::new(type_info);
        }
        match self.handle() {
            Some(handle) => Rc::new(handle),
            None => Rc::new(self.clone()),
        }
    }

    /// The type in the representation a member declares with the Rust type
    /// `target`: [`to_handle`](Self::to_handle), or the untyped form when
    /// the target takes any value. The error names the type; the caller
    /// adds the member.
    pub fn to_declared(&self, target: ValueType) -> Result<BoxedValue, String> {
        if target.is::<RuntimeTypeValue>() {
            return Ok(Rc::new(self.clone()));
        }
        if target.is_object() {
            return Ok(self.to_untyped_value());
        }
        self.to_handle(target).ok_or_else(|| {
            if target.is::<&'static TypeInfo>() || target.is::<Option<&'static TypeInfo>>() {
                format!("{} is not a class of the object model: only a class can be named here", self.0.full_name())
            } else {
                format!("{} cannot be passed as a {}", self.0.full_name(), target.name())
            }
        })
    }
}

impl PartialEq for RuntimeTypeValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(&*other.0)
    }
}

impl fmt::Debug for RuntimeTypeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "typeof({})", self.0.full_name())
    }
}

/// The run-time form of the type name: what the managed `Type.ToString()`
/// shows.
impl fmt::Display for RuntimeTypeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.full_name())
    }
}

/// The value of deferred content: the equivalent of the managed
/// `Func<IServiceProvider, object>` the compiled markup stores in a property
/// whose content is built on demand. Calling it builds the content anew.
#[derive(Clone)]
pub struct DeferredContentFactory(Rc<dyn Fn(Option<Rc<dyn IServiceProvider>>) -> XamlResult<MarkupValue>>);

impl DeferredContentFactory {
    pub fn new(build: impl Fn(Option<Rc<dyn IServiceProvider>>) -> XamlResult<MarkupValue> + 'static) -> Self {
        Self(Rc::new(build))
    }

    /// Builds the content with `service_provider` as the parent service
    /// provider of the build.
    pub fn invoke(&self, service_provider: Option<Rc<dyn IServiceProvider>>) -> XamlResult<MarkupValue> {
        (self.0)(service_provider)
    }
}

impl PartialEq for DeferredContentFactory {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for DeferredContentFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeferredContentFactory")
    }
}

/// The context type converters receive: the equivalent of the runtime
/// library's `ITypeDescriptorContext`, of which markup only uses the service
/// provider.
pub trait ITypeDescriptorContext: IServiceProvider {}

/// Handles compare by identity, so that they can be held in untyped values.
impl<'a> PartialEq for dyn ITypeDescriptorContext + 'a {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// The untyped forms of values, shared with generated code: the conventions
/// of the runtime library ([`compiled`](ferroui_markup_xaml::xaml_il::runtime::compiled)).
pub use ferroui_markup_xaml::xaml_il::runtime::compiled::{box_object, normalize_object, to_untyped};

/// The arrays bindings index into ([`BindableArray`](ferroui_base::data::model::BindableArray)),
/// by the Rust types that hold them, with the Rust type of their elements.
static BINDABLE_ARRAYS: std::sync::Mutex<Vec<(ValueType, ValueType)>> = std::sync::Mutex::new(Vec::new());

/// States that a member declared as `Rc<BindableArray<T>>` (or the `Option` of it) is
/// a member of the array type `T[]`: a compiled binding path indexes it with the array
/// element node (`Property[0]`), as the path of the managed original indexes a `T[]`.
/// Process-wide and idempotent; a crate calls it from its `register_types()` for the
/// element types of the arrays its metadata declares.
pub fn register_bindable_array<T: ferroui_base::PropertyValue>() {
    use ferroui_base::data::model::BindableArray;
    let element = ValueType::of::<T>();
    let mut arrays = BINDABLE_ARRAYS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    for handle in [ValueType::of::<Rc<BindableArray<T>>>(), ValueType::of::<Option<Rc<BindableArray<T>>>>()] {
        if !arrays.iter().any(|(known, _)| known.id() == handle.id()) {
            arrays.push((handle, element));
        }
    }
}

/// The element type of the bindable array held as the Rust type `handle`.
pub(crate) fn bindable_array_element(handle: TypeId) -> Option<ValueType> {
    let arrays = BINDABLE_ARRAYS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    arrays.iter().find(|(known, _)| known.id() == handle).map(|(_, element)| *element)
}

/// A value of an array type: its element type and its elements, untyped.
/// See the module documentation.
#[derive(Clone)]
pub struct RuntimeArray {
    element_type: Rc<dyn IXamlType>,
    items: Rc<Vec<MarkupValue>>,
}

type ArrayForm = fn(&[MarkupValue], bool) -> Option<BoxedValue>;

thread_local! {
    static ARRAY_FORMS: std::cell::RefCell<std::collections::HashMap<TypeId, ArrayForm>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

impl RuntimeArray {
    pub fn new(element_type: Rc<dyn IXamlType>, items: Vec<MarkupValue>) -> Self {
        Self { element_type, items: Rc::new(items) }
    }

    pub fn element_type(&self) -> &Rc<dyn IXamlType> {
        &self.element_type
    }

    pub fn items(&self) -> &[MarkupValue] {
        &self.items
    }

    /// States that arrays of elements held as `T` are passed to members
    /// that declare `Vec<T>` or `Option<Vec<T>>`. Per thread; idempotent.
    pub fn register_element<T: Clone + PartialEq + 'static>() {
        fn form<T: Clone + PartialEq + 'static>(items: &[MarkupValue], optional: bool) -> Option<BoxedValue> {
            let mut typed: Vec<T> = Vec::with_capacity(items.len());
            for item in items {
                typed.push(ferroui_base::metadata::from_markup_value::<T>(item)?);
            }
            Some(if optional { Rc::new(Some(typed)) } else { Rc::new(typed) })
        }
        ValueTypes::register_nullable::<Vec<T>>();
        ARRAY_FORMS.with(|forms| {
            let mut forms = forms.borrow_mut();
            forms.insert(TypeId::of::<Vec<T>>(), form::<T> as ArrayForm);
            forms.insert(TypeId::of::<Option<Vec<T>>>(), form::<T> as ArrayForm);
        });
    }

    /// The array in the Rust type `target` a member declares. The error
    /// names the array type and the target.
    pub fn to_declared(&self, target: ValueType) -> Result<BoxedValue, String> {
        if target.is_object() || target.is::<RuntimeArray>() {
            return Ok(Rc::new(self.clone()));
        }
        if target.is::<Option<RuntimeArray>>() {
            return Ok(Rc::new(Some(self.clone())));
        }
        let form = ARRAY_FORMS.with(|forms| forms.borrow().get(&target.id()).copied());
        let optional = target.name().starts_with("core::option::Option<");
        form.and_then(|form| form(&self.items, optional)).ok_or_else(|| {
            format!(
                "An array of {} cannot be passed as a {}: the element type has no registered array form",
                self.element_type.full_name(),
                target.name()
            )
        })
    }
}

/// Arrays compare by identity of their storage: an array is a reference.
impl PartialEq for RuntimeArray {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
    }
}

impl fmt::Debug for RuntimeArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.element_type.full_name(), self.items.len())
    }
}

/// A value of a list of the runtime library created in markup:
/// `System.Collections.Generic.List<T>` (with the element type `T`) or
/// `System.Collections.ArrayList` (without an element type). See the module
/// documentation.
///
/// The items are held untyped in the list the items controls read
/// (`FerroList<Option<BoxedValue>>`), shared: the list a collection handle
/// receives is the list markup filled.
// Deviation (DEVIATIONS.md, Run-time type system of markup): upstream creates the
// `List<T>` and the `ArrayList` of the runtime library; here both are this list of
// untyped items.
#[derive(Clone)]
pub struct RuntimeList {
    element_type: Option<Rc<dyn IXamlType>>,
    items: Rc<FerroList<MarkupValue>>,
}

impl RuntimeList {
    pub fn new(element_type: Option<Rc<dyn IXamlType>>) -> Self {
        Self { element_type, items: Rc::new(FerroList::new()) }
    }

    /// The element type of a `List<T>`; `None` for an `ArrayList`.
    pub fn element_type(&self) -> Option<&Rc<dyn IXamlType>> {
        self.element_type.as_ref()
    }

    /// The shared list of the items.
    pub fn items(&self) -> &Rc<FerroList<MarkupValue>> {
        &self.items
    }

    pub fn count(&self) -> usize {
        self.items.count()
    }

    /// The item at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> MarkupValue {
        self.items.get(index)
    }

    /// Replaces the item at `index`. Panics if out of range.
    pub fn set(&self, index: usize, item: MarkupValue) {
        self.items.set(index, Self::item(item));
    }

    /// Adds an item and returns its index.
    pub fn add(&self, item: MarkupValue) -> usize {
        self.items.add(Self::item(item));
        self.items.count() - 1
    }

    /// An item as the list holds it: an object of the object model in its
    /// untyped form.
    fn item(item: MarkupValue) -> MarkupValue {
        item.map(|item| untyped_object_form(&item).unwrap_or(item))
    }

    /// The list in the Rust type `target` a member declares for a collection:
    /// the shared list of the items, cast to `target` with the assignability
    /// casts of the untyped value conversions. `None` if `target` takes the
    /// list as it is (an untyped target, a list) or no cast to `target` is
    /// registered.
    pub fn to_declared(&self, target: ValueType) -> Option<BoxedValue> {
        if target.is_object() || target.is::<RuntimeList>() {
            return None;
        }
        let items: BoxedValue = Rc::new(self.items.clone());
        ValueTypes::try_cast(&items, target)
    }
}

/// Lists compare by identity of their storage: a list is a reference.
impl PartialEq for RuntimeList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.items, &other.items)
    }
}

impl fmt::Debug for RuntimeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.element_type {
            Some(element_type) => write!(f, "List<{}>[{}]", element_type.full_name(), self.items.count()),
            None => write!(f, "ArrayList[{}]", self.items.count()),
        }
    }
}

/// The form in which an object of the object model is handed to an UNTYPED
/// target: the convention of the runtime library, shared with generated
/// code ([`compiled::untyped_object_form`]).
///
/// [`compiled::untyped_object_form`]: ferroui_markup_xaml::xaml_il::runtime::compiled::untyped_object_form
pub use ferroui_markup_xaml::xaml_il::runtime::compiled::untyped_object_form;

