//! The methods of the framework language that are not methods of the
//! run-time type system: the pseudo methods with a custom call emitter
//! (`FerroAttachedInstancePropertyGetterMethod`,
//! `OptionsMarkupExtensionMethod`), the deferred content factory of the
//! runtime helpers, and the build and populate methods of the documents of
//! a group.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupValue};
use ferroui_base::BoxedValue;
use ferroui_markup_xaml::xaml_il::runtime::{DeferredContentBuilder, XamlIlRuntimeHelpers};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::{IXamlAstNode, IXamlLineInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlMethod, IXamlType};

use crate::compiler_extensions::transformers::{
    FerroAttachedInstancePropertyGetterMethod, OptionsMarkupExtensionMethod,
};
use crate::back_end::{document_method, DocumentMethodSignature};
use crate::compiler_extensions::IXamlDocumentTypeBuilderProvider;

pub use crate::back_end::DeferredTransformationFactoryMethod;
use crate::runtime::interpreter::{
    constant_value, runtime_error, EvalContext, IXamlMethodEvaluator, Interpreter, RuntimeDocument,
};
use crate::runtime::type_system::{DeferredContentFactory, RuntimeType};

use super::helpers::{evaluate_as_own_type, load_property, object_of, property_value};

struct NoLineInfo;

impl IXamlLineInfo for NoLineInfo {
    fn line(&self) -> i32 {
        0
    }
    fn position(&self) -> i32 {
        0
    }
    fn set_line(&self, _value: i32) {}
    fn set_position(&self, _value: i32) {}
}

/// Calls the methods of the framework language that have no invoker in the
/// run-time type system. See
/// [`FRAMEWORK_EVALUATORS`](super::FRAMEWORK_EVALUATORS) for the list.
pub struct FrameworkMethodEvaluator;

impl IXamlMethodEvaluator for FrameworkMethodEvaluator {
    fn invoke(
        &self,
        method: &Rc<dyn IXamlMethod>,
        context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>> {
        let any = method.as_any();
        if let Some(runtime) = any.downcast_ref::<crate::runtime::type_system::RuntimeMethod>() {
            if arguments.len() == 2 && method.name() == "set_Value" && method.declaring_type().is("FerroUI.Styling", "Setter") {
                return Some(set_setter_value(runtime, method, context, arguments));
            }
            return None;
        }
        // `(PropertyType) target.GetValue(Field)`.
        if let Some(getter) = any.downcast_ref::<FerroAttachedInstancePropertyGetterMethod>() {
            return Some((|| {
                let target = arguments.first().cloned().flatten();
                let object = object_of(&target, &NoLineInfo)?;
                let property = load_property(&getter.parent.field, &NoLineInfo)?;
                // The method the IL calls must exist.
                let _ = getter.parent.get_value_method()?;
                Ok(property_value(&object, property))
            })());
        }
        if let Some(options) = any.downcast_ref::<OptionsMarkupExtensionMethod>() {
            return Some(options_markup_extension(method, options, context, arguments));
        }
        if let Some(factory) = any.downcast_ref::<DeferredTransformationFactoryMethod>() {
            return Some(invoke_deferred_transformation_factory(factory, arguments));
        }
        if let Some(build) = any.downcast_ref::<DocumentBuildMethod>() {
            return Some(build.invoke(arguments));
        }
        if let Some(populate) = any.downcast_ref::<DocumentPopulateMethod>() {
            return Some(populate.invoke(arguments));
        }
        None
    }
}

/// The first branch whose condition holds provides the value; otherwise the
/// default node, or the default value of the return type. Only the option
/// nodes of the tested branches and the value node of the selected branch
/// are evaluated.
fn options_markup_extension(
    method: &Rc<dyn IXamlMethod>,
    options: &OptionsMarkupExtensionMethod,
    context: &mut EvalContext<'_>,
    arguments: &[MarkupValue],
) -> XamlResult<MarkupValue> {
    // The context argument (if any) is discarded: the method reloads the context where it needs it.
    let instance = arguments.first().cloned().flatten();
    let container = &options.extension_node_container;
    let return_type = method.return_type();
    for branch in container.branches() {
        let condition = &branch.condition_method;
        let mut condition_arguments: Vec<MarkupValue> = Vec::with_capacity(3);
        if !condition.is_static() {
            condition_arguments.push(instance.clone());
        }
        if branch.has_context() {
            let service_provider = context.configuration().type_mappings.service_provider()?;
            condition_arguments.push(context.context_value(&*service_provider)?);
        }
        let option = branch.option();
        condition_arguments.push(evaluate_as_own_type(&option, context)?);
        let result = context.call_method(condition, &condition_arguments, &*branch)?;
        if from_markup_value::<bool>(&result) == Some(true) {
            let value = branch.value();
            let own_type = value.type_().get_clr_type()?;
            let result = evaluate_as_own_type(&value, context)?;
            return context.convert(&*value, result, &own_type, &return_type);
        }
    }
    match container.default_node() {
        Some(default_node) => {
            let own_type = default_node.type_().get_clr_type()?;
            let result = evaluate_as_own_type(&default_node, context)?;
            context.convert(&*default_node, result, &own_type, &return_type)
        }
        None => default_value(&return_type),
    }
}

/// `default(T)`: null for a reference or nullable type, the zero value of a
/// value type.
fn default_value(type_: &Rc<dyn IXamlType>) -> XamlResult<MarkupValue> {
    if !type_.is_value_type() || type_.is_nullable() {
        return Ok(None);
    }
    if type_.is_enum() || type_.namespace().as_deref() == Some("System") {
        if let Ok(value) = constant_value(type_, &xamlx::type_system::XamlValue::Int32(0), &NoLineInfo) {
            return Ok(value);
        }
    }
    // A value type states its default through its parameterless constructor.
    if let Some(runtime) = type_.as_any().downcast_ref::<RuntimeType>() {
        if let Some(constructor) = runtime.runtime_constructors().into_iter().find(|c| {
            xamlx::type_system::IXamlConstructor::parameters(&**c).is_empty()
        }) {
            return constructor
                .invoke(&[])
                .map_err(|e| runtime_error("TargetInvocationException", e.to_string(), &NoLineInfo));
        }
    }
    Err(XamlError::load_exception(
        format!("Unable to create the default value of {}", type_.get_full_name()),
        None,
    ))
}

// --- deferred content ---------------------------------------------------------

/// The call of the deferred content customisation of the language
/// ([`DeferredTransformationFactoryMethod`]): creates the deferred content with the type
/// argument of the method as the result type from the build function of the interpreter.
fn invoke_deferred_transformation_factory(method: &DeferredTransformationFactoryMethod, arguments: &[MarkupValue]) -> XamlResult<MarkupValue> {
    let factory = arguments.first().and_then(from_markup_value::<DeferredContentFactory>).ok_or_else(|| {
        runtime_error("InvalidCastException", "The deferred content factory was not given a build function", &NoLineInfo)
    })?;
    let provider =
        arguments.get(1).and_then(from_markup_value::<Rc<dyn IServiceProvider>>).ok_or_else(|| {
            runtime_error("InvalidCastException", "The deferred content factory was not given a service provider", &NoLineInfo)
        })?;
    let result_type = match method.type_argument() {
        None => ValueType::object(),
        Some(argument) => argument
            .as_any()
            .downcast_ref::<RuntimeType>()
            .and_then(RuntimeType::handle)
            .unwrap_or_else(ValueType::object),
    };
    // A failure while the content is built is the load error of the failing node:
    // returned by the fallible entry points of the runtime library, raised by the
    // infallible ones (a template instantiated later by a control).
    let builder = DeferredContentBuilder::try_new(move |service_provider| {
        factory.invoke(Some(service_provider.clone())).map_err(load_exception)
    });
    let content = XamlIlRuntimeHelpers::try_deferred_transformation_factory_for(result_type, builder, &provider)
        .map_err(|e| runtime_error("InvalidOperationException", e.message().to_string(), &NoLineInfo))?;
    let content: BoxedValue = Rc::new(content);
    Ok(Some(crate::runtime::type_system::normalize_object(content)))
}

/// The failure of a member the loaded document called (a constructor, a setter, an `Add`
/// method, a markup extension): the exception the member throws in the managed original,
/// which leaves the load as it is and is not an error of the compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XamlMemberException {
    message: String,
    line_number: Option<i32>,
    line_position: Option<i32>,
}

impl XamlMemberException {
    /// The message of the member.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The line of the node that was being evaluated.
    pub fn line_number(&self) -> Option<i32> {
        self.line_number
    }

    /// The position of the node that was being evaluated.
    pub fn line_position(&self) -> Option<i32> {
        self.line_position
    }
}

impl std::fmt::Display for XamlMemberException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for XamlMemberException {}

/// A failed load as the load exception of the runtime library: the message with the
/// position, and as the inner error either the error of the compiler or, when a member the
/// document called failed, the failure of that member ([`XamlMemberException`]).
pub(crate) fn load_exception(error: XamlError) -> ferroui_markup_xaml::XamlLoadException {
    if error.is_derived_type("TargetInvocationException") {
        let inner = XamlMemberException {
            message: error.message(),
            line_number: error.line_number(),
            line_position: error.line_position(),
        };
        return ferroui_markup_xaml::XamlLoadException::with_inner(describe(&error), inner);
    }
    ferroui_markup_xaml::XamlLoadException::with_inner(describe(&error), error)
}

/// The message of a load error with its position, as the exception of the
/// managed original prints it.
pub(crate) fn describe(error: &XamlError) -> String {
    match (error.line_number(), error.line_position()) {
        (Some(line), Some(position)) => format!("{} (line {line} position {position})", error.message()),
        _ => error.message(),
    }
}

// --- documents ----------------------------------------------------------------

/// What the interpreter needs to run a document of a group: filled in once
/// the group has been transformed.
pub struct DocumentBody {
    pub interpreter: Rc<Interpreter>,
    pub root: Rc<dyn IXamlAstNode>,
    pub document: Rc<RuntimeDocument>,
}

/// The output of the back end for one document: the handles of its
/// populate method and (for a document that can be instantiated on its own)
/// its build method, which the include group transformer puts into method
/// call nodes. The counterpart of the type builder of a document of the
/// managed original.
pub struct RuntimeDocumentTypeBuilderProvider {
    body: Rc<RefCell<Option<DocumentBody>>>,
    populate: Rc<dyn IXamlMethod>,
    build: Option<Rc<dyn IXamlMethod>>,
}

impl RuntimeDocumentTypeBuilderProvider {
    /// `declaring_type` stands for the generated type the methods belong to;
    /// `root_type` is the type of the root object of the document.
    pub fn new(
        name: &str,
        root_type: Rc<dyn IXamlType>,
        service_provider: Rc<dyn IXamlType>,
        void: Rc<dyn IXamlType>,
        has_build_method: bool,
    ) -> Rc<Self> {
        let body: Rc<RefCell<Option<DocumentBody>>> = Rc::new(RefCell::new(None));
        let populate: Rc<dyn IXamlMethod> = Rc::new(DocumentPopulateMethod {
            signature: DocumentMethodSignature {
                name: crate::compiler_extensions::FerroXamlIlCompiler::POPULATE_NAME,
                document_name: name.to_string(),
                declaring_type: root_type.clone(),
                return_type: void,
                parameters: vec![service_provider.clone(), root_type.clone()],
            },
            body: body.clone(),
        });
        let build: Option<Rc<dyn IXamlMethod>> = has_build_method.then(|| {
            Rc::new(DocumentBuildMethod {
                signature: DocumentMethodSignature {
                    name: crate::compiler_extensions::FerroXamlIlCompiler::BUILD_NAME,
                    document_name: name.to_string(),
                    declaring_type: root_type.clone(),
                    return_type: root_type,
                    parameters: vec![service_provider],
                },
                body: body.clone(),
            }) as Rc<dyn IXamlMethod>
        });
        Rc::new(Self { body, populate, build })
    }

    /// Supplies the transformed document the methods run.
    pub fn set_body(&self, body: DocumentBody) {
        *self.body.borrow_mut() = Some(body);
    }

    /// `Populate(serviceProvider, target)`.
    pub fn populate(&self, service_provider: Option<Rc<dyn IServiceProvider>>, target: &MarkupValue) -> XamlResult<()> {
        run_populate(&self.body, service_provider, target)
    }

    /// `Build(serviceProvider)`.
    pub fn build(&self, service_provider: Option<Rc<dyn IServiceProvider>>) -> XamlResult<MarkupValue> {
        run_build(&self.body, service_provider)
    }
}

impl IXamlDocumentTypeBuilderProvider for RuntimeDocumentTypeBuilderProvider {
    fn populate_method(&self) -> Rc<dyn IXamlMethod> {
        self.populate.clone()
    }
    fn build_method(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.build.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn body_missing() -> XamlError {
    XamlError::invalid_operation("The document of a build or populate method has not been compiled")
}

fn run_populate(
    body: &RefCell<Option<DocumentBody>>,
    service_provider: Option<Rc<dyn IServiceProvider>>,
    target: &MarkupValue,
) -> XamlResult<()> {
    let (interpreter, root, document) = {
        let body = body.borrow();
        let body = body.as_ref().ok_or_else(body_missing)?;
        (body.interpreter.clone(), body.root.clone(), body.document.clone())
    };
    interpreter.populate(&root, service_provider, target, &document)
}

fn run_build(
    body: &RefCell<Option<DocumentBody>>,
    service_provider: Option<Rc<dyn IServiceProvider>>,
) -> XamlResult<MarkupValue> {
    let (interpreter, root, document) = {
        let body = body.borrow();
        let body = body.as_ref().ok_or_else(body_missing)?;
        (body.interpreter.clone(), body.root.clone(), body.document.clone())
    };
    interpreter.build(&root, service_provider, &document)
}

fn service_provider_argument(arguments: &[MarkupValue]) -> Option<Rc<dyn IServiceProvider>> {
    arguments.first().and_then(from_markup_value::<Rc<dyn IServiceProvider>>)
}

/// The populate method of a document: `void Populate(IServiceProvider, T)`.
pub struct DocumentPopulateMethod {
    signature: DocumentMethodSignature,
    body: Rc<RefCell<Option<DocumentBody>>>,
}

impl DocumentPopulateMethod {
    fn invoke(&self, arguments: &[MarkupValue]) -> XamlResult<MarkupValue> {
        // The method keeps its document alive, as a generated method keeps its code:
        // deferred content of another document may call it long after the load.
        let body = &self.body;
        let target = arguments.get(1).cloned().flatten();
        run_populate(body, service_provider_argument(arguments), &target)?;
        Ok(None)
    }
}

/// The build method of a document: `T Build(IServiceProvider)`.
pub struct DocumentBuildMethod {
    signature: DocumentMethodSignature,
    body: Rc<RefCell<Option<DocumentBody>>>,
}

impl DocumentBuildMethod {
    fn invoke(&self, arguments: &[MarkupValue]) -> XamlResult<MarkupValue> {
        // The method keeps its document alive, as a generated method keeps its code:
        // deferred content of another document may call it long after the load.
        let body = &self.body;
        run_build(body, service_provider_argument(arguments))
    }
}

document_method!(DocumentPopulateMethod);
document_method!(DocumentBuildMethod);

/// `Setter.set_Value(object)`: the setter holds a value of exactly the value
/// type of the registered property it targets (in the managed original the
/// object assigned already is an instance of that type; here the untyped
/// value is brought to the property's Rust type with the assignability
/// casts). The property is read from the setter; a value that is not of the
/// type of the property (a binding, the unset marker, a template the setter
/// instantiates) is passed as it is.
fn set_setter_value(
    runtime: &crate::runtime::type_system::RuntimeMethod,
    method: &Rc<dyn IXamlMethod>,
    context: &mut EvalContext<'_>,
    arguments: &[MarkupValue],
) -> XamlResult<MarkupValue> {
    let mut arguments = arguments.to_vec();
    if arguments[1].is_some() {
        let getter = method
            .declaring_type()
            .get_all_properties()
            .into_iter()
            .find(|p| p.name() == "Property")
            .and_then(|p| p.getter());
        let property = match getter {
            Some(getter) => context.call_method(&getter, &arguments[..1], &NoLineInfo).ok(),
            None => None,
        };
        let property = property.and_then(|p| from_markup_value::<&'static ferroui_base::FerroProperty>(&p));
        if let Some(property) = property {
            if let Ok(exact) = super::helpers::exact_property_value(property, &arguments[1], &NoLineInfo) {
                arguments[1] = Some(exact);
            }
        }
    }
    runtime.invoke(&arguments).map_err(|e| runtime_error("TargetInvocationException", format!("Setter.Value: {e}"), &NoLineInfo))
}
