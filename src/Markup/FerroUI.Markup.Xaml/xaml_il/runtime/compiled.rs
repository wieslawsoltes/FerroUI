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

use std::fmt::Display;
use std::rc::Rc;

use ferroui_base::controls::{INameScope, NameScope, NameScopeError, NameScopeRef};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{from_markup_value, into_markup_value, IServiceProvider, MarkupInvokeError, MarkupValue};
use ferroui_base::{BoxedValue, FerroObject, Ref, StyledElement, TypeInfo};

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

/// The type name of the exception a member call of generated code that
/// fails with an error of its own is reported as: the run-time loader
/// reports the failure of a member it invokes (`EndInit`, a setter) as the
/// exception that wraps the exception of the member.
pub const TARGET_INVOCATION_EXCEPTION: &str = "TargetInvocationException";

/// The name scope field of the context of a document
/// (`FerroXamlIlContextNameScopeField`): the name scope of the parent
/// service provider, `None` when it has none.
pub fn name_scope_of(parent: Option<&Rc<dyn IServiceProvider>>) -> Option<Rc<dyn INameScope>> {
    parent.and_then(|parent| parent.get_name_scope())
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

/// `value` as the value of a member that declares the Rust type `T`, through
/// the assignability casts of the untyped value conversions (an interface
/// handle, a registered cast, a nullable form): the conversion the run-time
/// loader applies to the argument (`to_exact` of the value as the loader
/// holds it, [`into_markup_value`]). The emitter writes it only where
/// [`ValueTypes::is_assignable`] holds for the two types, which is exactly
/// when the cast succeeds.
pub fn cast<T: Clone + 'static, V: PartialEq + 'static>(value: V) -> T {
    let target = ValueType::of::<T>();
    let converted = match into_markup_value(value) {
        Some(boxed) if boxed.value_type_id() == target.id() => Some(boxed),
        Some(boxed) => ValueTypes::try_cast(&boxed, target),
        None => ValueTypes::try_convert(None, target).flatten(),
    };
    match converted.as_ref().and_then(|converted| converted.downcast_ref::<T>()) {
        Some(value) => value.clone(),
        None => panic!("{} is not assignable from the value (the emitter checked that it is)", target.name()),
    }
}
