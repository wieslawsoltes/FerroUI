//! The evaluation of compiled binding paths: `XamlIlBindingPathNode` and
//! its elements become calls of the compiled binding path builder, with
//! property descriptions materialised from the invokers of the run-time
//! type system (the counterpart of the property info, accessor factory and
//! trampoline emitters of the IL back end).

use std::rc::Rc;

use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::plugins::PropertyAccessorFactory;
use ferroui_base::data::core::{
    BoxedPropertyGetter, BoxedPropertySetter, ClrPropertyInfo, FalliblePropertyGetter, IPropertyInfo, PropertySetter,
    ValueType, ValueTypes, INDEXER_NAME,
};
use ferroui_base::data::{BindingError, CompiledBindingPathBuilder};
use ferroui_base::metadata::{from_markup_value, MarkupDelegate, MarkupInvokeError, MarkupValue};
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_markup_xaml::markup_extensions::compiled_bindings::PropertyInfoAccessorFactory;
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::{IXamlAstValueNode, IXamlLineInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlMethod, IXamlProperty, IXamlType};

use crate::compiler_extensions::{
    XamlIlBindingPathElementNode, XamlIlBindingPathNode, XamlIlPropertyAccessorFactory,
};
use crate::runtime::interpreter::{runtime_error, EvalContext, EvalResult};
use crate::runtime::type_system::{box_object, RuntimeMethod, RuntimeType};

use super::helpers::{boxed, evaluate_as_own_type, load_property};
use super::services::name_scope_of;

/// The accessors of a plain property of the run-time type system, as the accessors of a
/// property description: the `ClrPropertyInfo` the IL back end generates for a property.
/// The accessors of metadata take the handle of the owner, so the description reads and
/// writes through its boxed accessors; the accessors that are given a view of the owner
/// recover the handle where that is possible (objects of the object model).
#[derive(Clone)]
struct RuntimePropertyAccessors {
    name: String,
    getter: Option<Rc<dyn IXamlMethod>>,
    setter: Option<Rc<dyn IXamlMethod>>,
    /// The arguments of an indexer; empty for a property.
    indexer_arguments: Vec<MarkupValue>,
    /// A static property: its accessors take no instance.
    is_static: bool,
    /// The getter returns `System.Boolean`: it returns one of the two cached boxes
    /// (`XamlIlClrPropertyGetterResult::CachedBoxedBoolean`), so that reading a boolean
    /// property does not allocate a box per read.
    cached_boxed_boolean: bool,
}

/// The owner in the form invokers take it. A member receives the handle of
/// its instance; from a borrowed value the handle can only be recovered for
/// objects of the object model.
fn owner_handle(target: &dyn AnyValue) -> Result<MarkupValue, BindingError> {
    match ValueTypes::as_object(target).and_then(box_object) {
        Some(handle) => Ok(Some(handle)),
        None => Err(BindingError::message(format!(
            "A property of '{}' cannot be read through metadata: the value is not an object of the object model",
            target.type_name()
        ))),
    }
}

fn invoke(method: &Rc<dyn IXamlMethod>, arguments: &[MarkupValue]) -> Result<MarkupValue, BindingError> {
    match method.as_any().downcast_ref::<RuntimeMethod>() {
        Some(runtime) => runtime.invoke(arguments).map_err(|e| BindingError::message(e.to_string())),
        None => Err(BindingError::message(format!("Method {} has no invoker", method.name()))),
    }
}

impl RuntimePropertyAccessors {
    /// The arguments of an accessor: the owner (unless the property is static), the
    /// arguments of an indexer and, for a setter, the value.
    fn arguments(
        &self,
        owner: impl FnOnce() -> Result<MarkupValue, BindingError>,
        value: Option<Option<&BoxedValue>>,
    ) -> Result<Vec<MarkupValue>, BindingError> {
        let mut arguments = Vec::with_capacity(2 + self.indexer_arguments.len());
        if !self.is_static {
            arguments.push(owner()?);
        }
        arguments.extend(self.indexer_arguments.iter().cloned());
        if let Some(value) = value {
            arguments.push(value.cloned());
        }
        Ok(arguments)
    }

    fn get(&self, owner: impl FnOnce() -> Result<MarkupValue, BindingError>) -> Result<Option<BoxedValue>, BindingError> {
        let getter = self
            .getter
            .as_ref()
            .ok_or_else(|| BindingError::message(format!("Property {} doesn't have a getter", self.name)))?;
        let value = invoke(getter, &self.arguments(owner, None)?)?;
        if self.cached_boxed_boolean {
            if let Some(value) = value.as_ref().and_then(|value| value.downcast_ref::<bool>()) {
                return Ok(Some(ferroui_base::utilities::BooleanBoxes::box_(*value)));
            }
        }
        Ok(value)
    }

    fn set(
        &self,
        owner: impl FnOnce() -> Result<MarkupValue, BindingError>,
        value: Option<&BoxedValue>,
    ) -> Result<(), BindingError> {
        let setter = self
            .setter
            .as_ref()
            .ok_or_else(|| BindingError::message(format!("Property {} doesn't have a setter", self.name)))?;
        invoke(setter, &self.arguments(owner, Some(value))?).map(|_| ())
    }

    /// The property description over these accessors.
    fn into_property_info(self, property_type: ValueType) -> ClrPropertyInfo {
        let name = self.name.clone();
        let (has_getter, has_setter) = (self.getter.is_some(), self.setter.is_some());
        let this = Rc::new(self);
        let getter: Option<FalliblePropertyGetter> = has_getter.then(|| {
            let this = this.clone();
            Rc::new(move |target: &dyn AnyValue| this.get(|| owner_handle(target))) as FalliblePropertyGetter
        });
        let setter: Option<PropertySetter> = has_setter.then(|| {
            let this = this.clone();
            Rc::new(move |target: &dyn AnyValue, value: Option<&BoxedValue>| this.set(|| owner_handle(target), value))
                as PropertySetter
        });
        let boxed_getter: Option<BoxedPropertyGetter> = has_getter.then(|| {
            let this = this.clone();
            Rc::new(move |target: &BoxedValue| this.get(|| Ok(Some(target.clone())))) as BoxedPropertyGetter
        });
        let boxed_setter: Option<BoxedPropertySetter> = has_setter.then(|| {
            let this = this.clone();
            Rc::new(move |target: &BoxedValue, value: Option<&BoxedValue>| this.set(|| Ok(Some(target.clone())), value))
                as BoxedPropertySetter
        });
        ClrPropertyInfo::new_fallible(&name, getter, setter, property_type).with_boxed_accessors(boxed_getter, boxed_setter)
    }
}

fn invoke_command(method: &Rc<dyn IXamlMethod>, arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    match method.as_any().downcast_ref::<RuntimeMethod>() {
        Some(runtime) => runtime.invoke(arguments),
        None => Err(MarkupInvokeError::Failed(format!("Method {} has no invoker", method.name()))),
    }
}

/// Raises the failure of the execute trampoline of a method used as a command. The
/// trampoline of the managed original unboxes (value types) or casts (reference types) the
/// command parameter to the parameter type and calls the method; the exception of a failed
/// cast, or the one the method throws, escapes `ICommand.Execute`. The counterpart of an
/// escaping exception is a panic that names it.
fn raise_command_failure(method: &Rc<dyn IXamlMethod>, parameter: Option<&BoxedValue>, error: MarkupInvokeError) -> ! {
    match error {
        MarkupInvokeError::Argument { expected, actual, .. } => {
            let is_value_type = method.parameters().first().is_some_and(|p| p.is_value_type());
            if parameter.is_none() && is_value_type {
                // `unbox.any` of a null reference.
                panic!("NullReferenceException: Object reference not set to an instance of an object.");
            }
            panic!("InvalidCastException: Unable to cast object of type '{actual}' to type '{expected}'.");
        }
        MarkupInvokeError::ArgumentCount { .. } => {
            panic!("TargetParameterCountException: {}.{}: {error}", method.declaring_type().get_full_name(), method.name())
        }
        MarkupInvokeError::Failed(message) => panic!("{message}"),
    }
}

fn handle_of(type_: &Rc<dyn IXamlType>) -> ValueType {
    type_.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::handle).unwrap_or_else(ValueType::object)
}

/// The description of a plain property (`XamlIlClrPropertyInfoEmitter.Emit`):
/// `indexer_arguments` are the constant arguments of an indexer.
pub(crate) fn property_info(
    context: &mut EvalContext<'_>,
    property: &Rc<dyn IXamlProperty>,
    indexer_arguments: Option<&[Rc<dyn IXamlAstValueNode>]>,
    _line_info: &dyn IXamlLineInfo,
) -> XamlResult<Rc<dyn IPropertyInfo>> {
    let mut arguments = Vec::new();
    for argument in indexer_arguments.unwrap_or(&[]) {
        arguments.push(evaluate_as_own_type(argument, context)?);
    }
    let name = match indexer_arguments {
        Some(_) => INDEXER_NAME.to_string(),
        None => property.name(),
    };
    let accessors = RuntimePropertyAccessors {
        name,
        getter: property.getter(),
        setter: property.setter(),
        indexer_arguments: arguments,
        is_static: property.getter().or_else(|| property.setter()).is_some_and(|accessor| accessor.is_static()),
        cached_boxed_boolean: property.getter().is_some_and(|getter| getter.return_type().is("System", "Boolean")),
    };
    Ok(Rc::new(accessors.into_property_info(handle_of(&property.property_type()))))
}

fn inpc_factory() -> PropertyAccessorFactory {
    Rc::new(PropertyInfoAccessorFactory::create_inpc_property_accessor)
}

fn class_of(type_: &Rc<dyn IXamlType>, line_info: &dyn IXamlLineInfo) -> XamlResult<&'static ferroui_base::TypeInfo> {
    type_.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::type_info).ok_or_else(|| {
        XamlError::load_exception(
            format!("{} is not a class of the object model: it cannot be an ancestor type", type_.get_full_name()),
            Some(line_info),
        )
    })
}

fn level_of(level: i32, line_info: &dyn IXamlLineInfo) -> XamlResult<usize> {
    usize::try_from(level).map_err(|_| {
        runtime_error("ArgumentOutOfRangeException", "The ancestor level of a binding path is negative", line_info)
    })
}

fn element(
    builder: CompiledBindingPathBuilder,
    element: &XamlIlBindingPathElementNode,
    node: &Rc<XamlIlBindingPathNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<CompiledBindingPathBuilder> {
    let line_info: &dyn IXamlLineInfo = &**node;
    Ok(match element {
        XamlIlBindingPathElementNode::Not(_) => builder.not(),
        XamlIlBindingPathElementNode::StreamObservable(_) => builder.stream_observable(),
        XamlIlBindingPathElementNode::StreamTask(_) => builder.stream_task(),
        XamlIlBindingPathElementNode::SelfElement(_) => builder.self_(),
        XamlIlBindingPathElementNode::FindAncestor(e) => {
            builder.ancestor(Some(class_of(&e.type_, line_info)?), level_of(e.level, line_info)?)
        }
        XamlIlBindingPathElementNode::FindVisualAncestor(e) => {
            builder.visual_ancestor(Some(class_of(&e.type_, line_info)?), level_of(e.level, line_info)?)
        }
        XamlIlBindingPathElementNode::ElementName(e) => {
            let scope = name_scope_of(context.runtime_context()?).ok_or_else(|| {
                runtime_error("ArgumentNullException", "Value cannot be null. (Parameter 'nameScope')", line_info)
            })?;
            builder.element_name(NameScopeRef(scope), &e.name)
        }
        XamlIlBindingPathElementNode::TemplatedParent(_) => builder.templated_parent(),
        XamlIlBindingPathElementNode::FerroProperty(e) => {
            let property = load_property(&e.field, line_info)?;
            match e.accepts_null {
                true => builder.ferro_property_with(property, true),
                false => builder.ferro_property(property),
            }
        }
        XamlIlBindingPathElementNode::ClrProperty(e) => {
            // --- typed emission (hook of the type system) ------------------------------
            // `builder.Property<TSource, TValue>(info, accessorFactory, acceptsNull)`: a path
            // that is this single property (`try_enable_typed_emission`) gets the typed
            // element from the typed path hook of the property's metadata declaration, so
            // that the binding is a `TypedBindingExpression`. A property without a typed
            // form gets the untyped element below, with the same semantics.
            if e.emit_typed.get() {
                if let Some(typed) =
                    crate::runtime::type_system::typed_path_element(&*e.property, &builder, e.accepts_null)
                {
                    return Ok(typed);
                }
            }
            // --- end of typed emission --------------------------------------------------
            let info = property_info(context, &e.property, None, line_info)?;
            match e.accepts_null {
                true => builder.property_with(info, inpc_factory(), true),
                false => builder.property(info, inpc_factory()),
            }
        }
        XamlIlBindingPathElementNode::ClrIndexer(e) => {
            let info = property_info(context, &e.property, Some(&e.values), line_info)?;
            let factory: PropertyAccessorFactory = match e.accessor_factory(context.configuration())? {
                XamlIlPropertyAccessorFactory::Indexer { index } => {
                    let index = evaluate_as_own_type(&index, context)?;
                    let index = from_markup_value::<i32>(&index).ok_or_else(|| {
                        runtime_error("InvalidCastException", "The argument of an indexer is not a 32-bit integer", line_info)
                    })?;
                    Rc::new(move |target, property| {
                        PropertyInfoAccessorFactory::create_indexer_property_accessor(target, property, index)
                    })
                }
                _ => inpc_factory(),
            };
            builder.property(info, factory)
        }
        XamlIlBindingPathElementNode::ArrayIndexer(e) => builder.array_element(&e.values),
        XamlIlBindingPathElementNode::TypeCast(e) => {
            let runtime = e.type_.as_any().downcast_ref::<RuntimeType>();
            let target = match runtime.and_then(RuntimeType::type_info) {
                Some(class) => CastTarget::Class(class),
                None => CastTarget::Value(runtime.and_then(RuntimeType::handle).ok_or_else(|| {
                    XamlError::load_exception(
                        format!("{} has no run-time representation to cast to", e.type_.get_full_name()),
                        Some(line_info),
                    )
                })?),
            };
            builder.type_cast_value(target)
        }
        XamlIlBindingPathElementNode::ClrMethod(e) => {
            // `builder.Method(method, delegateType[, true])`: per source object a
            // delegate bound to the source.
            let method = e.method.clone();
            let name = method.name();
            builder.method_untyped(
                &name,
                Rc::new(move |owner: &BoxedValue| {
                    let method = method.clone();
                    let owner = owner.clone();
                    let is_static = method.is_static();
                    let delegate = MarkupDelegate::new(move |arguments| {
                        let mut all = Vec::with_capacity(arguments.len() + 1);
                        if !is_static {
                            all.push(Some(owner.clone()));
                        }
                        all.extend(arguments.iter().cloned());
                        // A delegate has nowhere to report a failure to.
                        invoke(&method, &all).ok().flatten()
                    });
                    Rc::new(delegate) as BoxedValue
                }),
                e.accepts_null,
            )
        }
        XamlIlBindingPathElementNode::ClrMethodAsCommand(e) => {
            // The execute and can-execute trampolines: the owner, then the command
            // parameter if the method takes one, cast to the parameter type (`unbox.any` /
            // `castclass` of the trampoline). A failed cast and a failure of the method
            // escape `ICommand.Execute` as the exception does in the managed original.
            let execute_method = e.execute_method.clone();
            let takes_parameter = !execute_method.parameters().is_empty();
            let execute: ferroui_base::data::core::plugins::UntypedExecute =
                Rc::new(move |owner: &BoxedValue, parameter: Option<&BoxedValue>| {
                    let mut arguments = vec![Some(owner.clone())];
                    if takes_parameter {
                        arguments.push(parameter.cloned());
                    }
                    if let Err(error) = invoke_command(&execute_method, &arguments) {
                        raise_command_failure(&execute_method, parameter, error);
                    }
                });
            let can_execute: Option<ferroui_base::data::core::plugins::UntypedCanExecute> =
                e.can_execute_method.clone().map(|can_execute| {
                    Rc::new(move |owner: &BoxedValue, parameter: Option<&BoxedValue>| {
                        let result = invoke(&can_execute, &[Some(owner.clone()), parameter.cloned()]);
                        result.ok().and_then(|r| from_markup_value::<bool>(&r)).unwrap_or(false)
                    }) as ferroui_base::data::core::plugins::UntypedCanExecute
                });
            let depends_on: Vec<&str> = e.depends_on_properties.iter().map(String::as_str).collect();
            let notifier = e
                .execute_method
                .declaring_type()
                .as_any()
                .downcast_ref::<RuntimeType>()
                .and_then(RuntimeType::markup)
                .and_then(|markup| markup.notify_property_changed);
            builder.command_untyped(&e.execute_method.name(), execute, can_execute, &depends_on, notifier)
        }
    })
}

/// `new CompiledBindingPathBuilder()`, the builder call of each transform
/// element and then of each element, `Build()`.
pub(crate) fn evaluate(node: &Rc<XamlIlBindingPathNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    node.try_enable_typed_emission();
    let mut builder = CompiledBindingPathBuilder::new();
    let transform_elements = node.transform_elements.borrow().clone();
    let elements = node.elements.borrow().clone();
    for path_element in transform_elements.iter().chain(elements.iter()) {
        builder = element(builder, path_element, node, context)?;
    }
    let type_ = IXamlAstValueNode::type_(&**node).get_clr_type()?;
    Ok(EvalResult::value(type_, boxed(builder.build())))
}
