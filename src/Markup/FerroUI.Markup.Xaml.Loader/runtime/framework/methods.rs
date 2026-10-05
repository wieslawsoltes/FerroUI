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
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlCustomAttribute, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlType,
};

use crate::compiler_extensions::transformers::{
    FerroAttachedInstancePropertyGetterMethod, OptionsMarkupExtensionMethod,
};
use crate::compiler_extensions::IXamlDocumentTypeBuilderProvider;
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
            return Some(factory.invoke(arguments));
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

/// `XamlIlRuntimeHelpers.DeferredTransformationFactoryV3<T>`: the deferred
/// content customisation of the language as a generic method definition.
///
/// Metadata declares no generic methods, so the method the language found
/// by name is wrapped: [`make_generic_method`](IXamlMethod::make_generic_method)
/// records the type argument, and the call creates the deferred content
/// with it as the result type from the build function of the interpreter.
pub struct DeferredTransformationFactoryMethod {
    inner: Rc<dyn IXamlMethod>,
    type_argument: Option<Rc<dyn IXamlType>>,
}

impl DeferredTransformationFactoryMethod {
    pub fn new(inner: Rc<dyn IXamlMethod>) -> Rc<dyn IXamlMethod> {
        Rc::new(Self { inner, type_argument: None })
    }

    fn invoke(&self, arguments: &[MarkupValue]) -> XamlResult<MarkupValue> {
        let factory = arguments.first().and_then(from_markup_value::<DeferredContentFactory>).ok_or_else(|| {
            runtime_error("InvalidCastException", "The deferred content factory was not given a build function", &NoLineInfo)
        })?;
        let provider =
            arguments.get(1).and_then(from_markup_value::<Rc<dyn IServiceProvider>>).ok_or_else(|| {
                runtime_error("InvalidCastException", "The deferred content factory was not given a service provider", &NoLineInfo)
            })?;
        let result_type = match &self.type_argument {
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

impl IXamlMember for DeferredTransformationFactoryMethod {
    fn name(&self) -> String {
        self.inner.name()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.inner.declaring_type()
    }
}

impl IXamlMethod for DeferredTransformationFactoryMethod {
    fn is_public(&self) -> bool {
        self.inner.is_public()
    }
    fn is_private(&self) -> bool {
        self.inner.is_private()
    }
    fn is_family(&self) -> bool {
        self.inner.is_family()
    }
    fn is_static(&self) -> bool {
        true
    }
    fn contains_generic_parameters(&self) -> bool {
        self.type_argument.is_none()
    }
    fn is_generic_method(&self) -> bool {
        true
    }
    fn is_generic_method_definition(&self) -> bool {
        self.type_argument.is_none()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.inner.return_type()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.inner.parameters()
    }
    fn make_generic_method(&self, type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlMethod>> {
        match type_arguments {
            [argument] => {
                Ok(Rc::new(Self { inner: self.inner.clone(), type_argument: Some(argument.clone()) }))
            }
            _ => Err(XamlError::argument(format!(
                "{} takes one type argument, {} were given",
                self.inner.name(),
                type_arguments.len()
            ))),
        }
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.inner.custom_attributes()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        self.inner.get_parameter_info(index)
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.type_argument.iter().cloned().collect()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(other) => {
                self.inner.equals(&*other.inner)
                    && match (&self.type_argument, &other.type_argument) {
                        (None, None) => true,
                        (Some(a), Some(b)) => a.equals(&**b),
                        _ => false,
                    }
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        self.inner.get_hash_code()
    }
    fn as_any(&self) -> &dyn Any {
        self
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

    /// The transformed root node of the document and the configuration it was
    /// transformed with: what the emitter of Rust source (the second back end)
    /// reads, with what the document adds to its contexts. `None` until the
    /// group has been transformed.
    #[cfg(any(feature = "emitter", test))]
    pub(crate) fn transformed_root(
        &self,
    ) -> Option<(Rc<dyn IXamlAstNode>, Rc<xamlx::transform::TransformerConfiguration>, Rc<RuntimeDocument>)> {
        let body = self.body.borrow();
        let body = body.as_ref()?;
        Some((body.root.clone(), body.interpreter.configuration.clone(), body.document.clone()))
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

struct DocumentMethodSignature {
    name: &'static str,
    document_name: String,
    declaring_type: Rc<dyn IXamlType>,
    return_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
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

macro_rules! document_method {
    ($type_:ident) => {
        impl IXamlMember for $type_ {
            fn name(&self) -> String {
                self.signature.name.to_string()
            }
            fn declaring_type(&self) -> Rc<dyn IXamlType> {
                self.signature.declaring_type.clone()
            }
        }

        impl IXamlMethod for $type_ {
            fn is_public(&self) -> bool {
                true
            }
            fn is_private(&self) -> bool {
                false
            }
            fn is_family(&self) -> bool {
                false
            }
            fn is_static(&self) -> bool {
                true
            }
            fn contains_generic_parameters(&self) -> bool {
                false
            }
            fn is_generic_method(&self) -> bool {
                false
            }
            fn is_generic_method_definition(&self) -> bool {
                false
            }
            fn return_type(&self) -> Rc<dyn IXamlType> {
                self.signature.return_type.clone()
            }
            fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
                self.signature.parameters.clone()
            }
            fn make_generic_method(&self, _type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlMethod>> {
                Err(XamlError::invalid_operation(format!(
                    "{} of document {} is not a generic method definition",
                    self.signature.name, self.signature.document_name
                )))
            }
            fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
                Vec::new()
            }
            fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
                match self.signature.parameters.get(index) {
                    Some(parameter) => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
                    None => Err(XamlError::internal(
                        "ArgumentOutOfRangeException",
                        format!("Method {} doesn't have a parameter {index}", self.signature.name),
                    )),
                }
            }
            fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
                Vec::new()
            }
            fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
                Vec::new()
            }
            fn equals(&self, other: &dyn IXamlMethod) -> bool {
                other.as_any().downcast_ref::<$type_>().is_some_and(|other| std::ptr::eq(self, other))
            }
            fn get_hash_code(&self) -> u64 {
                self as *const Self as usize as u64
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
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
