//! The interpreter: the run-time replacement of the IL compiler of the
//! managed original (`IL/XamlIlCompiler.cs`, `Emit/XamlEmitContext.cs`,
//! `IL/ILEmitHelpers.cs`).
//!
//! Where the IL compiler emits a `Build` and a `Populate` method for a
//! document and runs them, [`Interpreter::build`] and
//! [`Interpreter::populate`] evaluate the transformed AST directly. A node
//! evaluates to a value (value nodes) or manipulates a target value
//! (manipulation nodes; the target is the value the IL leaves on the
//! evaluation stack).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::metadata::{IServiceProvider, MarkupInvokeError, MarkupValue};
use ferroui_base::utilities::Uri;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstVisitor, IXamlAstValueNode, IXamlLineInfo, IXamlPropertySetter,
    IXamlWrappedMethod, XamlAstClrProperty, XamlAstExtensions, XamlAstNewClrObjectNode, XamlAstNodeExtensions,
    XamlDocument, XamlMethodWithCasts,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::extensions::{query_node_interface, query_property_setter_interface, query_wrapped_method_interface};
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::{IXamlConstructor, IXamlMethod, IXamlType, XamlPseudoType};

use crate::runtime::type_system::{RuntimeConstructor, RuntimeMethod, RuntimeTypeSystem};

use super::evaluators;
use super::runtime_context::{
    namespace_info_static_provider, IRuntimeContextServices, IStaticServiceProvider, RuntimeContext,
    RuntimeContextDefinition, XmlNamespaceInfoProvider,
};
use super::services::DefaultRuntimeContextServices;

/// What evaluating a node yields: the equivalent of the emit result of the
/// IL back end. A node that leaves no value (a manipulation, an imperative
/// node) has no return type.
pub struct EvalResult {
    /// The static type of the value; `None` for a node that yields nothing.
    pub return_type: Option<Rc<dyn IXamlType>>,
    pub value: MarkupValue,
}

impl EvalResult {
    /// The result of a node that yields nothing.
    pub fn void() -> Self {
        Self { return_type: None, value: None }
    }

    /// The result of a value node.
    pub fn value(return_type: Rc<dyn IXamlType>, value: MarkupValue) -> Self {
        Self { return_type: Some(return_type), value }
    }
}

/// Evaluates nodes: the equivalent of a node emitter of the IL back end.
/// `None` means the node is not one this evaluator handles.
///
/// A manipulation node manipulates [`EvalContext::target`].
pub trait IXamlNodeEvaluator {
    fn evaluate(&self, node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> Option<XamlResult<EvalResult>>;
}

/// A node that evaluates itself; exposed by a node through
/// `IXamlAstNode::query_interface` (the equivalent of a node that emits
/// itself).
pub trait IXamlAstEvaluableNode {
    fn evaluate(&self, context: &mut EvalContext<'_>) -> XamlResult<EvalResult>;
}

/// Performs property setters: the equivalent of a property setter emitter.
/// `None` means the setter is not one this evaluator handles.
pub trait IXamlSetterEvaluator {
    /// Performs the setter on `target` with already evaluated arguments.
    fn evaluate(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        context: &mut EvalContext<'_>,
        target: &MarkupValue,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<()>>;

    /// Performs the setter evaluating the arguments itself, for setters
    /// that interleave the evaluation of the arguments with their own work
    /// (the optimised setter path of the IL back end). Used when the setter
    /// is the only possible one.
    fn evaluate_with_arguments(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        context: &mut EvalContext<'_>,
        target: &MarkupValue,
        arguments: &[Rc<dyn IXamlAstValueNode>],
    ) -> Option<XamlResult<()>> {
        let _ = (setter, context, target, arguments);
        None
    }
}

/// A property setter that performs itself; exposed by a setter through
/// `IXamlPropertySetter::query_interface`.
pub trait IXamlEvaluablePropertySetter {
    fn evaluate(&self, context: &mut EvalContext<'_>, target: &MarkupValue, arguments: &[MarkupValue])
        -> XamlResult<()>;
}

/// Calls methods that are not methods of the run-time type system: the
/// equivalent of a method with a custom call emitter. `arguments` holds
/// the instance (for an instance method) followed by the arguments. `None`
/// means the method is not one this evaluator handles.
pub trait IXamlMethodEvaluator {
    fn invoke(
        &self,
        method: &Rc<dyn IXamlMethod>,
        context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>>;
}

/// Calls constructors that are not constructors of the run-time type system
/// (a back end that creates objects of types it defines itself, a test that
/// records the objects a document creates). `None` means the constructor is
/// not one this evaluator handles.
pub trait IXamlConstructorEvaluator {
    fn invoke(
        &self,
        constructor: &Rc<dyn IXamlConstructor>,
        context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>>;
}

/// Calls wrapped methods: the equivalent of a wrapped method emitter.
pub trait IXamlWrappedMethodEvaluator {
    fn invoke(
        &self,
        method: &Rc<dyn IXamlWrappedMethod>,
        context: &mut EvalContext<'_>,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<MarkupValue>>;
}

/// A wrapped method that calls itself; exposed by a wrapped method through
/// `IXamlWrappedMethod::query_interface`.
pub trait IXamlEvaluableWrappedMethod {
    fn invoke(&self, context: &mut EvalContext<'_>, arguments: &[MarkupValue]) -> XamlResult<MarkupValue>;
}

/// Evaluates the provide-value target property descriptor of a property
/// (the provide-value target property emitter of the emit mappings).
/// `Ok(None)` means the name of the property is the descriptor.
pub type ProvideValueTargetPropertyEvaluator =
    Rc<dyn Fn(&mut EvalContext<'_>, &Rc<XamlAstClrProperty>) -> XamlResult<Option<MarkupValue>>>;

/// Runs when a context is created.
pub type RuntimeContextCallback = Rc<dyn Fn(&Rc<RuntimeContext>) -> XamlResult<()>>;

/// An error raised by evaluated code: the equivalent of an exception of the
/// managed runtime (`kind` is its type name), with the position of the node
/// being evaluated.
pub fn runtime_error(kind: &'static str, message: impl Into<String>, line_info: &dyn IXamlLineInfo) -> XamlError {
    XamlError::load_exception(message, Some(line_info)).with_derived_type_name(kind)
}

fn invoke_error(error: MarkupInvokeError, what: &str, line_info: &dyn IXamlLineInfo) -> XamlError {
    match error {
        MarkupInvokeError::Argument { .. } => {
            runtime_error("InvalidCastException", format!("{what}: {error}"), line_info)
        }
        MarkupInvokeError::ArgumentCount { .. } => {
            runtime_error("InvalidOperationException", format!("{what}: {error}"), line_info)
        }
        MarkupInvokeError::Failed(message) => runtime_error("TargetInvocationException", message, line_info),
    }
}

/// The context as the service provider its inner service provider is built
/// on, without keeping the context alive. A context that is gone provides
/// nothing.
struct WeakContextServiceProvider(std::rc::Weak<RuntimeContext>);

impl IServiceProvider for WeakContextServiceProvider {
    fn get_service(&self, service_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        // Only the services below the inner provider: asking the context itself
        // would ask the inner provider again.
        self.0.upgrade()?.get_service_below_inner(service_type)
    }
}

/// What a document adds to the contexts created for it (its base URI and
/// its static providers: the namespace information provider, if the
/// language maps one) and what the interpreter decided once about its
/// nodes. It keeps the nodes of the document alive.
pub struct RuntimeDocument {
    pub base_uri: Option<Uri>,
    pub static_providers: Rc<[Rc<dyn IStaticServiceProvider>]>,
    /// The root node: the decisions below are remembered by node address,
    /// so the nodes must outlive them.
    root: Option<Rc<dyn IXamlAstNode>>,
    parent_stack_caches: RefCell<HashMap<usize, ParentStackCache>>,
    assignment_plans: RefCell<HashMap<usize, Rc<evaluators::AssignmentPlan>>>,
}

impl RuntimeDocument {
    pub fn new(
        base_uri: Option<Uri>,
        static_providers: Vec<Rc<dyn IStaticServiceProvider>>,
        root: Option<Rc<dyn IXamlAstNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base_uri,
            static_providers: Rc::from(static_providers),
            root,
            parent_stack_caches: RefCell::new(HashMap::new()),
            assignment_plans: RefCell::new(HashMap::new()),
        })
    }

    /// A document without base URI and static providers.
    pub fn empty() -> Rc<Self> {
        Self::new(None, Vec::new(), None)
    }

    /// The root node of the document.
    pub fn root(&self) -> Option<&Rc<dyn IXamlAstNode>> {
        self.root.as_ref()
    }

    fn parent_stack_cache(&self, scope: usize) -> ParentStackCache {
        self.parent_stack_caches.borrow_mut().entry(scope).or_default().clone()
    }

    pub(crate) fn assignment_plan(
        &self,
        node: usize,
        create: impl FnOnce() -> XamlResult<evaluators::AssignmentPlan>,
    ) -> XamlResult<Rc<evaluators::AssignmentPlan>> {
        if let Some(plan) = self.assignment_plans.borrow().get(&node) {
            return Ok(plan.clone());
        }
        let plan = Rc::new(create()?);
        self.assignment_plans.borrow_mut().insert(node, plan.clone());
        Ok(plan)
    }
}

type ParentStackCache = Rc<RefCell<HashMap<usize, bool>>>;

/// The interpreter of the transformed AST.
///
/// Framework-specific nodes, property setters and methods plug in through
/// the evaluator lists, which are consulted in order before the built-in
/// evaluation of the nodes that evaluate themselves upstream.
pub struct Interpreter {
    pub configuration: Rc<TransformerConfiguration>,
    pub type_system: Rc<RuntimeTypeSystem>,
    /// Starts with the evaluator of the standard nodes (the default
    /// emitters of the IL compiler).
    pub node_evaluators: Vec<Rc<dyn IXamlNodeEvaluator>>,
    pub setter_evaluators: Vec<Rc<dyn IXamlSetterEvaluator>>,
    pub method_evaluators: Vec<Rc<dyn IXamlMethodEvaluator>>,
    pub constructor_evaluators: Vec<Rc<dyn IXamlConstructorEvaluator>>,
    pub wrapped_method_evaluators: Vec<Rc<dyn IXamlWrappedMethodEvaluator>>,
    /// Maps the context onto the service contracts.
    pub services: Rc<dyn IRuntimeContextServices>,
    pub provide_value_target_property_evaluator: Option<ProvideValueTargetPropertyEvaluator>,
    /// Run inside the constructor of a context, before its inner service
    /// provider is created (the context type builder callback).
    pub context_initializers: Vec<RuntimeContextCallback>,
    /// Run on a created context (the context factory callback).
    pub context_factory_callbacks: Vec<RuntimeContextCallback>,
    definition: RuntimeContextDefinition,
}

impl Interpreter {
    /// An interpreter with the standard evaluators and the default service
    /// contracts. Add evaluators to the lists, then share the interpreter
    /// with `Rc::new`.
    pub fn new(configuration: Rc<TransformerConfiguration>, type_system: Rc<RuntimeTypeSystem>) -> Self {
        let definition = RuntimeContextDefinition::new(&configuration);
        Self {
            configuration,
            type_system,
            node_evaluators: vec![Rc::new(evaluators::StandardNodeEvaluator)],
            setter_evaluators: Vec::new(),
            method_evaluators: Vec::new(),
            constructor_evaluators: Vec::new(),
            wrapped_method_evaluators: Vec::new(),
            services: Rc::new(DefaultRuntimeContextServices),
            provide_value_target_property_evaluator: None,
            context_initializers: Vec::new(),
            context_factory_callbacks: Vec::new(),
            definition,
        }
    }

    /// Which services the contexts of this interpreter implement.
    pub fn context_definition(&self) -> RuntimeContextDefinition {
        self.definition
    }

    /// What a parsed document adds to its contexts: the base URI and, if
    /// the language maps a namespace information provider, the namespace
    /// information of the document.
    pub fn document(&self, document: &XamlDocument, base_uri: Option<&str>) -> XamlResult<Rc<RuntimeDocument>> {
        let base_uri = match base_uri {
            Some(base_uri) => Some(Uri::absolute(base_uri).map_err(|e| {
                XamlError::internal("UriFormatException", format!("Invalid base URI '{base_uri}': {e}"))
            })?),
            None => None,
        };
        let mut static_providers: Vec<Rc<dyn IStaticServiceProvider>> = Vec::new();
        if self.definition.xml_namespace_info_provider {
            static_providers.push(namespace_info_static_provider(
                XmlNamespaceInfoProvider::new(self.configuration.clone(), document),
                self.services.clone(),
            ));
        }
        Ok(RuntimeDocument::new(base_uri, static_providers, document.root().ok()))
    }

    /// The context factory: creates the context of a `Build`, `Populate` or
    /// deferred build with `parent` as its parent service provider.
    pub fn create_context(
        self: &Rc<Self>,
        parent: Option<Rc<dyn IServiceProvider>>,
        document: &RuntimeDocument,
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<RuntimeContext>> {
        let context = RuntimeContext::new(
            self.definition,
            self.services.clone(),
            parent,
            document.static_providers.clone(),
            document.base_uri.clone(),
        );
        for initializer in &self.context_initializers {
            initializer(&context)?;
        }
        // Last, so that the own services of the context are ready.
        if let Some(factory) = &self.configuration.type_mappings.inner_service_provider_factory_method {
            let mut eval = EvalContext::new(self, None, RuntimeDocument::empty(), 0);
            // The inner provider is owned by the context and is given the context it is
            // built on: a strong reference would be a cycle nothing collects.
            let provider: Rc<dyn IServiceProvider> = Rc::new(WeakContextServiceProvider(Rc::downgrade(&context)));
            let provider: MarkupValue = Some(Rc::new(provider));
            let inner = eval.call_method(factory, &[provider], line_info)?;
            let inner = match &inner {
                None => None,
                Some(_) => Some(ferroui_base::metadata::from_markup_value::<Rc<dyn IServiceProvider>>(&inner).ok_or_else(
                    || {
                        runtime_error(
                            "InvalidCastException",
                            "The inner service provider factory didn't return a service provider",
                            line_info,
                        )
                    },
                )?),
            };
            context.set_inner_service_provider(inner);
        }
        for callback in &self.context_factory_callbacks {
            callback(&context)?;
        }
        Ok(context)
    }

    fn root_group(
        root: &Rc<dyn IXamlAstNode>,
    ) -> XamlResult<(Rc<dyn IXamlAstValueNode>, Rc<dyn xamlx::ast::IXamlAstManipulationNode>)> {
        let group = root.as_value_with_manipulation_node().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlValueWithManipulationNode'.",
                root.type_name()
            ))
        })?;
        let manipulation = group.manipulation().ok_or_else(|| {
            XamlError::internal("NullReferenceException", "The root node doesn't have a manipulation")
        })?;
        Ok((group.value(), manipulation))
    }

    /// `object Build(IServiceProvider)`: creates the root object of a
    /// transformed document and populates it.
    pub fn build(
        self: &Rc<Self>,
        root: &Rc<dyn IXamlAstNode>,
        parent_service_provider: Option<Rc<dyn IServiceProvider>>,
        document: &Rc<RuntimeDocument>,
    ) -> XamlResult<MarkupValue> {
        let (root_value, _) = Self::root_group(root)?;
        let mut need_context_local = false;
        if let Some(new_object) = root_value.cast::<XamlAstNewClrObjectNode>() {
            let arguments = new_object.arguments.borrow().clone();
            let mut argument_names = Vec::with_capacity(arguments.len());
            for argument in &arguments {
                argument_names.push(argument.type_().get_clr_type()?.get_full_name());
            }
            if let [argument] = arguments.as_slice() {
                need_context_local =
                    argument.type_().get_clr_type()?.equals(&*self.configuration.type_mappings.service_provider()?);
            }
            let parameter_names: Vec<String> =
                new_object.constructor.parameters().iter().map(|p| p.get_full_name()).collect();
            if parameter_names != argument_names {
                return Err(XamlError::invalid_operation(
                    "Cannot compile Build method. Parameters mismatch.\
                     Type needs to have a parameterless ctor or a ctor with a single IServiceProvider argument.\
                     Or x:Arguments directive with matching arguments needs to be set",
                ));
            }
        }
        let context = match need_context_local {
            true => Some(self.create_context(parent_service_provider.clone(), document, &*root_value)?),
            false => None,
        };
        let scope = Rc::as_ptr(&root_value) as *const () as usize;
        let mut eval = EvalContext::new(self, context, document.clone(), scope);
        let root_type = root_value.type_().get_clr_type()?;
        let instance = eval.evaluate(&root_value.as_node(), Some(&root_type))?;
        self.populate(root, parent_service_provider, &instance, document)?;
        Ok(instance)
    }

    /// `void Populate(IServiceProvider, T target)`: sets up an existing
    /// root object from a transformed document.
    pub fn populate(
        self: &Rc<Self>,
        root: &Rc<dyn IXamlAstNode>,
        parent_service_provider: Option<Rc<dyn IServiceProvider>>,
        root_instance: &MarkupValue,
        document: &Rc<RuntimeDocument>,
    ) -> XamlResult<()> {
        let (_, manipulation) = Self::root_group(root)?;
        let context = self.create_context(parent_service_provider, document, &*manipulation)?;
        context.set_root_object(root_instance.clone());
        context.set_intermediate_root_object(root_instance.clone());
        let scope = Rc::as_ptr(&manipulation) as *const () as usize;
        let mut eval = EvalContext::new(self, Some(context), document.clone(), scope);
        eval.manipulate(&manipulation.as_node(), root_instance)
    }
}

struct ParentStackVisitor<'a> {
    cache: &'a RefCell<HashMap<usize, bool>>,
    parents: Vec<usize>,
}

fn node_address(node: &Rc<dyn IXamlAstNode>) -> usize {
    Rc::as_ptr(node) as *const () as usize
}

impl IXamlAstVisitor for ParentStackVisitor<'_> {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let address = node_address(&node);
        let mut cache = self.cache.borrow_mut();
        if cache.contains_key(&address) {
            return Ok(node);
        }
        if node.as_needs_parent_stack().is_some_and(|n| n.needs_parent_stack()) {
            cache.insert(address, true);
            for parent in &self.parents {
                cache.insert(*parent, true);
            }
        } else {
            cache.insert(address, false);
        }
        Ok(node)
    }
    fn push(&mut self, node: Rc<dyn IXamlAstNode>) {
        self.parents.push(node_address(&node));
    }
    fn pop(&mut self) {
        self.parents.pop();
    }
}

/// The state of one evaluation: the equivalent of the emit context of one
/// emitted method (`Build`, `Populate`, or the build method of a deferred
/// content closure).
pub struct EvalContext<'a> {
    interpreter: &'a Rc<Interpreter>,
    runtime_context: Option<Rc<RuntimeContext>>,
    document: Rc<RuntimeDocument>,
    scope: usize,
    parent_nodes: Vec<Rc<dyn IXamlAstNode>>,
    current_node: Option<Rc<dyn IXamlAstNode>>,
    targets: Vec<MarkupValue>,
    locals: Vec<(usize, MarkupValue)>,
    parent_stack_cache: Option<ParentStackCache>,
}

impl<'a> EvalContext<'a> {
    /// A context for evaluating in the scope `scope` (the address of the
    /// node the evaluation starts from). `runtime_context` is the context
    /// local; a scope that has none cannot evaluate nodes that need it.
    pub fn new(
        interpreter: &'a Rc<Interpreter>,
        runtime_context: Option<Rc<RuntimeContext>>,
        document: Rc<RuntimeDocument>,
        scope: usize,
    ) -> Self {
        Self {
            interpreter,
            runtime_context,
            document,
            scope,
            parent_nodes: Vec::new(),
            current_node: None,
            targets: Vec::new(),
            locals: Vec::new(),
            parent_stack_cache: None,
        }
    }

    pub fn interpreter(&self) -> &'a Rc<Interpreter> {
        self.interpreter
    }

    pub fn configuration(&self) -> &Rc<TransformerConfiguration> {
        &self.interpreter.configuration
    }

    pub fn type_system(&self) -> &Rc<RuntimeTypeSystem> {
        &self.interpreter.type_system
    }

    /// The document the evaluated nodes belong to.
    pub fn document(&self) -> &Rc<RuntimeDocument> {
        &self.document
    }

    /// The run-time context of this evaluation (the context local).
    pub fn runtime_context(&self) -> XamlResult<&Rc<RuntimeContext>> {
        self.runtime_context.as_ref().ok_or_else(|| {
            XamlError::invalid_operation("The current emit context doesn't supply a runtime context")
        })
    }

    /// The run-time context as a value of the type `type_`.
    pub fn context_value(&self, type_: &dyn IXamlType) -> XamlResult<MarkupValue> {
        let context = self.runtime_context()?;
        Ok(Some(self.interpreter.services.context_value(context, type_)))
    }

    /// The nodes around the node being evaluated, nearest first.
    pub fn parent_nodes(&self) -> impl Iterator<Item = &Rc<dyn IXamlAstNode>> {
        self.parent_nodes.iter().rev()
    }

    /// The value the manipulation node being evaluated manipulates.
    pub fn target(&self) -> XamlResult<MarkupValue> {
        self.targets.last().cloned().ok_or_else(|| {
            XamlError::invalid_operation("A manipulation node was evaluated without a value to manipulate")
        })
    }

    /// Evaluates a value node and converts the value to `expected_type`
    /// with the conversions the IL back end emits implicitly.
    pub fn evaluate(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        expected_type: Option<&Rc<dyn IXamlType>>,
    ) -> XamlResult<MarkupValue> {
        Ok(self.evaluate_typed(node, expected_type)?.value)
    }

    /// [`evaluate`](Self::evaluate), also returning the type the node
    /// itself reported (before the conversion).
    pub fn evaluate_typed(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        expected_type: Option<&Rc<dyn IXamlType>>,
    ) -> XamlResult<EvalResult> {
        let pushed = match self.current_node.replace(node.clone()) {
            Some(current) => {
                self.parent_nodes.push(current);
                true
            }
            None => false,
        };
        let result = self.evaluate_core(node, expected_type);
        self.current_node = if pushed { self.parent_nodes.pop() } else { None };
        result
    }

    /// Evaluates a manipulation node on `target`.
    pub fn manipulate(&mut self, node: &Rc<dyn IXamlAstNode>, target: &MarkupValue) -> XamlResult<()> {
        self.targets.push(target.clone());
        let result = self.evaluate_typed(node, None);
        self.targets.pop();
        result.map(|_| ())
    }

    /// Evaluates an imperative node.
    pub fn execute(&mut self, node: &Rc<dyn IXamlAstNode>) -> XamlResult<()> {
        self.evaluate_typed(node, None).map(|_| ())
    }

    fn evaluate_core(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        expected_type: Option<&Rc<dyn IXamlType>>,
    ) -> XamlResult<EvalResult> {
        let mut result = self.evaluate_node(node)?;
        match (&result.return_type, expected_type) {
            (None, None) => {}
            (Some(returned), None) => {
                return Err(XamlError::load_exception(
                    format!(
                        "Emit of node {} resulted in {} while caller expected void",
                        node.to_node_string(),
                        returned.get_fqn()
                    ),
                    Some(&**node),
                ));
            }
            (None, Some(expected)) => {
                return Err(XamlError::load_exception(
                    format!(
                        "Emit of node {} resulted in void while caller expected {}",
                        node.to_node_string(),
                        expected.get_fqn()
                    ),
                    Some(&**node),
                ));
            }
            (Some(returned), Some(expected)) => {
                if !returned.equals(&**expected) {
                    let value = result.value.take();
                    result.value = self.convert(&**node, value, returned, expected)?;
                }
            }
        }
        Ok(result)
    }

    fn evaluate_node(&mut self, node: &Rc<dyn IXamlAstNode>) -> XamlResult<EvalResult> {
        let interpreter = self.interpreter;
        for evaluator in &interpreter.node_evaluators {
            if let Some(result) = evaluator.evaluate(node, self) {
                return result;
            }
        }
        if let Some(evaluable) = query_node_interface::<dyn IXamlAstEvaluableNode>(node) {
            return evaluable.evaluate(self);
        }
        if let Some(result) = evaluators::evaluate_intrinsic(node, self) {
            return result;
        }
        Err(XamlError::load_exception(
            format!("Unable to find evaluator for node type: {}", node.type_name()),
            Some(&**node),
        ))
    }

    /// The conversion the IL back end emits when a node yields a value of
    /// the type `from` where `to` is expected (`ILEmitHelpers.EmitConvert`):
    /// null to a reference or nullable type, a value type to its nullable
    /// form, boxing, unboxing and the checked reference cast.
    pub fn convert(
        &self,
        line_info: &dyn IXamlLineInfo,
        value: MarkupValue,
        from: &Rc<dyn IXamlType>,
        to: &Rc<dyn IXamlType>,
    ) -> XamlResult<MarkupValue> {
        if from.equals(&**to) {
            return Ok(value);
        }
        if XamlPseudoType::is_null(&**from) {
            if to.is_value_type() && !to.is_nullable() {
                return Err(XamlError::load_exception(
                    format!("Unable to convert {{x:Null}} to {}", to.get_fqn()),
                    Some(line_info),
                ));
            }
            return Ok(value);
        }
        if from.is_value_type() && to.is_value_type() {
            if to.is_nullable_of(&**from) {
                return Ok(value);
            }
            return Err(XamlError::load_exception(
                format!(
                    "Don't know how to convert value type {} to value type {}",
                    from.get_full_name(),
                    to.get_full_name()
                ),
                Some(line_info),
            ));
        }
        if !to.is_value_type() && from.is_value_type() {
            if !to.is_assignable_from(&**from) {
                return Err(XamlError::load_exception(
                    format!(
                        "Don't know how to convert value type {} to reference type {}",
                        from.get_full_name(),
                        to.get_full_name()
                    ),
                    Some(line_info),
                ));
            }
            return Ok(value);
        }
        if to.is_value_type() && !from.is_value_type() {
            if !from.is("System", "Object") {
                return Err(XamlError::load_exception(
                    format!(
                        "Don't know how to convert reference type {} to value type {}",
                        from.get_full_name(),
                        to.get_full_name()
                    ),
                    Some(line_info),
                ));
            }
            return self.cast(line_info, value, to);
        }
        if to.is_assignable_from(&**from) {
            // Downcast, always safe.
            Ok(value)
        } else if from.is_interface() || from.is_assignable_from(&**to) {
            // Upcast or cast from interface, checked at run time.
            self.cast(line_info, value, to)
        } else {
            Err(XamlError::load_exception(
                format!(
                    "Don't know how to convert reference type {} to reference type {}",
                    from.get_full_name(),
                    to.get_full_name()
                ),
                Some(line_info),
            ))
        }
    }

    /// The checked cast of the managed runtime: `unbox.any` for a value
    /// type (null fails, except for a nullable type), `castclass` for a
    /// reference type (null passes).
    pub fn cast(
        &self,
        line_info: &dyn IXamlLineInfo,
        value: MarkupValue,
        to: &Rc<dyn IXamlType>,
    ) -> XamlResult<MarkupValue> {
        let Some(boxed) = &value else {
            if to.is_value_type() && !to.is_nullable() {
                return Err(runtime_error(
                    "NullReferenceException",
                    format!("Unable to unbox null to {}", to.get_full_name()),
                    line_info,
                ));
            }
            return Ok(None);
        };
        if self.interpreter.type_system.is_instance(&value, &**to) {
            // The cast yields the object in the handle of its own class, whatever handle
            // it was held in (a control read from an untyped property is held as the
            // control base class): members of the target type take it through the
            // registered upcasts.
            return Ok(Some(crate::runtime::type_system::normalize_object(boxed.clone())));
        }
        Err(runtime_error(
            "InvalidCastException",
            format!(
                "Unable to cast object of type '{}' to type '{}'.",
                self.interpreter.type_system.runtime_type_of(boxed).full_name(),
                to.full_name()
            ),
            line_info,
        ))
    }

    /// Calls a method: `arguments` holds the instance (for an instance
    /// method) followed by the arguments.
    pub fn call_method(
        &mut self,
        method: &Rc<dyn IXamlMethod>,
        arguments: &[MarkupValue],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<MarkupValue> {
        let interpreter = self.interpreter;
        for evaluator in &interpreter.method_evaluators {
            if let Some(result) = evaluator.invoke(method, self, arguments) {
                return result;
            }
        }
        let any = method.as_any();
        if let Some(runtime) = any.downcast_ref::<RuntimeMethod>() {
            return runtime.invoke(arguments).map_err(|e| {
                invoke_error(e, &format!("{}.{}", method.declaring_type().full_name(), method.name()), line_info)
            });
        }
        if let Some(with_casts) = any.downcast_ref::<XamlMethodWithCasts>() {
            // The arguments whose declared type differs from the parameter
            // type of the wrapped method are cast to it.
            let base_parameters = with_casts.base_parameters_with_this();
            let parameters = method.parameters();
            let first_cast = (0..parameters.len()).find(|&c| !base_parameters[c].equals(&*parameters[c]));
            let Some(first_cast) = first_cast else {
                return self.call_method(with_casts.method(), arguments, line_info);
            };
            let mut cast_arguments = arguments.to_vec();
            for c in (first_cast..parameters.len().min(arguments.len())).rev() {
                cast_arguments[c] = self.cast(line_info, arguments[c].clone(), &base_parameters[c])?;
            }
            return self.call_method(with_casts.method(), &cast_arguments, line_info);
        }
        Err(XamlError::load_exception(
            format!(
                "Unable to find evaluator for method {}.{}: it is not a method of the run-time type system",
                method.declaring_type().full_name(),
                method.name()
            ),
            Some(line_info),
        ))
    }

    /// Calls a constructor.
    pub fn call_constructor(
        &mut self,
        constructor: &Rc<dyn IXamlConstructor>,
        arguments: &[MarkupValue],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<MarkupValue> {
        let interpreter = self.interpreter;
        for evaluator in &interpreter.constructor_evaluators {
            if let Some(result) = evaluator.invoke(constructor, self, arguments) {
                return result;
            }
        }
        match constructor.as_any().downcast_ref::<RuntimeConstructor>() {
            Some(runtime) => runtime.invoke(arguments).map_err(|e| {
                invoke_error(e, &format!("{}..ctor", constructor.declaring_type().full_name()), line_info)
            }),
            None => Err(XamlError::load_exception(
                format!(
                    "Unable to find evaluator for a constructor of {}: it is not a constructor of the run-time type system",
                    constructor.declaring_type().full_name()
                ),
                Some(line_info),
            )),
        }
    }

    /// Calls a wrapped method.
    pub fn call_wrapped_method(
        &mut self,
        method: &Rc<dyn IXamlWrappedMethod>,
        arguments: &[MarkupValue],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<MarkupValue> {
        let interpreter = self.interpreter;
        for evaluator in &interpreter.wrapped_method_evaluators {
            if let Some(result) = evaluator.invoke(method, self, arguments) {
                return result;
            }
        }
        if let Some(evaluable) = query_wrapped_method_interface::<dyn IXamlEvaluableWrappedMethod>(method) {
            return evaluable.invoke(self, arguments);
        }
        if let Some(result) = evaluators::call_standard_wrapped_method(method, self, arguments, line_info) {
            return result;
        }
        Err(XamlError::invalid_operation(format!(
            "Unable to find evaluator for wrapped method type: {}",
            method.type_name()
        )))
    }

    /// Performs a property setter on `target` with evaluated arguments.
    pub fn call_setter(
        &mut self,
        setter: &Rc<dyn IXamlPropertySetter>,
        target: &MarkupValue,
        arguments: &[MarkupValue],
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<()> {
        let interpreter = self.interpreter;
        for evaluator in &interpreter.setter_evaluators {
            if let Some(result) = evaluator.evaluate(setter, self, target, arguments) {
                return result;
            }
        }
        if let Some(evaluable) = query_property_setter_interface::<dyn IXamlEvaluablePropertySetter>(setter) {
            return evaluable.evaluate(self, target, arguments);
        }
        if let Some(result) = evaluators::call_standard_setter(setter, self, target, arguments, line_info) {
            return result;
        }
        Err(XamlError::invalid_operation(format!(
            "Unable to find evaluator for property setter type: {}",
            setter.type_name()
        )))
    }

    /// The value of a compiler local.
    pub fn local(&self, node: &Rc<dyn IXamlAstNode>) -> XamlResult<MarkupValue> {
        let address = node_address(node);
        self.locals.iter().rev().find(|(local, _)| *local == address).map(|(_, value)| value.clone()).ok_or_else(
            || XamlError::load_exception("Attempt to read uninitialized local variable", Some(&**node)),
        )
    }

    /// Stores the value of a compiler local.
    pub fn set_local(&mut self, node: &Rc<dyn IXamlAstNode>, value: MarkupValue) {
        let address = node_address(node);
        match self.locals.iter_mut().find(|(local, _)| *local == address) {
            Some(slot) => slot.1 = value,
            None => self.locals.push((address, value)),
        }
    }

    /// Whether the node (or a node below it) needs the parent stack: what
    /// decides whether the object a node initialises is pushed.
    pub fn needs_parent_stack(&mut self, node: &Rc<dyn IXamlAstNode>) -> XamlResult<bool> {
        let scope = self.scope;
        let document = &self.document;
        let cache = self.parent_stack_cache.get_or_insert_with(|| document.parent_stack_cache(scope)).clone();
        let address = node_address(node);
        if let Some(known) = cache.borrow().get(&address) {
            return Ok(*known);
        }
        visit_node(node, &mut ParentStackVisitor { cache: &cache, parents: Vec::new() })?;
        let known = cache.borrow().get(&address).copied();
        Ok(known.unwrap_or(false))
    }

    /// Fails if this evaluation tracks which nodes need the parent stack
    /// and `node` is not among the tracked nodes: it is evaluated outside
    /// of any object initialisation that could have provided the stack.
    pub fn verify_parent_stack(&self, node: &Rc<dyn IXamlAstNode>) -> XamlResult<()> {
        match &self.parent_stack_cache {
            Some(cache) if !cache.borrow().contains_key(&node_address(node)) => Err(XamlError::load_exception(
                "Node needs parent stack, but one doesn't seem to be provided",
                Some(&**node),
            )),
            _ => Ok(()),
        }
    }
}
