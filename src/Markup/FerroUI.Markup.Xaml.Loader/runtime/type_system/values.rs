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
//! `RuntimeArray` itself. Registered BindableArray views share one mutable typed store.
//! Generated code uses the same runtime array. A member that declares another collection type
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

use ferroui_base::data::core::ValueType;
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
pub fn register_bindable_array<T:ferroui_base::PropertyValue>(){ferroui_base::data::model::register_bindable_array::<T>();}
pub(crate) fn bindable_array_element(handle:TypeId)->Option<ValueType>{ferroui_base::data::model::bindable_array_element(handle)}

pub use ferroui_markup_xaml::xaml_il::runtime::RuntimeArray;
struct ArrayElementType(Rc<dyn IXamlType>);
impl ferroui_markup_xaml::xaml_il::runtime::RuntimeArrayElement for ArrayElementType {
 fn full_name(&self)->String{self.0.full_name()}
 fn value_type(&self)->Option<ValueType>{self.0.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::handle)}
 fn as_any(&self)->&dyn std::any::Any{self}
}
pub(crate) fn new_array(element_type:Rc<dyn IXamlType>,items:Vec<MarkupValue>)->RuntimeArray {
 RuntimeArray::new(Rc::new(ArrayElementType(element_type)),items)
}
pub(crate) fn array_element_type(array:&RuntimeArray)->Option<Rc<dyn IXamlType>> {
 array.element_type().as_any().downcast_ref::<ArrayElementType>().map(|element|element.0.clone())
}

/// A value of a list of the runtime library created in markup:
/// `System.Collections.Generic.List<T>` (with the element type `T`) or
/// `System.Collections.ArrayList` (without an element type). See the module
/// documentation. The type is the one of the runtime library, which generated code
/// creates too: a list is the same value whichever back end built its document.
pub use ferroui_markup_xaml::xaml_il::runtime::RuntimeList;

/// The element type of a `List<T>` the run-time loader creates: the type of its type
/// system.
struct ListElementType(Rc<dyn IXamlType>);

impl ferroui_markup_xaml::xaml_il::runtime::RuntimeListElement for ListElementType {
    fn full_name(&self) -> String {
        self.0.full_name()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// `new List<T>()` for the element type `element_type`, `new ArrayList()` without one.
pub(crate) fn new_list(element_type: Option<Rc<dyn IXamlType>>) -> RuntimeList {
    RuntimeList::new(element_type.map(|element_type| {
        Rc::new(ListElementType(element_type)) as Rc<dyn ferroui_markup_xaml::xaml_il::runtime::RuntimeListElement>
    }))
}

/// The element type of `list` as a type of the type system: the type the run-time loader
/// created the list with, or, for a list generated code created, the full name it states.
pub(crate) enum ListElement {
    Type(Rc<dyn IXamlType>),
    Named(String),
}

pub(crate) fn list_element(list: &RuntimeList) -> Option<ListElement> {
    let element = list.element_type()?;
    Some(match element.as_any().downcast_ref::<ListElementType>() {
        Some(element) => ListElement::Type(element.0.clone()),
        None => ListElement::Named(element.full_name()),
    })
}

/// The form in which an object of the object model is handed to an UNTYPED
/// target: the convention of the runtime library, shared with generated
/// code ([`compiled::untyped_object_form`]).
///
/// [`compiled::untyped_object_form`]: ferroui_markup_xaml::xaml_il::runtime::compiled::untyped_object_form
pub use ferroui_markup_xaml::xaml_il::runtime::compiled::untyped_object_form;

