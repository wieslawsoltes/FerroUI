//! The evaluation of the nodes of the compiler library itself: what the
//! node emitters of the IL back end (`IL/Emitters/*.cs`) and the `Emit`
//! methods of the self-emitting nodes (`Ast/Clr.cs`, `Ast/Intrinsics.cs`,
//! `Ast/CompilerHelpers.cs`) do, node by node, in the same order.

use std::rc::Rc;

use ferroui_base::metadata::{IServiceProvider, MarkupDelegate, MarkupValue};
use ferroui_base::BoxedValue;
use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlLineInfo, IXamlPropertySetter, IXamlWrappedMethod, XamlAstCompilerLocalNode,
    XamlAstContextLocalNode, XamlAstExtensions, XamlAstImperativeValueManipulation,
    XamlAstLocalInitializationNodeEmitter, XamlAstManipulationImperativeNode, XamlAstNeedsParentStackValueNode,
    XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlAstRuntimeCastNode, XamlAstTextNode, XamlConstantNode,
    XamlDeferredContentInitializeIntermediateRootNode, XamlDeferredContentNode, XamlDirectCallPropertySetter,
    XamlIntermediateRootObjectNode, XamlLoadMethodDelegateNode, XamlManipulationGroupNode, XamlMarkupExtensionNode,
    XamlNoReturnMethodCallNode, XamlNullExtensionNode, XamlObjectInitializationNode, XamlPropertyAssignmentNode,
    XamlPropertyValueManipulationNode, XamlRootObjectNode, XamlStaticExtensionNode, XamlStaticMember,
    XamlStaticOrTargetedReturnMethodCallNode, XamlTypeExtensionNode, XamlValueNodeWithBeginInit, XamlWrappedMethod,
    XamlWrappedMethodWithCasts,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::AdderSetter;
use xamlx::type_system::{IXamlField, IXamlType, XamlPseudoType, XamlValue};

use crate::runtime::type_system::{DeferredContentFactory, RuntimeField, RuntimeMethod, RuntimeType};

use super::interpreter::{runtime_error, EvalContext, EvalResult, IXamlNodeEvaluator};

fn clr_type(node: &Rc<dyn IXamlAstValueNode>) -> XamlResult<Rc<dyn IXamlType>> {
    node.type_().get_clr_type()
}

/// Evaluates a value node as a value of its own type.
fn evaluate_as_own_type(node: &Rc<dyn IXamlAstValueNode>, context: &mut EvalContext<'_>) -> XamlResult<MarkupValue> {
    let type_ = clr_type(node)?;
    context.evaluate(&node.as_node(), Some(&type_))
}

/// The evaluator of the standard nodes: the default emitters of the IL
/// compiler (new object, text, method call, property assignment, property
/// value manipulation, manipulation group, value with manipulation, markup
/// extension, object initialisation).
pub struct StandardNodeEvaluator;

impl IXamlNodeEvaluator for StandardNodeEvaluator {
    fn evaluate(&self, node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> Option<XamlResult<EvalResult>> {
        if let Some(n) = node.cast::<XamlAstNewClrObjectNode>() {
            return Some(new_object(&n, context));
        }
        if let Some(n) = node.cast::<XamlAstTextNode>() {
            return Some(text(&n, context));
        }
        if node.as_method_call_base_node().is_some() {
            return Some(method_call(node, context));
        }
        if let Some(n) = node.cast::<XamlPropertyAssignmentNode>() {
            return Some(property_assignment(node, &n, context));
        }
        if let Some(n) = node.cast::<XamlPropertyValueManipulationNode>() {
            return Some(property_value_manipulation(&n, context));
        }
        if let Some(n) = node.cast::<XamlManipulationGroupNode>() {
            return Some(manipulation_group(&n, context));
        }
        if node.as_value_with_manipulation_node().is_some() {
            return Some(value_with_manipulation(node, context));
        }
        if let Some(n) = node.cast::<XamlMarkupExtensionNode>() {
            return Some(markup_extension(node, &n, context));
        }
        if let Some(n) = node.cast::<XamlObjectInitializationNode>() {
            return Some(object_initialization(node, &n, context));
        }
        None
    }
}

fn new_object(node: &Rc<XamlAstNewClrObjectNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let type_ = node.type_.borrow().get_clr_type()?;
    let parameters = node.constructor.parameters();
    let arguments = node.arguments.borrow().clone();
    let mut values = Vec::with_capacity(arguments.len());
    for (c, argument) in arguments.iter().enumerate() {
        let parameter = parameters.get(c).ok_or_else(|| {
            XamlError::load_exception("The constructor has fewer parameters than the node has arguments", Some(&**node))
        })?;
        values.push(context.evaluate(&argument.as_node(), Some(parameter))?);
    }
    let instance = context.call_constructor(&node.constructor, &values, &**node)?;
    Ok(EvalResult::value(type_, instance))
}

fn text(node: &Rc<XamlAstTextNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let type_ = node.type_.borrow().get_clr_type()?;
    if !type_.equals(&*context.configuration().well_known_types().string) {
        return Err(XamlError::load_exception(
            "Text node type wasn't resolved to well-known System.String",
            Some(&**node),
        ));
    }
    let value: BoxedValue = Rc::new(node.text());
    Ok(EvalResult::value(type_, Some(value)))
}

fn method_call(node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let Some(call) = node.as_method_call_base_node() else { return Ok(EvalResult::void()) };
    let this_from_arguments = node.is::<XamlStaticOrTargetedReturnMethodCallNode>();
    let expects_void = node.is::<XamlNoReturnMethodCallNode>();
    let method = call.method();
    let parameters = method.parameters_with_this();
    let offset = usize::from(!this_from_arguments);
    let arguments = call.arguments.borrow().clone();
    let mut values = Vec::with_capacity(arguments.len() + offset);
    if !this_from_arguments {
        values.push(context.target()?);
    }
    for (c, argument) in arguments.iter().enumerate() {
        let expected = parameters.get(c + offset).ok_or_else(|| {
            XamlError::load_exception(
                format!("Method {} has fewer parameters than the node has arguments", method.name()),
                Some(&**node),
            )
        })?;
        values.push(context.evaluate(&argument.as_node(), Some(expected))?);
    }
    let result = context.call_wrapped_method(&method, &values, &**node)?;
    let return_type = method.return_type();
    let is_void = return_type.equals(&*context.configuration().well_known_types().void);
    if !expects_void && is_void {
        return Err(XamlError::load_exception(
            format!("XamlStaticReturnMethodCallNode expects a value while {} returns void", method.name()),
            Some(&**node),
        ));
    }
    Ok(if is_void || expects_void { EvalResult::void() } else { EvalResult::value(return_type, result) })
}

/// What is decided once about a property assignment: the setters that can
/// take its values, and the static types of the values.
pub struct AssignmentPlan {
    _node: Rc<dyn IXamlAstNode>,
    setters: Vec<Rc<dyn IXamlPropertySetter>>,
    value_types: Vec<Rc<dyn IXamlType>>,
}

fn last_parameter(setter: &Rc<dyn IXamlPropertySetter>) -> XamlResult<Rc<dyn IXamlType>> {
    setter.parameters().last().cloned().ok_or_else(|| {
        XamlError::internal("ArgumentOutOfRangeException", "A property setter doesn't have a value parameter")
    })
}

fn assignment_plan(node: &Rc<dyn IXamlAstNode>, assignment: &XamlPropertyAssignmentNode) -> XamlResult<AssignmentPlan> {
    let values = assignment.values.borrow().clone();
    let possible = assignment.possible_setters.borrow().clone();
    let mut value_types = Vec::with_capacity(values.len());
    for value in &values {
        value_types.push(clr_type(value)?);
    }
    let dynamic_type = value_types
        .last()
        .cloned()
        .ok_or_else(|| XamlError::invalid_operation("Sequence contains no elements"))?;

    // The setters that take as many values as the assignment has.
    let mut setters: Vec<Rc<dyn IXamlPropertySetter>> =
        possible.iter().filter(|s| s.parameters().len() == values.len()).cloned().collect();
    if values.len() > 1 && setters.len() > 1 {
        for c in 0..values.len().saturating_sub(2) {
            let failed = possible
                .iter()
                .find(|s| s.parameters().get(c).is_none_or(|p| !p.is_directly_assignable_from(&*value_types[c])));
            if let Some(failed) = failed {
                return Err(XamlError::load_exception(
                    format!(
                        "Can not statically cast {} to {} and runtime type checking is only supported for the last setter argument",
                        value_types[c].get_fqn(),
                        failed.parameters().get(c).map(|p| p.get_fqn()).unwrap_or_default()
                    ),
                    Some(&**node),
                ));
            }
        }
    }
    if setters.is_empty() {
        return Err(XamlError::load_exception("No setters found for property assignment", Some(&**node)));
    }

    // Removes the setters that can never be chosen.
    if setters.len() > 1 {
        if dynamic_type.is_value_type() {
            // A value of a value type always uses the first one.
            setters.truncate(1);
        } else {
            let mut index = 0;
            while index < setters.len() {
                let setter = setters[index].clone();
                let type_ = last_parameter(&setter)?;
                // The value is assignable and the setter allows null: it always matches.
                if type_.is_assignable_from(&*dynamic_type) && setter.binder_parameters().allow_runtime_null.get() {
                    setters.truncate(index + 1);
                    break;
                }
                // A previous setter already matches the type of this one or a base type of it.
                let mut redundant = false;
                for previous in &setters[..index] {
                    if last_parameter(previous)?.is_assignable_from(&*type_)
                        && (previous.binder_parameters().allow_runtime_null.get()
                            || !setter.binder_parameters().allow_runtime_null.get())
                    {
                        redundant = true;
                        break;
                    }
                }
                if redundant {
                    setters.remove(index);
                    continue;
                }
                index += 1;
            }
        }
    }
    Ok(AssignmentPlan { _node: node.clone(), setters, value_types })
}

fn property_assignment(
    node: &Rc<dyn IXamlAstNode>,
    assignment: &Rc<XamlPropertyAssignmentNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let address = Rc::as_ptr(node) as *const () as usize;
    let plan = context.document().clone().assignment_plan(address, || assignment_plan(node, assignment))?;
    let target = context.target()?;
    let values = assignment.values.borrow().clone();
    let (dynamic_value, leading) = values
        .split_last()
        .ok_or_else(|| XamlError::invalid_operation("Sequence contains no elements"))?;

    if let [setter] = plan.setters.as_slice() {
        if let Some(result) = evaluate_setter_with_arguments(setter, context, &target, &values, &**node) {
            result?;
            return Ok(EvalResult::void());
        }
        let mut arguments = Vec::with_capacity(values.len());
        for value in leading {
            arguments.push(evaluate_as_own_type(value, context)?);
        }
        arguments.push(context.evaluate(&dynamic_value.as_node(), Some(&last_parameter(setter)?))?);
        context.call_setter(setter, &target, &arguments, &**node)?;
        return Ok(EvalResult::void());
    }

    // Several setters: the one that takes the value is chosen by the
    // run-time type of the last value (the dynamic setter method).
    let mut arguments = Vec::with_capacity(values.len());
    for value in &values {
        arguments.push(evaluate_as_own_type(value, context)?);
    }
    let dynamic_index = arguments.len() - 1;
    let dynamic_type = &plan.value_types[dynamic_index];
    let line_info: &dyn IXamlLineInfo = &**dynamic_value;
    let type_system = context.type_system().clone();
    let mut first_setter_allowing_null: Option<&Rc<dyn IXamlPropertySetter>> = None;
    for setter in &plan.setters {
        if setter.binder_parameters().allow_runtime_null.get() && first_setter_allowing_null.is_none() {
            first_setter_allowing_null = Some(setter);
        }
        let parameter = last_parameter(setter)?;
        let mut type_on_stack = dynamic_type.clone();
        // Only checked at run time if the value isn't known to be assignable.
        if !parameter.is_assignable_from(&**dynamic_type) {
            // For a nullable value type the value is checked against the value type; null is handled below.
            let checked = match parameter.is_nullable() {
                true => parameter.generic_arguments().into_iter().next().unwrap_or_else(|| parameter.clone()),
                false => parameter.clone(),
            };
            if !type_system.is_instance(&arguments[dynamic_index], &*checked) {
                continue;
            }
            type_on_stack = checked;
        } else if !setter.binder_parameters().allow_runtime_null.get() && arguments[dynamic_index].is_none() {
            continue;
        }
        let value = arguments[dynamic_index].take();
        arguments[dynamic_index] = context.convert(line_info, value, &type_on_stack, &parameter)?;
        context.call_setter(setter, &target, &arguments, &**node)?;
        return Ok(EvalResult::void());
    }
    // The value didn't match any type, but it may be null.
    if arguments[dynamic_index].is_none() {
        return match first_setter_allowing_null {
            Some(setter) => {
                let parameter = last_parameter(setter)?;
                arguments[dynamic_index] = context.convert(line_info, None, &XamlPseudoType::null(), &parameter)?;
                context.call_setter(setter, &target, &arguments, &**node)?;
                Ok(EvalResult::void())
            }
            None => Err(runtime_error(
                "NullReferenceException",
                format!("No setter of property {} accepts null", assignment.property.name()),
                &**node,
            )),
        };
    }
    let actual = arguments[dynamic_index].as_ref().map(|v| type_system.runtime_type_of(v).full_name());
    Err(runtime_error(
        "InvalidCastException",
        format!(
            "No setter of property {} accepts a value of type '{}'",
            assignment.property.name(),
            actual.unwrap_or_default()
        ),
        &**node,
    ))
}

/// The optimised path of a single setter: a setter that evaluates its
/// arguments itself.
fn evaluate_setter_with_arguments(
    setter: &Rc<dyn IXamlPropertySetter>,
    context: &mut EvalContext<'_>,
    target: &MarkupValue,
    values: &[Rc<dyn IXamlAstValueNode>],
    line_info: &dyn IXamlLineInfo,
) -> Option<XamlResult<()>> {
    let interpreter = context.interpreter();
    for evaluator in &interpreter.setter_evaluators {
        if let Some(result) = evaluator.evaluate_with_arguments(setter, context, target, values) {
            return Some(result);
        }
    }
    let any = setter.as_any();
    let parameters = setter.parameters();
    let evaluate_arguments = |context: &mut EvalContext<'_>, arguments: &mut Vec<MarkupValue>| -> XamlResult<()> {
        for (i, value) in values.iter().enumerate() {
            let parameter = parameters.get(i).ok_or_else(|| {
                XamlError::load_exception("The setter has fewer parameters than the assignment has values", Some(line_info))
            })?;
            arguments.push(context.evaluate(&value.as_node(), Some(parameter))?);
        }
        Ok(())
    };
    if let Some(direct) = any.downcast_ref::<XamlDirectCallPropertySetter>() {
        return Some((|| {
            let mut arguments = Vec::with_capacity(values.len() + 1);
            arguments.push(target.clone());
            evaluate_arguments(context, &mut arguments)?;
            context.call_method(direct.method(), &arguments, line_info).map(|_| ())
        })());
    }
    if let Some(adder) = any.downcast_ref::<AdderSetter>() {
        return Some((|| {
            // The collection is read before the arguments are evaluated.
            let collection = context.call_method(adder.getter(), std::slice::from_ref(target), line_info)?;
            let mut arguments = Vec::with_capacity(values.len() + 1);
            arguments.push(collection);
            evaluate_arguments(context, &mut arguments)?;
            context.call_method(adder.adder(), &arguments, line_info).map(|_| ())
        })());
    }
    None
}

/// The setters of the compiler library: a direct method call and the
/// adder of a collection property.
pub(crate) fn call_standard_setter(
    setter: &Rc<dyn IXamlPropertySetter>,
    context: &mut EvalContext<'_>,
    target: &MarkupValue,
    arguments: &[MarkupValue],
    line_info: &dyn IXamlLineInfo,
) -> Option<XamlResult<()>> {
    let any = setter.as_any();
    let with_target = |first: MarkupValue| {
        let mut all = Vec::with_capacity(arguments.len() + 1);
        all.push(first);
        all.extend(arguments.iter().cloned());
        all
    };
    if let Some(direct) = any.downcast_ref::<XamlDirectCallPropertySetter>() {
        let all = with_target(target.clone());
        return Some(context.call_method(direct.method(), &all, line_info).map(|_| ()));
    }
    if let Some(adder) = any.downcast_ref::<AdderSetter>() {
        return Some((|| {
            let collection = context.call_method(adder.getter(), std::slice::from_ref(target), line_info)?;
            context.call_method(adder.adder(), &with_target(collection), line_info).map(|_| ())
        })());
    }
    None
}

/// The wrapped methods of the compiler library: a plain method, and a
/// method whose arguments are cast to its parameter types first.
pub(crate) fn call_standard_wrapped_method(
    method: &Rc<dyn IXamlWrappedMethod>,
    context: &mut EvalContext<'_>,
    arguments: &[MarkupValue],
    line_info: &dyn IXamlLineInfo,
) -> Option<XamlResult<MarkupValue>> {
    let any = method.as_any();
    if let Some(wrapped) = any.downcast_ref::<XamlWrappedMethod>() {
        return Some(context.call_method(wrapped.method(), arguments, line_info));
    }
    if let Some(with_casts) = any.downcast_ref::<XamlWrappedMethodWithCasts>() {
        return Some((|| {
            let inner = with_casts.method();
            let inner_parameters = inner.parameters_with_this();
            let parameters = method.parameters_with_this();
            let first_cast = (0..parameters.len()).find(|&c| !inner_parameters[c].equals(&*parameters[c]));
            let Some(first_cast) = first_cast else {
                return context.call_wrapped_method(inner, arguments, line_info);
            };
            let mut cast_arguments = arguments.to_vec();
            for c in (first_cast..parameters.len().min(arguments.len())).rev() {
                cast_arguments[c] = context.cast(line_info, arguments[c].clone(), &inner_parameters[c])?;
            }
            context.call_wrapped_method(inner, &cast_arguments, line_info)
        })());
    }
    None
}

fn property_value_manipulation(
    node: &Rc<XamlPropertyValueManipulationNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let property = node.property.borrow().clone();
    let getter = property.getter().ok_or_else(|| {
        XamlError::invalid_operation(format!("Property {} doesn't have a getter", property.to_node_string()))
    })?;
    let target = context.target()?;
    let value = context.call_method(&getter, std::slice::from_ref(&target), &**node)?;
    let manipulation = node.manipulation.borrow().clone();
    context.manipulate(&manipulation.as_node(), &value)?;
    Ok(EvalResult::void())
}

fn manipulation_group(node: &Rc<XamlManipulationGroupNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let target = context.target()?;
    let count = node.children.borrow().len();
    for c in 0..count {
        let Some(child) = node.children.borrow().get(c).cloned() else { break };
        context.manipulate(&child.as_node(), &target)?;
    }
    Ok(EvalResult::void())
}

fn value_with_manipulation(node: &Rc<dyn IXamlAstNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let Some(vwm) = node.as_value_with_manipulation_node() else { return Ok(EvalResult::void()) };
    let type_ = IXamlAstValueNode::type_(vwm).get_clr_type()?;
    let created = context.evaluate_typed(&vwm.value().as_node(), Some(&type_))?;
    if let Some(manipulation) = vwm.manipulation() {
        let empty_group =
            manipulation.cast::<XamlManipulationGroupNode>().is_some_and(|group| group.children.borrow().is_empty());
        if !empty_group {
            context.manipulate(&manipulation.as_node(), &created.value)?;
        }
    }
    Ok(created)
}

fn markup_extension(
    node: &Rc<dyn IXamlAstNode>,
    me: &Rc<XamlMarkupExtensionNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    context.verify_parent_stack(node)?;
    let property = context
        .parent_nodes()
        .find_map(|parent| parent.cast::<XamlPropertyAssignmentNode>())
        .map(|assignment| assignment.property.clone());
    let parameters = me.provide_value.parameters();
    let needs_context = !parameters.is_empty();
    let provide_value_target =
        needs_context && context.interpreter().context_definition().provide_value_target && property.is_some();

    let value = me.value();
    let mut arguments = Vec::with_capacity(2);
    arguments.push(evaluate_as_own_type(&value, context)?);
    if let Some(parameter) = parameters.first() {
        arguments.push(context.context_value(&**parameter)?);
    }
    if let (true, Some(property)) = (provide_value_target, &property) {
        let evaluator = context.interpreter().provide_value_target_property_evaluator.clone();
        let descriptor = match evaluator {
            Some(evaluator) => evaluator(context, property)?,
            None => None,
        };
        let descriptor = match descriptor {
            Some(descriptor) => descriptor,
            None => {
                let name: BoxedValue = Rc::new(property.name());
                Some(name)
            }
        };
        context.runtime_context()?.set_target_property(descriptor);
    }
    let result = context.call_method(&me.provide_value, &arguments, &**node)?;
    if provide_value_target {
        context.runtime_context()?.set_target_property(None);
    }
    Ok(EvalResult::value(me.provide_value.return_type(), result))
}

/// Calls a parameterless instance method of the initialisation contract
/// (`BeginInit`, `EndInit`) on `target`.
fn call_support_initialize(
    context: &mut EvalContext<'_>,
    support_initialize: &Rc<dyn IXamlType>,
    name: &str,
    target: &MarkupValue,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let method = support_initialize.get_method(|m| m.name() == name)?;
    context.call_method(&method, std::slice::from_ref(target), line_info).map(|_| ())
}

fn object_initialization(
    node: &Rc<dyn IXamlAstNode>,
    init: &Rc<XamlObjectInitializationNode>,
    context: &mut EvalContext<'_>,
) -> XamlResult<EvalResult> {
    let type_ = init.type_.borrow().clone();
    let target = context.target()?;
    let support_initialize = context
        .configuration()
        .type_mappings
        .support_initialize
        .clone()
        .filter(|support_initialize| support_initialize.is_assignable_from(&*type_));
    if let Some(support_initialize) = &support_initialize {
        if !init.skip_begin_init.get() {
            call_support_initialize(context, support_initialize, "BeginInit", &target, &**node)?;
        }
    }
    let add_to_parent_stack = context.interpreter().context_definition().parent_stack_provider
        && !type_.is_value_type()
        && context.needs_parent_stack(node)?;
    if add_to_parent_stack {
        context.runtime_context()?.push_parent(target.clone());
    }
    context.manipulate(&init.manipulation().as_node(), &target)?;
    if add_to_parent_stack {
        context.runtime_context()?.pop_parent();
    }
    if let Some(support_initialize) = &support_initialize {
        call_support_initialize(context, support_initialize, "EndInit", &target, &**node)?;
    }
    Ok(EvalResult::void())
}

fn numeric_constant(constant: &XamlValue) -> Option<(i128, f64)> {
    Some(match constant {
        XamlValue::Boolean(v) => (i128::from(*v), f64::from(u8::from(*v))),
        XamlValue::Char(v) => (i128::from(u32::from(*v)), f64::from(u32::from(*v))),
        XamlValue::SByte(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Byte(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int16(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::UInt16(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int32(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::UInt32(v) => (i128::from(*v), f64::from(*v)),
        XamlValue::Int64(v) => (i128::from(*v), *v as f64),
        XamlValue::UInt64(v) => (i128::from(*v), *v as f64),
        XamlValue::Single(v) => (*v as i128, f64::from(*v)),
        XamlValue::Double(v) => (*v as i128, *v),
        _ => return None,
    })
}

/// The run-time value of a compile-time constant of the type `type_`: the
/// value the IL back end loads for it.
pub fn constant_value(
    type_: &Rc<dyn IXamlType>,
    constant: &XamlValue,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<MarkupValue> {
    fn boxed<T: PartialEq + 'static>(value: T) -> XamlResult<MarkupValue> {
        let value: BoxedValue = Rc::new(value);
        Ok(Some(value))
    }
    let unsupported = || {
        XamlError::load_exception(
            format!("Don't know how to load the constant {constant:?} as a value of {}", type_.get_full_name()),
            Some(line_info),
        )
    };
    if let XamlValue::String(text) = constant {
        return boxed(text.clone());
    }
    if matches!(constant, XamlValue::Null) {
        return Ok(None);
    }
    let (integer, float) = numeric_constant(constant).ok_or_else(unsupported)?;
    if type_.is_enum() {
        // The constant is the numeric value: of a member, or of a combination of flags.
        let markup = type_.as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::markup);
        let value = i64::try_from(integer).ok().and_then(|value| {
            let markup = markup?;
            match markup.enum_from_value {
                Some(from_value) => from_value(value),
                None => markup.enum_members.iter().find(|m| m.value == value).map(|m| (m.get)()),
            }
        });
        return match value {
            Some(value) => Ok(Some(value)),
            None => Err(XamlError::load_exception(
                format!(
                    "{integer} is not a value of the enumeration {}: it is neither a member nor a combination of its flags",
                    type_.get_full_name()
                ),
                Some(line_info),
            )),
        };
    }
    macro_rules! integer {
        ($type_:ty) => {
            boxed(<$type_>::try_from(integer).map_err(|_| unsupported())?)
        };
    }
    if type_.namespace().as_deref() == Some("System") {
        match type_.name().as_str() {
            "Boolean" => return boxed(integer != 0),
            "Char" => {
                return boxed(u32::try_from(integer).ok().and_then(char::from_u32).ok_or_else(unsupported)?);
            }
            "SByte" => return integer!(i8),
            "Byte" => return integer!(u8),
            "Int16" => return integer!(i16),
            "UInt16" => return integer!(u16),
            "Int32" => return integer!(i32),
            "UInt32" => return integer!(u32),
            "Int64" => return integer!(i64),
            "UInt64" => return integer!(u64),
            "Single" => return boxed(float as f32),
            "Double" => return boxed(float),
            _ => {}
        }
    }
    // The constant is typed as something else (`System.Object`): its own kind decides.
    match constant {
        XamlValue::Boolean(v) => boxed(*v),
        XamlValue::Char(v) => boxed(*v),
        XamlValue::SByte(v) => boxed(*v),
        XamlValue::Byte(v) => boxed(*v),
        XamlValue::Int16(v) => boxed(*v),
        XamlValue::UInt16(v) => boxed(*v),
        XamlValue::Int32(v) => boxed(*v),
        XamlValue::UInt32(v) => boxed(*v),
        XamlValue::Int64(v) => boxed(*v),
        XamlValue::UInt64(v) => boxed(*v),
        XamlValue::Single(v) => boxed(*v),
        XamlValue::Double(v) => boxed(*v),
        _ => Err(unsupported()),
    }
}

fn field_value(field: &Rc<dyn IXamlField>, line_info: &dyn IXamlLineInfo) -> XamlResult<MarkupValue> {
    if let Some(runtime) = field.as_any().downcast_ref::<RuntimeField>() {
        return runtime
            .get()
            .map_err(|e| runtime_error("TargetInvocationException", e.to_string(), line_info));
    }
    if field.is_literal() {
        return constant_value(&field.field_type(), &field.get_literal_value()?, line_info);
    }
    Err(XamlError::load_exception(
        format!(
            "Unable to read field {}.{}: it is not a field of the run-time type system",
            field.declaring_type().full_name(),
            field.name()
        ),
        Some(line_info),
    ))
}

/// The nodes that emit themselves upstream (`Ast/Intrinsics.cs`,
/// `Ast/CompilerHelpers.cs`, the deferred content nodes of `Ast/Clr.cs`).
pub(crate) fn evaluate_intrinsic(
    node: &Rc<dyn IXamlAstNode>,
    context: &mut EvalContext<'_>,
) -> Option<XamlResult<EvalResult>> {
    if node.is::<XamlNullExtensionNode>() {
        return Some(Ok(EvalResult::value(XamlPseudoType::null(), None)));
    }
    if let Some(n) = node.cast::<XamlTypeExtensionNode>() {
        return Some((|| {
            let type_ = n.value().get_clr_type()?;
            let value = context.type_system().type_value(&type_);
            Ok(EvalResult::value(n.system_type().clone(), Some(value)))
        })());
    }
    if let Some(n) = node.cast::<XamlStaticExtensionNode>() {
        return Some((|| match n.resolve_member(true)? {
            Some(XamlStaticMember::Property(property)) => {
                let getter = property.getter().ok_or_else(|| {
                    XamlError::invalid_operation(format!("Property {} doesn't have a getter", property.name()))
                })?;
                let value = context.call_method(&getter, &[], &*n)?;
                Ok(EvalResult::value(getter.return_type(), value))
            }
            Some(XamlStaticMember::Field(field)) => {
                let value = field_value(&field, &*n)?;
                Ok(EvalResult::value(field.field_type(), value))
            }
            None => Err(XamlError::invalid_operation("Operation is not valid due to the current state of the object.")),
        })());
    }
    if let Some(n) = node.cast::<XamlConstantNode>() {
        return Some((|| {
            let type_ = IXamlAstValueNode::type_(&*n).get_clr_type()?;
            let value = constant_value(&type_, &n.constant, &*n)?;
            Ok(EvalResult::value(type_, value))
        })());
    }
    if let Some(n) = node.cast::<XamlRootObjectNode>() {
        return Some((|| {
            let value = context.runtime_context()?.root_object_field();
            Ok(EvalResult::value(n.type_.borrow().get_clr_type()?, value))
        })());
    }
    if let Some(n) = node.cast::<XamlIntermediateRootObjectNode>() {
        return Some((|| {
            let value = context.runtime_context()?.intermediate_root_object();
            Ok(EvalResult::value(n.type_.borrow().get_clr_type()?, value))
        })());
    }
    if let Some(n) = node.cast::<XamlLoadMethodDelegateNode>() {
        return Some(load_method_delegate(&n, context));
    }
    if let Some(n) = node.cast::<XamlAstCompilerLocalNode>() {
        return Some(context.local(node).map(|value| EvalResult::value(n.type_.clone(), value)));
    }
    if let Some(n) = node.cast::<XamlAstLocalInitializationNodeEmitter>() {
        return Some((|| {
            let local = n.local();
            let result = context.evaluate_typed(&n.base.value().as_node(), Some(&local.type_))?;
            context.set_local(&local.as_node(), result.value.clone());
            Ok(result)
        })());
    }
    if let Some(n) = node.cast::<XamlValueNodeWithBeginInit>() {
        return Some((|| {
            let value = n.base.value();
            let type_ = clr_type(&value)?;
            let result = context.evaluate_typed(&value.as_node(), Some(&type_))?;
            let support_initialize = context.configuration().type_mappings.support_initialize.clone();
            if let Some(support_initialize) = support_initialize.filter(|s| s.is_assignable_from(&*type_)) {
                call_support_initialize(context, &support_initialize, "BeginInit", &result.value, &*n)?;
            }
            Ok(result)
        })());
    }
    if let Some(n) = node.cast::<XamlAstManipulationImperativeNode>() {
        // The value this node is "supposed" to manipulate is discarded.
        return Some(context.execute(&n.imperative().as_node()).map(|_| EvalResult::void()));
    }
    if let Some(n) = node.cast::<XamlAstImperativeValueManipulation>() {
        return Some((|| {
            let value = evaluate_as_own_type(&n.value(), context)?;
            context.manipulate(&n.manipulation().as_node(), &value)?;
            Ok(EvalResult::void())
        })());
    }
    if let Some(n) = node.cast::<XamlAstContextLocalNode>() {
        return Some((|| {
            let type_ = IXamlAstValueNode::type_(&*n).get_clr_type()?;
            let value = context.context_value(&*type_)?;
            Ok(EvalResult::value(type_, value))
        })());
    }
    if let Some(n) = node.cast::<XamlAstRuntimeCastNode>() {
        return Some((|| {
            let object = context.configuration().well_known_types().object.clone();
            let value = context.evaluate(&n.value().as_node(), Some(&object))?;
            let type_ = n.type_.borrow().get_clr_type()?;
            let value = context.cast(&*n, value, &type_)?;
            Ok(EvalResult::value(type_, value))
        })());
    }
    if let Some(n) = node.cast::<XamlAstNeedsParentStackValueNode>() {
        return Some((|| {
            context.verify_parent_stack(node)?;
            let value = n.base.value();
            let type_ = clr_type(&value)?;
            context.evaluate_typed(&value.as_node(), Some(&type_))
        })());
    }
    if let Some(n) = node.cast::<XamlDeferredContentNode>() {
        return Some(deferred_content(&n, context));
    }
    if let Some(n) = node.cast::<XamlDeferredContentInitializeIntermediateRootNode>() {
        return Some((|| {
            let value = n.value();
            let type_ = clr_type(&value)?;
            let runtime_context = context.runtime_context()?.clone();
            let root = context.evaluate(&value.as_node(), Some(&type_))?;
            runtime_context.set_intermediate_root_object(root);
            Ok(EvalResult::value(type_, runtime_context.intermediate_root_object()))
        })());
    }
    None
}

fn load_method_delegate(node: &Rc<XamlLoadMethodDelegateNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let declaring_type = node.method.declaring_type();
    let instance = context.evaluate(&node.base.value().as_node(), Some(&declaring_type))?;
    if node.method.as_any().downcast_ref::<RuntimeMethod>().is_none() {
        return Err(XamlError::load_exception(
            format!(
                "Unable to create a delegate for {}.{}: it is not a method of the run-time type system",
                declaring_type.full_name(),
                node.method.name()
            ),
            Some(&**node),
        ));
    }
    let method = node.method.clone();
    let is_static = method.is_static();
    // The delegate does not keep its target alive: the target (the root object) owns
    // the element the handler is attached to, and a strong reference would close a
    // cycle root -> element -> handler -> root. A handler whose target is gone does
    // nothing.
    let instance = instance.as_ref().map(ferroui_base::data::core::WeakValue::new);
    let delegate = MarkupDelegate::new(move |arguments| {
        let runtime = method.as_any().downcast_ref::<RuntimeMethod>()?;
        let mut all = Vec::with_capacity(arguments.len() + 1);
        if !is_static {
            match &instance {
                Some(instance) => all.push(Some(crate::runtime::type_system::normalize_object(instance.upgrade()?))),
                None => all.push(None),
            }
        }
        all.extend(arguments.iter().cloned());
        // A handler has nowhere to report a failure to.
        runtime.invoke(&all).ok().flatten()
    });
    let delegate: BoxedValue = Rc::new(delegate);
    Ok(EvalResult::value(node.delegate_type.clone(), Some(delegate)))
}

fn deferred_content(node: &Rc<XamlDeferredContentNode>, context: &mut EvalContext<'_>) -> XamlResult<EvalResult> {
    let type_ = IXamlAstValueNode::type_(&**node).get_clr_type()?;
    let object = context.configuration().well_known_types().object.clone();
    let has_root_object_provider = context.interpreter().context_definition().root_object_provider;
    let interpreter = context.interpreter().clone();
    let document = context.document().clone();
    let value = node.value();
    let scope = Rc::as_ptr(node) as *const () as usize;

    // The build method of the closure: creates its own context, chained to
    // the service provider it is called with, and builds the content.
    let factory = DeferredContentFactory::new(move |service_provider: Option<Rc<dyn IServiceProvider>>| {
        let runtime_context = interpreter.create_context(service_provider.clone(), &document, &*value)?;
        if has_root_object_provider {
            // The root object is the one of the calling service provider.
            if let Some(parent) = &service_provider {
                if let Some(root) = interpreter.services.get_parent_root_object(parent) {
                    runtime_context.set_root_object(root);
                }
            }
        }
        let mut eval = EvalContext::new(&interpreter, Some(runtime_context), document.clone(), scope);
        eval.evaluate(&value.as_node(), Some(&object))
    });
    let factory: BoxedValue = Rc::new(factory);

    // Lets the language save values of the parent context, pass its own service provider and so on.
    let result = match node.deferred_content_customization() {
        Some(customization) => {
            let customization = match node.deferred_content_customization_type_parameter() {
                Some(type_parameter) => customization.make_generic_method(std::slice::from_ref(type_parameter))?,
                None => customization.clone(),
            };
            let service_provider = context.configuration().type_mappings.service_provider()?;
            let context_value = context.context_value(&*service_provider)?;
            context.call_method(&customization, &[Some(factory), context_value], &**node)?
        }
        None => Some(factory),
    };
    Ok(EvalResult::value(type_, result))
}
