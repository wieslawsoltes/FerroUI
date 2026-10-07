//! The helpers that Rust source generated from markup calls (the emitter of
//! the ahead-of-time compiler, docs/porting/xaml.md, section 9.9 item R3).
//!
//! Not a port of an upstream file: upstream's compiler emits IL that calls
//! the members of the framework and the runtime helpers directly and lets
//! their exceptions propagate. Generated Rust has an error channel instead
//! ([`XamlLoadException`]); these helpers perform the steps of the run-time
//! loader whose failure is a load error, with the same messages, so that a
//! document built by generated code fails exactly where and as the run-time
//! loader fails.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::Rc;

use ferroui_base::controls::{INameScope, NameScope, NameScopeError, NameScopeRef};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{
    from_markup_value, into_markup_value, service, IServiceProvider, MarkupInvokeError, MarkupValue,
};
use ferroui_base::utilities::Uri;
use ferroui_base::data::core::{
    BoxedPropertyGetter, BoxedPropertySetter, ClrPropertyInfo, FalliblePropertyGetter, IPropertyInfo, PropertySetter,
};
use ferroui_base::data::core::plugins::PropertyAccessorFactory;
use ferroui_base::data::{BindingBase, BindingError, BindingPriority, CompiledBindingPathBuilder};
use ferroui_base::metadata::{MarkupInvoke, MarkupProperty, MarkupType};
use ferroui_base::AnyValue;
use ferroui_base::styling::{Setter, SetterBase, StyleBase};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, TypeInfo, UnsetValueType};

use super::{
    DeferredContent, DeferredContentBuilder, FerroXamlIlXmlNamespaceInfo, FrameworkContextServices,
    IFerroXamlIlXmlNamespaceInfoProvider, IStaticServiceProvider, IXamlIlContextServices, XamlIlContext,
    XamlIlContextDefinition, XamlIlRuntimeHelpers, XmlNamespaces,
};
use crate::markup_extensions::compiled_bindings::PropertyInfoAccessorFactory;
use crate::{ServiceProviderExtensions, XamlLoadException};

/// The exception a step of generated code failed with: the type name of the
/// exception the managed original throws there (the name the run-time
/// loader gives its error as well, `XamlError::type_name` of the loader
/// crate), its message and the position of the node being built. It is the
/// inner error of the [`XamlLoadException`] the step returns
/// ([`at`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledLoadError {
    type_name: &'static str,
    message: String,
    line_number: i32,
    line_position: i32,
}

impl CompiledLoadError {
    /// The name of the exception type (`ArgumentException`,
    /// `TargetInvocationException`, ...).
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The message, without the position.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The line of the node that was being built.
    pub fn line_number(&self) -> i32 {
        self.line_number
    }

    /// The position of the node that was being built.
    pub fn line_position(&self) -> i32 {
        self.line_position
    }
}

impl Display for CompiledLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.type_name, self.message)
    }
}

impl std::error::Error for CompiledLoadError {}

/// The load error of a failure, an exception of the type `type_name`, at
/// the position `line`, `position` of the document, with the message the
/// run-time loader gives a failure of the node it was evaluating: the
/// message of the load exception of the compiler (the message followed by
/// `Line <line>, position <position>.` unless both are 0), then the
/// position in parentheses. The inner error is a [`CompiledLoadError`]
/// that carries the type name.
pub fn at(type_name: &'static str, message: impl Display, line: i32, position: i32) -> XamlLoadException {
    let message = message.to_string();
    let positioned = match (line, position) {
        (0, 0) => message.clone(),
        _ => format!("{message} Line {line}, position {position}."),
    };
    XamlLoadException::with_inner(
        format!("{positioned} (line {line} position {position})"),
        CompiledLoadError { type_name, message, line_number: line, line_position: position },
    )
}

/// Whether `uri` is the URI `root` followed by `name`, compared as upstream's
/// generated loader compares a requested URI with the URI of a document:
/// `string.Equals(uri, documentUri, StringComparison.OrdinalIgnoreCase)`, an
/// ordinal comparison of the characters after their simple (one character to
/// one character) upper-case mapping, for every script, not only ASCII.
pub fn uri_equals(uri: &str, root: &str, name: &str) -> bool {
    fn fold(character: char) -> char {
        let mut upper = character.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(single), None) => single,
            // A mapping to several characters (`ß` -> `SS`) is not a simple mapping.
            _ => character,
        }
    }
    let mut expected = root.chars().chain(name.chars());
    let mut actual = uri.chars();
    loop {
        match (actual.next(), expected.next()) {
            (None, None) => return true,
            (Some(a), Some(b)) if a == b || fold(a) == fold(b) => {}
            _ => return false,
        }
    }
}

/// `target.Bind(property, binding)`: what the binding setter of a registered
/// property does. `value` is viewed through the binding contract as the
/// run-time loader views it (in its untyped form, [`to_value`]); a value that
/// is no binding is the loader's error at `line`, `position` (the position of
/// the value).
pub fn bind<V: PartialEq + 'static>(
    target: &Ref<FerroObject>,
    property: &'static FerroProperty,
    value: V,
    line: i32,
    position: i32,
) -> Result<(), XamlLoadException> {
    let value = to_value(value);
    let binding = from_markup_value::<Rc<dyn BindingBase>>(&value).ok_or_else(|| match &value {
        None => at("NullReferenceException", "The binding is null", line, position),
        Some(value) => at(
            "InvalidCastException",
            format!("Unable to cast object of type '{}' to type 'FerroUI.Data.BindingBase'.", value.type_name()),
            line,
            position,
        ),
    })?;
    let _ = target.bind_binding(property, &*binding);
    Ok(())
}

/// `value` boxed as it is (the boxing of a value the run-time loader makes
/// without a conversion).
pub fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

/// The canonical handle of a class of the object model (`Ref<T>`): the type
/// a run-time type check of generated code names a class by.
pub fn class_handle(class: &'static TypeInfo) -> ValueType {
    let handle = class.handle().unwrap_or_else(|| panic!("{} has no handle", class.full_name()));
    ValueType::new(handle, class.name())
}

/// The canonical handle of a markup type (its first handle): the type a
/// run-time type check of generated code names a markup type by.
pub fn markup_handle(markup: &'static MarkupType) -> ValueType {
    let handle = markup.handles.first().unwrap_or_else(|| panic!("{} has no handle", markup.full_name()));
    handle()
}

/// `value is T` for the type with the handle `target` (the `isinst` check
/// the run-time choice of a property setter makes): `value` is not null and
/// its run-time type (the class of an object, else the type of the value) is
/// assignable to `target`.
///
/// The run-time loader asks its type system, which projects the same
/// metadata ([`TypeInfo`] for classes, the contracts and bases a markup type
/// declares); generated code asks the assignability of the untyped value
/// conversions ([`ValueTypes::is_assignable`]), which the declarations of
/// that metadata register.
pub fn is_instance(value: &MarkupValue, target: ValueType) -> bool {
    let Some(value) = value else { return false };
    if let Some(object) = ValueTypes::as_object(&**value) {
        let class = object.get_type();
        if let Some((target_class, false)) = TypeInfo::find_by_handle(target.id()) {
            return target_class.is_assignable_from(class);
        }
        return match class.handle() {
            Some(handle) => ValueTypes::is_assignable(ValueType::new(handle, class.name()), target),
            None => false,
        };
    }
    ValueTypes::is_assignable(ValueType::of_value(&**value), target)
}

/// The value `value` as the Rust type `T` a member declares, with the
/// conversion of the run-time loader (`to_exact`: the value as it is, the
/// assignability casts of the untyped value conversions, null converted to
/// the null of `T`). A value that does not convert is the loader's error for
/// the argument `index` of `member` at `line`, `position`.
pub fn exact<T: Clone + 'static>(
    value: MarkupValue,
    member: &str,
    index: usize,
    line: i32,
    position: i32,
) -> Result<T, XamlLoadException> {
    let target = ValueType::of::<T>();
    let value = match (&value, target.is_object()) {
        (Some(boxed), true) => untyped_object_form(boxed).map(Some).unwrap_or(value),
        _ => value,
    };
    let converted = match &value {
        Some(boxed) if boxed.value_type_id() == target.id() => Some(boxed.clone()),
        Some(boxed) => ValueTypes::try_cast(boxed, target),
        None => ValueTypes::try_convert(None, target).flatten(),
    };
    match converted.as_ref().and_then(|converted| converted.downcast_ref::<T>()) {
        Some(converted) => Ok(converted.clone()),
        None => {
            let error = MarkupInvokeError::Argument {
                index,
                expected: target.name(),
                actual: match &value {
                    Some(value) => value.type_name().to_string(),
                    None => "null".to_string(),
                },
            };
            Err(at("InvalidCastException", format!("{member}: {error}"), line, position))
        }
    }
}

/// The type name of the exception a member call of generated code that
/// fails with an error of its own is reported as: the run-time loader
/// reports the failure of a member it invokes (`EndInit`, a setter) as the
/// exception that wraps the exception of the member.
pub const TARGET_INVOCATION_EXCEPTION: &str = "TargetInvocationException";

/// The result of a member generated code invoked, with its error as the
/// load error the run-time loader reports for the failure of a member it
/// invokes: the exception that wraps the exception of the member
/// ([`TARGET_INVOCATION_EXCEPTION`]) at `line`, `position` ([`at`]). The
/// error path is one function for every member, outside the generated
/// code.
#[inline]
pub fn invoked<T, E: Display>(result: Result<T, E>, line: i32, position: i32) -> Result<T, XamlLoadException> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(invocation_error(&error, line, position)),
    }
}

#[cold]
#[inline(never)]
fn invocation_error(error: &dyn Display, line: i32, position: i32) -> XamlLoadException {
    at(TARGET_INVOCATION_EXCEPTION, error, line, position)
}

/// The name scope field of the context of a document
/// (`FerroXamlIlContextNameScopeField`): the name scope of the parent
/// service provider, `None` when it has none.
pub fn name_scope_of(parent: Option<&Rc<dyn IServiceProvider>>) -> Option<Rc<dyn INameScope>> {
    parent.and_then(|parent| parent.get_name_scope())
}

/// The value of `Setter.Value` as the run-time loader passes it to the
/// setter: converted to the exact type of the property the setter already
/// names (managed code assigns an instance of that type already; here the
/// untyped value is brought to the property's Rust type with the
/// assignability casts of the untyped value conversions). A value that is
/// not of the type of the property (a binding, the unset marker, a template
/// the setter instantiates), and any value of a setter without its property,
/// is passed as it is.
pub fn setter_value(setter: &ferroui_base::styling::Setter, value: MarkupValue) -> MarkupValue {
    let (Some(boxed), Some(property)) = (&value, setter.property()) else { return value };
    let target = ValueType::new(property.property_type(), property.property_type_name());
    let boxed = match target.is_object() {
        true => untyped_object_form(boxed).unwrap_or_else(|| boxed.clone()),
        false => boxed.clone(),
    };
    let converted = match boxed.value_type_id() == target.id() {
        true => Some(boxed),
        false => ValueTypes::try_cast(&boxed, target),
    };
    match converted {
        Some(converted) => Some(converted),
        None => value,
    }
}

/// A setter added to a style (`<Setter Property=".." Value=".."/>` in a style
/// or a control theme): `new Setter()`, `Property` set to `property`,
/// `Value` set to `value` ([`setter_value`]), then `style.Add(setter)`, the
/// calls the statements of generated code make, through the same typed
/// functions. `value` has no statements of its own; it is evaluated before
/// the call (it reads nothing of the setter).
#[inline(never)]
pub fn add_setter(style: &Ref<StyleBase>, property: &'static FerroProperty, value: MarkupValue) {
    let setter = new_setter(property);
    add_setter_value(style, &setter, value);
}

/// The first half of [`add_setter`] for a value with statements of its own:
/// `new Setter()` with `Property` set to `property`.
#[inline(never)]
pub fn new_setter(property: &'static FerroProperty) -> Rc<Setter> {
    let setter = Setter::__markup_new_0();
    Setter::__markup_set_Property(&setter, Some(property));
    setter
}

/// The second half of [`add_setter`]: `Value` of `setter` set to `value`
/// ([`setter_value`]), then `style.Add(setter)`.
#[inline(never)]
pub fn add_setter_value(style: &Ref<StyleBase>, setter: &Rc<Setter>, value: MarkupValue) {
    Setter::__markup_set_Value(setter, setter_value(setter, value));
    StyleBase::__markup_Add_0(style, setter.clone() as Rc<dyn SetterBase>);
}

/// [`new_setter`] of a setter that is on the parent stack of `context`
/// while its members are set: pushed after it is created, before `Property`
/// is set.
#[inline(never)]
pub fn new_setter_with_parent(context: &Rc<XamlIlContext>, property: &'static FerroProperty) -> Rc<Setter> {
    let setter = Setter::__markup_new_0();
    context.push_parent(to_value(setter.clone()));
    Setter::__markup_set_Property(&setter, Some(property));
    setter
}

/// [`add_setter_value`] of a setter on the parent stack of `context`:
/// popped after `Value` is set, before it is added to `style`.
#[inline(never)]
pub fn add_setter_value_with_parent(context: &Rc<XamlIlContext>, style: &Ref<StyleBase>, setter: &Rc<Setter>, value: MarkupValue) {
    Setter::__markup_set_Value(setter, setter_value(setter, value));
    context.pop_parent();
    StyleBase::__markup_Add_0(style, setter.clone() as Rc<dyn SetterBase>);
}

/// `target.SetValue(property, FerroProperty.UnsetValue, BindingPriority.LocalValue)`:
/// what the unset-value setter of a registered property does.
pub fn unset_value(target: &Ref<FerroObject>, property: &'static FerroProperty) {
    let _ = target.set_value_untyped(property, &UnsetValueType, BindingPriority::LocalValue);
}

/// The full name of the run-time type of a value, as the error of a property
/// assignment names it: the class of an object, else the markup type of the
/// value, else the Rust type.
fn runtime_type_name(value: &BoxedValue) -> String {
    if let Some(object) = ValueTypes::as_object(&**value) {
        return object.get_type().full_name();
    }
    match MarkupType::find_by_handle(value.value_type_id()) {
        Some(markup) => markup.full_name(),
        None => value.type_name().to_string(),
    }
}

/// `castclass` of a value of type `object` to the reference type with the
/// handle `target` (named `target_name`): the conversion the run-time loader
/// applies where a value of type `object` is expected as another reference
/// type. Null passes; an instance of the type is held in the handle of its
/// run-time class; anything else is the loader's error.
pub fn cast_checked(
    value: MarkupValue,
    target: ValueType,
    target_name: &str,
    line: i32,
    position: i32,
) -> Result<MarkupValue, XamlLoadException> {
    let Some(boxed) = &value else { return Ok(None) };
    if is_instance(&value, target) {
        return Ok(Some(normalize_object(boxed.clone())));
    }
    Err(at(
        "InvalidCastException",
        format!("Unable to cast object of type '{}' to type '{target_name}'.", runtime_type_name(boxed)),
        line,
        position,
    ))
}

/// The error of a property assignment whose setter is chosen at run time
/// when no setter takes the value: `value` null (no setter allows null) or
/// of a type none of them takes.
pub fn no_setter(property: &str, value: &MarkupValue, line: i32, position: i32) -> XamlLoadException {
    match value {
        None => at("NullReferenceException", format!("No setter of property {property} accepts null"), line, position),
        Some(value) => at(
            "InvalidCastException",
            format!("No setter of property {property} accepts a value of type '{}'", runtime_type_name(value)),
            line,
            position,
        ),
    }
}

/// The definition of a registered property as the property itself (the
/// value of the static field that holds it).
pub fn property(property: &'static FerroProperty) -> &'static FerroProperty {
    property
}

/// A registered property as a value (the descriptor of the provide-value
/// target property): its definition, as the run-time loader boxes it.
pub fn property_value(property: &'static FerroProperty) -> MarkupValue {
    boxed(property)
}

/// The metadata of a class of the object model.
///
/// # Panics
/// Panics if the class has none: the emitter names only classes whose
/// metadata it found.
pub fn class_markup(class: &'static TypeInfo) -> &'static MarkupType {
    MarkupType::find_by_type_info(class).unwrap_or_else(|| panic!("{} has no metadata", class.full_name()))
}

/// The description of the plain (declared) property `name` of the markup
/// type `markup` (`XamlIlClrPropertyInfoEmitter`): what the provide-value
/// target property of a markup extension is for a property that is not a
/// registered one. Its accessors invoke the accessors of the declaration as
/// the run-time loader invokes them (an object passed untyped in the
/// framework's untyped form, an object returned in the handle of its
/// run-time class, a failure named by the accessor); `property_type` is the
/// handle of the type of the property; a getter of a `System.Boolean`
/// property (`cached_boxed_boolean`) returns the shared boxes.
///
/// There is one description per declared property and thread, created
/// when it is first asked for, as upstream's compiler keeps one per property
/// in a static field of the generated helper type of the assembly
/// (`XamlIlClrPropertyInfoEmitter`, `<Type>.<Name>!Field`); `property_type`
/// and `cached_boxed_boolean` follow from the declaration.
///
/// # Panics
/// Panics if `markup` declares no such property: the emitter writes the
/// call only for a declared property.
pub fn clr_property_info(
    markup: &'static MarkupType,
    name: &'static str,
    is_static: bool,
    property_type: ValueType,
    cached_boxed_boolean: bool,
) -> Rc<dyn IPropertyInfo> {
    thread_local! {
        static INFOS: RefCell<HashMap<(usize, &'static str, bool), Rc<dyn IPropertyInfo>>> = RefCell::new(HashMap::new());
    }
    let key = (markup as *const MarkupType as usize, name, is_static);
    if let Some(info) = INFOS.with(|infos| infos.borrow().get(&key).cloned()) {
        return info;
    }
    let info = new_clr_property_info(markup, name, is_static, property_type, cached_boxed_boolean);
    INFOS.with(|infos| infos.borrow_mut().insert(key, info.clone()));
    info
}

/// The description of `Setter.Value` ([`clr_property_info`]) as a value:
/// the provide-value target property of a markup extension that gives a
/// setter its value.
#[inline(never)]
pub fn setter_value_property() -> MarkupValue {
    boxed(clr_property_info(
        <Setter as ferroui_base::metadata::MarkupTyped>::MARKUP,
        "Value",
        false,
        ValueType::object(),
        false,
    ))
}

fn new_clr_property_info(
    markup: &'static MarkupType,
    name: &str,
    is_static: bool,
    property_type: ValueType,
    cached_boxed_boolean: bool,
) -> Rc<dyn IPropertyInfo> {
    let declared = match is_static {
        true => markup.static_properties.iter().find(|property| property.name == name),
        false => markup.find_property(name),
    };
    let declared = declared.unwrap_or_else(|| panic!("{} declares no property {name}", markup.full_name()));
    let accessors = Rc::new(DeclaredAccessors {
        owner: markup.full_name(),
        property: declared,
        is_static,
        cached_boxed_boolean,
    });
    let getter: Option<FalliblePropertyGetter> = declared.get.is_some().then(|| {
        let accessors = accessors.clone();
        Rc::new(move |target: &dyn AnyValue| accessors.get(|| owner_handle(target))) as FalliblePropertyGetter
    });
    let setter: Option<PropertySetter> = declared.set.is_some().then(|| {
        let accessors = accessors.clone();
        Rc::new(move |target: &dyn AnyValue, value: Option<&BoxedValue>| accessors.set(|| owner_handle(target), value))
            as PropertySetter
    });
    let boxed_getter: Option<BoxedPropertyGetter> = declared.get.is_some().then(|| {
        let accessors = accessors.clone();
        Rc::new(move |target: &BoxedValue| accessors.get(|| Ok(Some(target.clone())))) as BoxedPropertyGetter
    });
    let boxed_setter: Option<BoxedPropertySetter> = declared.set.is_some().then(|| {
        let accessors = accessors.clone();
        Rc::new(move |target: &BoxedValue, value: Option<&BoxedValue>| accessors.set(|| Ok(Some(target.clone())), value))
            as BoxedPropertySetter
    });
    Rc::new(
        ClrPropertyInfo::new_fallible(name, getter, setter, property_type).with_boxed_accessors(boxed_getter, boxed_setter),
    )
}

/// The element of a compiled binding path for the plain (declared) property
/// `name` of `markup` (`builder.Property(info, accessorFactory[, acceptsNull])`):
/// when the path is typed (`typed`, a path that is this single property), the
/// typed element the declaration of the property generates, if it has one;
/// otherwise the element over [`clr_property_info`] with the accessor
/// factory that follows property change notifications.
#[allow(clippy::too_many_arguments)]
pub fn path_property(
    builder: &CompiledBindingPathBuilder,
    markup: &'static MarkupType,
    name: &'static str,
    is_static: bool,
    property_type: ValueType,
    cached_boxed_boolean: bool,
    accepts_null: bool,
    typed: bool,
) -> CompiledBindingPathBuilder {
    if typed {
        let declared = match is_static {
            true => None,
            false => markup.find_property(name),
        };
        if let Some(typed) = declared.and_then(|property| property.typed_path_element) {
            if let Some(builder) = typed(builder, accepts_null) {
                return builder;
            }
        }
    }
    let info = clr_property_info(markup, name, is_static, property_type, cached_boxed_boolean);
    let factory: PropertyAccessorFactory = Rc::new(PropertyInfoAccessorFactory::create_inpc_property_accessor);
    match accepts_null {
        true => builder.property_with(info, factory, true),
        false => builder.property(info, factory),
    }
}

/// The name scope of an element name in a binding path: the name scope field
/// of the context; none is the loader's error.
pub fn path_name_scope(scope: Option<&Rc<dyn INameScope>>, line: i32, position: i32) -> Result<NameScopeRef, XamlLoadException> {
    scope
        .cloned()
        .map(NameScopeRef)
        .ok_or_else(|| at("ArgumentNullException", "Value cannot be null. (Parameter 'nameScope')", line, position))
}

/// The accessors of a declared property, invoked as the run-time loader
/// invokes the accessor methods it projects from the declaration.
struct DeclaredAccessors {
    owner: String,
    property: &'static MarkupProperty,
    is_static: bool,
    cached_boxed_boolean: bool,
}

impl DeclaredAccessors {
    fn invoke(
        &self,
        accessor: &str,
        invoke: MarkupInvoke,
        owner: impl FnOnce() -> Result<MarkupValue, BindingError>,
        value: Option<Option<&BoxedValue>>,
    ) -> Result<MarkupValue, BindingError> {
        let mut arguments = Vec::with_capacity(2);
        if !self.is_static {
            arguments.push(owner()?);
        }
        if let Some(value) = value {
            // An object passed to a property of type `object` is handed over in the untyped form.
            let value = match value {
                Some(value) if (self.property.type_)().is_object() => untyped_object_form(value).or(Some(value.clone())),
                other => other.cloned(),
            };
            arguments.push(value);
        }
        let result = invoke(&arguments).map_err(|error| {
            let error = match error {
                MarkupInvokeError::Failed(message) => {
                    MarkupInvokeError::Failed(format!("{}.{accessor}_{}: {message}", self.owner, self.property.name))
                }
                other => other,
            };
            BindingError::message(error.to_string())
        })?;
        Ok(result.map(normalize_object))
    }

    fn get(&self, owner: impl FnOnce() -> Result<MarkupValue, BindingError>) -> Result<Option<BoxedValue>, BindingError> {
        let get = self
            .property
            .get
            .ok_or_else(|| BindingError::message(format!("Property {} doesn't have a getter", self.property.name)))?;
        let value = self.invoke("get", get, owner, None)?;
        if self.cached_boxed_boolean {
            if let Some(value) = value.as_ref().and_then(|value| value.downcast_ref::<bool>()) {
                return Ok(Some(ferroui_base::utilities::BooleanBoxes::box_(*value)));
            }
        }
        Ok(value)
    }

    fn set(&self, owner: impl FnOnce() -> Result<MarkupValue, BindingError>, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        let set = self
            .property
            .set
            .ok_or_else(|| BindingError::message(format!("Property {} doesn't have a setter", self.property.name)))?;
        self.invoke("set", set, owner, Some(value)).map(|_| ())
    }
}

/// The owner of a property in the form invokers take it: an object of the
/// object model in the handle of its run-time class.
fn owner_handle(target: &dyn AnyValue) -> Result<MarkupValue, BindingError> {
    match ValueTypes::as_object(target).and_then(box_object) {
        Some(handle) => Ok(Some(handle)),
        None => Err(BindingError::message(format!(
            "A property of '{}' cannot be read through metadata: the value is not an object of the object model",
            target.type_name()
        ))),
    }
}

/// The context as the service provider handed to user code (markup
/// extensions, constructors that take a service provider).
pub fn service_provider(context: &Rc<XamlIlContext>) -> Rc<dyn IServiceProvider> {
    context.clone()
}

/// `ProvideValue` of a markup extension with the context as its service
/// provider, as the run-time loader calls it: the provide-value target
/// property of the context set to `target` (the property the value is
/// provided for), `provide` called with `extension` and the context, then
/// the target property cleared.
///
/// Generic only over the extension and the result; it is instantiated once
/// per `ProvideValue` function, not per call site, and setting and clearing
/// the target property are functions of their own.
#[inline(never)]
pub fn provide_value<E: ?Sized, R>(
    context: &Rc<XamlIlContext>,
    target: MarkupValue,
    extension: &E,
    provide: impl FnOnce(&E, Rc<dyn IServiceProvider>) -> R,
) -> R {
    let provided = provide(extension, target_service_provider(context, target));
    clear_target_property(context);
    provided
}

/// [`provide_value`] of a `ProvideValue` that fails with an error of its
/// own: the error is the load error of a member the run-time loader invokes
/// ([`invoked`]) at `line`, `position`, and the target property stays set,
/// as it does when the run-time loader's call fails.
#[inline(never)]
pub fn provide_value_invoked<E: ?Sized, T, F: Display>(
    context: &Rc<XamlIlContext>,
    target: MarkupValue,
    extension: &E,
    provide: impl FnOnce(&E, Rc<dyn IServiceProvider>) -> Result<T, F>,
    line: i32,
    position: i32,
) -> Result<T, XamlLoadException> {
    let provided = invoked(provide(extension, target_service_provider(context, target)), line, position)?;
    clear_target_property(context);
    Ok(provided)
}

/// Sets the provide-value target property of `context` to `target` and
/// returns the context as the service provider ([`provide_value`]).
#[inline(never)]
fn target_service_provider(context: &Rc<XamlIlContext>, target: MarkupValue) -> Rc<dyn IServiceProvider> {
    context.set_target_property(target);
    service_provider(context)
}

#[inline(never)]
fn clear_target_property(context: &Rc<XamlIlContext>) {
    context.set_target_property(None);
}

/// The XML namespaces of a document as the compiler resolved them: for each
/// prefix (empty for the default namespace) the dotted namespaces and
/// assemblies it maps to (the namespace information the run-time loader
/// computes from the parsed document).
pub type XmlNamespaceTable = &'static [(&'static str, &'static [(&'static str, &'static str)])];

/// The definition of the context of the framework language: every service
/// of the context is mapped (the transformer configuration of the language
/// sets every mapping; the emitter checks that the configuration it compiles
/// with describes this definition).
pub const FRAMEWORK_CONTEXT: XamlIlContextDefinition = XamlIlContextDefinition {
    root_object_provider: true,
    parent_stack_provider: true,
    type_descriptor_context: true,
    provide_value_target: true,
    uri_context_provider: true,
    xml_namespace_info_provider: true,
};

/// The namespace information of a compiled document, as a static provider
/// of its contexts.
struct CompiledXmlNamespaceInfo {
    table: XmlNamespaceTable,
}

impl IFerroXamlIlXmlNamespaceInfoProvider for CompiledXmlNamespaceInfo {
    fn xml_namespaces(&self) -> XmlNamespaces {
        let mut namespaces = HashMap::with_capacity(self.table.len());
        for (prefix, infos) in self.table {
            let infos = infos
                .iter()
                .map(|(clr_namespace, clr_assembly_name)| FerroXamlIlXmlNamespaceInfo::with(clr_namespace, clr_assembly_name))
                .collect();
            namespaces.insert(prefix.to_string(), infos);
        }
        Rc::new(namespaces)
    }
}

impl IStaticServiceProvider for CompiledXmlNamespaceInfo {
    fn get_static_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        service(service_type, || {
            Rc::new(CompiledXmlNamespaceInfo { table: self.table }) as Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>
        })
    }
}

/// The context factory of a compiled document: the context of the framework
/// language with `parent` as its parent service provider, the document's
/// base URI and namespace information, the name scope field filled from
/// `parent`, and last the inner service provider
/// (`XamlIlRuntimeHelpers.CreateInnerServiceProviderV1`): what the run-time
/// loader's context factory does for a `Populate` or a deferred build.
pub fn create_context(
    parent: Option<Rc<dyn IServiceProvider>>,
    base_uri: Option<&str>,
    namespaces: XmlNamespaceTable,
) -> Rc<XamlIlContext> {
    let static_providers: Rc<[Rc<dyn IStaticServiceProvider>]> =
        Rc::new([Rc::new(CompiledXmlNamespaceInfo { table: namespaces }) as Rc<dyn IStaticServiceProvider>]);
    // The emitter writes the URI of the document, which is absolute.
    let base_uri = base_uri.and_then(|uri| Uri::absolute(uri).ok());
    let context =
        XamlIlContext::new(FRAMEWORK_CONTEXT, Rc::new(FrameworkContextServices), parent, static_providers, base_uri);
    context.initialize_name_scope_field();
    let inner = XamlIlRuntimeHelpers::create_inner_service_provider_v1(context.service_provider_below_inner());
    context.set_inner_service_provider(Some(inner));
    context
}

/// The context of a build of deferred content (a template, a deferred
/// resource) called with `service_provider`: [`create_context`] chained to
/// it, with the root object of `service_provider` as its root object, as
/// the run-time loader's build of deferred content creates it.
pub fn deferred_context(
    service_provider: &Rc<dyn IServiceProvider>,
    base_uri: Option<&str>,
    namespaces: XmlNamespaceTable,
) -> Rc<XamlIlContext> {
    let context = create_context(Some(service_provider.clone()), base_uri, namespaces);
    if let Some(root) = FrameworkContextServices.get_parent_root_object(service_provider) {
        context.set_root_object(root);
    }
    context
}

/// `XamlIlRuntimeHelpers.DeferredTransformationFactoryV3<T>(builder, context)`:
/// the deferred content of `builder` whose result is of the type with the
/// handle `result_type` (the "any value" type when the language gives no type
/// argument), capturing the resource nodes, the root object and the name
/// scope of `context`.
pub fn deferred_content(
    result_type: ValueType,
    context: &Rc<XamlIlContext>,
    builder: DeferredContentBuilder,
    line: i32,
    position: i32,
) -> Result<Rc<DeferredContent>, XamlLoadException> {
    XamlIlRuntimeHelpers::try_deferred_transformation_factory_for(result_type, builder, &service_provider(context))
        .map_err(|error| at("InvalidOperationException", error.message(), line, position))
}

/// The build function of deferred content as generated code writes it: a
/// function of its own per template or deferred resource, which creates its
/// context ([`deferred_context`]) from the service provider it is called with
/// and builds the content anew on every call.
pub type DeferredBuild = fn(&Rc<dyn IServiceProvider>) -> Result<MarkupValue, XamlLoadException>;

/// [`deferred_content`] of the build function `build`: every piece of
/// deferred content of generated code goes through this one function (a
/// function pointer, not a closure, so nothing is instantiated per template).
pub fn defer(
    result_type: ValueType,
    context: &Rc<XamlIlContext>,
    build: DeferredBuild,
    line: i32,
    position: i32,
) -> Result<Rc<DeferredContent>, XamlLoadException> {
    deferred_content(result_type, context, DeferredContentBuilder::try_new(build), line, position)
}

/// The context of `Populate` of a root object: [`create_context`], with the
/// root object and the intermediate root object set to `root`.
pub fn populate_context(
    parent: Option<Rc<dyn IServiceProvider>>,
    base_uri: Option<&str>,
    namespaces: XmlNamespaceTable,
    root: MarkupValue,
) -> Rc<XamlIlContext> {
    let context = create_context(parent, base_uri, namespaces);
    context.set_root_object(root.clone());
    context.set_intermediate_root_object(root);
    context
}

/// `context.FerroNameScope.Register(name, element)`: registers `element`
/// under `name` in the name scope of the context. A missing scope, a
/// completed scope and a duplicate name are load errors at the position of
/// the registration.
pub fn register_name(
    scope: Option<&Rc<dyn INameScope>>,
    name: &str,
    element: Ref<FerroObject>,
    line: i32,
    position: i32,
) -> Result<(), XamlLoadException> {
    let scope = scope.ok_or_else(|| {
        at("NullReferenceException", "The runtime context has no name scope to register a name in", line, position)
    })?;
    scope.try_register(name, element).map_err(|error: NameScopeError| {
        let type_name = match error {
            NameScopeError::Completed => "InvalidOperationException",
            NameScopeError::DuplicateName(_) => "ArgumentException",
        };
        at(type_name, error, line, position)
    })
}

/// The handling of the scope of the root object of a document: when the
/// root is a styled element its name scope becomes `scope`, then the scope
/// is completed. A context without a name scope is a load error at the
/// position of the root object, after the name scope of the root was
/// cleared.
pub fn complete_root_name_scope(
    root: Option<&StyledElement>,
    scope: Option<&Rc<dyn INameScope>>,
    line: i32,
    position: i32,
) -> Result<(), XamlLoadException> {
    if let Some(root) = root {
        NameScope::set_name_scope(root, scope.cloned().map(NameScopeRef));
    }
    let scope =
        scope.ok_or_else(|| at("NullReferenceException", "The runtime context has no name scope to complete", line, position))?;
    scope.complete();
    Ok(())
}

/// The form in which an object of the object model is handed to an UNTYPED
/// target (a property of type `object`: content, a tag, an item, a setter
/// value). The framework's convention for controls held in untyped values
/// is the handle of the control base class (`Ref<Control>`), whatever the
/// class of the control; other objects are held in the handle of their own
/// class. `None` if `value` already is in that form (or is no object).
///
/// The run-time loader converts the arguments of untyped members with it;
/// generated code reaches it through [`to_object`].
pub fn untyped_object_form(value: &BoxedValue) -> Option<BoxedValue> {
    thread_local! {
        static CONTROL: std::cell::OnceCell<Option<(&'static TypeInfo, ValueType)>> =
            const { std::cell::OnceCell::new() };
    }
    let object = ValueTypes::as_object(&**value)?;
    let control = CONTROL.with(|control| {
        *control.get_or_init(|| {
            let type_info = TypeInfo::find("FerroUI.Controls", "Control")?;
            Some((type_info, ValueType::new(type_info.handle()?, type_info.name())))
        })
    });
    let (control_type, control_handle) = control?;
    if !control_type.is_assignable_from(object.get_type()) || value.value_type_id() == control_handle.id() {
        return None;
    }
    let root: BoxedValue = Rc::new(object);
    ValueTypes::try_convert_registered(&root, control_handle)
}

/// `value` as the value of a member of type `object` (`System.Object`):
/// boxed, an object of the object model in its untyped form
/// ([`untyped_object_form`]), then cast to the untyped value, which is what
/// a member typed `object` receives from the run-time loader.
pub fn to_object<T: PartialEq + 'static>(value: T) -> MarkupValue {
    let boxed: BoxedValue = Rc::new(value);
    let boxed = untyped_object_form(&boxed).unwrap_or(boxed);
    if let Some(untyped) = boxed.downcast_ref::<Option<BoxedValue>>() {
        return untyped.clone();
    }
    match ValueTypes::try_cast(&boxed, ValueType::of::<Option<BoxedValue>>()) {
        Some(untyped) => untyped.downcast_ref::<Option<BoxedValue>>().cloned().flatten(),
        None => Some(boxed),
    }
}

/// Holds an object in the handle of its run-time class (`Ref<Button>` for a
/// button held as `Ref<Control>`), the form in which the assignability
/// casts of the untyped value conversions accept it for every base class
/// and interface. Other values are returned unchanged.
pub fn normalize_object(value: BoxedValue) -> BoxedValue {
    let Some(object) = ValueTypes::as_object(&*value).or_else(|| object_behind_contract(&value)) else {
        return value;
    };
    let type_info = object.get_type();
    match type_info.handle() {
        Some(handle) if handle != value.value_type_id() => box_object(object).unwrap_or(value),
        _ => value,
    }
}

/// Boxes an object in the handle of its run-time class.
pub fn box_object(object: Ref<FerroObject>) -> Option<BoxedValue> {
    let type_info = object.get_type();
    let handle = type_info.handle()?;
    let root: BoxedValue = Rc::new(object);
    if handle == root.value_type_id() {
        return Some(root);
    }
    ValueTypes::try_convert_registered(&root, ValueType::new(handle, type_info.name()))
}

/// `value` as the run-time loader holds the value of a node: boxed, in its
/// untyped form ([`to_untyped`]), an object in the handle of its run-time
/// class.
pub fn to_value<T: PartialEq + 'static>(value: T) -> MarkupValue {
    to_untyped(Rc::new(value))
}

/// The untyped (canonical) form of a value held in a typed box: null or the
/// contents of a nullable, the object itself for a reference type.
pub fn to_untyped(value: BoxedValue) -> MarkupValue {
    match ValueTypes::try_cast(&value, ValueType::object()) {
        Some(untyped) => match untyped.downcast_ref::<Option<BoxedValue>>() {
            Some(untyped) => untyped.clone().map(normalize_object),
            None => Some(value),
        },
        None => Some(value),
    }
}

/// The object of the object model behind a contract handle, for the
/// contracts of the base library that can tell (a property typed with the
/// contract returns its object through the contract handle, and the members
/// of the class of the object must be callable on it, as on the reference
/// of the managed original).
fn object_behind_contract(value: &BoxedValue) -> Option<Ref<FerroObject>> {
    use ferroui_base::controls::{IResourceDictionary, IResourceProvider};
    if let Some(dictionary) = value.downcast_ref::<Rc<dyn IResourceDictionary>>() {
        return dictionary.as_object().map(|object| object.to_ref());
    }
    if let Some(provider) = value.downcast_ref::<Rc<dyn IResourceProvider>>() {
        return provider.as_object().map(|object| object.to_ref());
    }
    None
}

/// The argument `value` as the Rust type `T` a member declares for it, as
/// the run-time loader passes the value of a node to an instance member: the
/// value in its untyped form ([`to_untyped`]), then the conversion of the
/// member's arguments (`MarkupArguments::next`). A value that does not
/// convert (a null instance) is the load error the loader raises for the
/// argument `index` of `member` (`Type.Member`) at `line`, `position`: an
/// `InvalidCastException`.
pub fn argument<T: Clone + 'static, V: PartialEq + 'static>(
    value: V,
    member: &str,
    index: usize,
    line: i32,
    position: i32,
) -> Result<T, XamlLoadException> {
    let value = to_untyped(Rc::new(value));
    from_markup_value::<T>(&value).ok_or_else(|| {
        let error = MarkupInvokeError::Argument {
            index,
            expected: std::any::type_name::<T>(),
            actual: match &value {
                Some(value) => value.type_name().to_string(),
                None => "null".to_string(),
            },
        };
        at("InvalidCastException", format!("{member}: {error}"), line, position)
    })
}

/// A nullable form (`Option<T>`, `Option<Rc<T>>`) the instance `T` of a member is borrowed
/// from by [`instance`].
pub trait NullableInstance<T> {
    /// The instance, `None` for null.
    fn instance(&self) -> Option<&T>;
}

impl<T> NullableInstance<T> for Option<T> {
    fn instance(&self) -> Option<&T> {
        self.as_ref()
    }
}

impl<T> NullableInstance<T> for Option<Rc<T>> {
    fn instance(&self) -> Option<&T> {
        self.as_deref()
    }
}

/// The instance of an instance member, borrowed from the nullable form `value` holds it in
/// (a nullable collection read from a property), as [`argument`] converts it without a
/// copy: a null instance is the load error the loader raises for the argument 0 of `member`
/// (`Type.Member`) at `line`, `position`, an `InvalidCastException`.
pub fn instance<'a, T: 'static>(
    value: &'a impl NullableInstance<T>,
    member: &str,
    line: i32,
    position: i32,
) -> Result<&'a T, XamlLoadException> {
    value.instance().ok_or_else(|| {
        let error = MarkupInvokeError::Argument { index: 0, expected: std::any::type_name::<T>(), actual: "null".to_string() };
        at("InvalidCastException", format!("{member}: {error}"), line, position)
    })
}

/// `value` as the value of a member that declares the Rust type `T`, through
/// the assignability casts of the untyped value conversions (a registered
/// cast, a nullable form): the conversion the run-time loader applies to the
/// argument (`to_exact` of the value as the loader holds it,
/// [`into_markup_value`]). The emitter writes it only where no static
/// conversion states the cast and [`ValueTypes::is_assignable`] holds for the
/// two types, which is when the cast succeeds; a value it does not convert is
/// the `InvalidCastException` of the loader at `line`, `position`.
pub fn cast<T: Clone + 'static, V: PartialEq + 'static>(value: V, line: i32, position: i32) -> Result<T, XamlLoadException> {
    let target = ValueType::of::<T>();
    let converted = match into_markup_value(value) {
        Some(boxed) if boxed.value_type_id() == target.id() => Some(boxed),
        Some(boxed) => ValueTypes::try_cast(&boxed, target),
        None => ValueTypes::try_convert(None, target).flatten(),
    };
    match converted.as_ref().and_then(|converted| converted.downcast_ref::<T>()) {
        Some(value) => Ok(value.clone()),
        None => Err(at("InvalidCastException", format!("Unable to cast the value to {}.", target.name()), line, position)),
    }
}

#[cfg(test)]
mod tests {
    use super::uri_equals;

    /// Not from upstream: the comparison of `string.Equals(.., StringComparison.OrdinalIgnoreCase)`.
    #[test]
    fn uris_are_compared_as_ordinal_ignore_case() {
        let root = "ferres://App/";
        assert!(uri_equals("ferres://App/Views/Main.xaml", root, "Views/Main.xaml"));
        assert!(uri_equals("FERRES://APP/VIEWS/MAIN.XAML", root, "Views/Main.xaml"));
        // Not only ASCII: every character by its simple upper-case mapping.
        assert!(uri_equals("ferres://App/Caf\u{c9}.xaml", root, "Caf\u{e9}.xaml"));
        assert!(uri_equals("ferres://App/\u{3a3}.xaml", root, "\u{3c3}.xaml"));
        // A mapping to several characters is not a simple mapping.
        assert!(!uri_equals("ferres://App/SS.xaml", root, "\u{df}.xaml"));
        assert!(!uri_equals("ferres://App/Views/Main.xaml", root, "Views/Main.xam"));
        assert!(!uri_equals("ferres://App/Views/Main.xam", root, "Views/Main.xaml"));
        assert!(!uri_equals("ferres://Other/Views/Main.xaml", root, "Views/Main.xaml"));
    }
}
