//! The property setters of the framework language: what the `Emit` and
//! `EmitWithArguments` methods of the setters of the managed original
//! generate (each setter type documents its IL).

use std::rc::Rc;

use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::{from_markup_value, MarkupValue};
use ferroui_base::UnsetValueType;
use xamlx::ast::{IXamlAstValueNode, IXamlLineInfo, IXamlPropertySetter};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::IXamlType;

use crate::compiler_extensions::transformers::{
    ClassBindingSetter, ClassValueSetter, FerroAttachedInstancePropertySetterMethod, ResourceAdderSetter,
    XamlDirectCallAddHandler, XamlIlDirectCallPropertySetter,
};
use crate::compiler_extensions::{
    BindingSetter, BindingWithPrioritySetter, SetValueWithPrioritySetter, UnsetValueSetter,
};
use crate::runtime::interpreter::{runtime_error, EvalContext, IXamlSetterEvaluator};

use super::helpers::{binding_of, boxed, enum_constant, evaluate_as, exact_property_value, load_field, load_property, object_of};
use super::nodes::create_source_info;

/// Performs the property setters of the framework language. See
/// [`FRAMEWORK_EVALUATORS`](super::FRAMEWORK_EVALUATORS) for the list.
pub struct FrameworkSetterEvaluator;

/// A position for errors of a setter: setters are not nodes, the error is
/// reported without one and the property assignment adds its own.
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

fn argument<'a>(arguments: &'a [MarkupValue], index: usize) -> XamlResult<&'a MarkupValue> {
    arguments.get(index).ok_or_else(|| {
        XamlError::internal("ArgumentOutOfRangeException", "A property setter received fewer arguments than it has parameters")
    })
}

fn parameter(parameters: &[Rc<dyn IXamlType>], index: usize) -> XamlResult<Rc<dyn IXamlType>> {
    parameters.get(index).cloned().ok_or_else(|| {
        XamlError::internal("ArgumentOutOfRangeException", "A property setter has fewer parameters than the assignment has values")
    })
}

fn priority_of(value: &MarkupValue, line_info: &dyn IXamlLineInfo) -> XamlResult<BindingPriority> {
    from_markup_value::<BindingPriority>(value).ok_or_else(|| {
        runtime_error("InvalidCastException", "The priority of a property assignment is not a binding priority", line_info)
    })
}

impl IXamlSetterEvaluator for FrameworkSetterEvaluator {
    fn evaluate(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        context: &mut EvalContext<'_>,
        target: &MarkupValue,
        arguments: &[MarkupValue],
    ) -> Option<XamlResult<()>> {
        let line = &NoLineInfo;
        let any = setter.as_any();
        // `target.Bind(property, binding)`.
        if let Some(s) = any.downcast_ref::<BindingSetter>() {
            return Some((|| bind(target, s.ferro_property(), argument(arguments, 0)?, line))());
        }
        // The priority is discarded: a binding decides its own priority.
        if let Some(s) = any.downcast_ref::<BindingWithPrioritySetter>() {
            return Some((|| bind(target, s.ferro_property(), argument(arguments, 1)?, line))());
        }
        // `target.SetValue<T>(property, value, priority)`.
        if let Some(s) = any.downcast_ref::<SetValueWithPrioritySetter>() {
            return Some((|| {
                let priority = priority_of(argument(arguments, 0)?, line)?;
                set_value(target, s.ferro_property(), argument(arguments, 1)?, priority, line)
            })());
        }
        // The instance passed to the setter is ignored in favour of the marker itself.
        if let Some(s) = any.downcast_ref::<UnsetValueSetter>() {
            return Some(unset_value(target, s, line));
        }
        if let Some(s) = any.downcast_ref::<XamlIlDirectCallPropertySetter>() {
            return Some((|| {
                let value = argument(arguments, 0)?.clone();
                context.call_method(&s.method, &[target.clone(), value], line).map(|_| ())
            })());
        }
        // `target.Classes.Set(className, value)`.
        if let Some(s) = any.downcast_ref::<ClassValueSetter>() {
            return Some((|| {
                let value = argument(arguments, 0)?.clone();
                let getter = s.types.styled_element_classes_property.getter().ok_or_else(|| {
                    XamlError::invalid_operation("Property StyledElement.Classes doesn't have a getter")
                })?;
                let classes = context.call_method(&getter, std::slice::from_ref(target), line)?;
                let set = s.classes_set_method()?;
                context.call_method(&set, &[classes, boxed(s.class_name.clone()), value], line).map(|_| ())
            })());
        }
        // `StyledElementExtensions.BindClass(target, className, binding, null)`.
        if let Some(s) = any.downcast_ref::<ClassBindingSetter>() {
            return Some((|| {
                let binding = argument(arguments, 0)?.clone();
                context
                    .call_method(
                        &s.types.classes_bind_method,
                        &[target.clone(), boxed(s.class_name.clone()), binding, None],
                        line,
                    )
                    .map(|_| ())
            })());
        }
        // `target.SetValue(Field, (object) value, BindingPriority.LocalValue)`.
        if let Some(s) = any.downcast_ref::<FerroAttachedInstancePropertySetterMethod>() {
            return Some((|| {
                let value = arguments.last().ok_or_else(|| {
                    XamlError::internal("ArgumentOutOfRangeException", "A property setter received no value")
                })?;
                // The method the IL calls must exist.
                let _ = s.parent.set_value_method()?;
                set_value(target, &s.parent.field, value, BindingPriority::LocalValue, line)
            })());
        }
        // Upstream only calls the add method here, without the routed event, the
        // routing strategies and the flag; the path is not reachable because the
        // assignment prefers the form that evaluates the arguments.
        if let Some(s) = any.downcast_ref::<XamlDirectCallAddHandler>() {
            return Some((|| add_handler(s, context, target, argument(arguments, 0)?.clone(), line))());
        }
        if let Some(s) = any.downcast_ref::<ResourceAdderSetter>() {
            return Some((|| {
                let key = argument(arguments, 0)?.clone();
                let value = argument(arguments, 1)?.clone();
                add_resource(s, context, target, |_| Ok(key), |_| Ok(value), line)
            })());
        }
        None
    }

    fn evaluate_with_arguments(
        &self,
        setter: &Rc<dyn IXamlPropertySetter>,
        context: &mut EvalContext<'_>,
        target: &MarkupValue,
        arguments: &[Rc<dyn IXamlAstValueNode>],
    ) -> Option<XamlResult<()>> {
        fn node(arguments: &[Rc<dyn IXamlAstValueNode>], index: usize) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
            arguments.get(index).cloned().ok_or_else(|| {
                XamlError::internal("ArgumentOutOfRangeException", "A property assignment has fewer values than its setter has parameters")
            })
        }
        let any = setter.as_any();
        let parameters = setter.parameters();
        if let Some(s) = any.downcast_ref::<BindingSetter>() {
            return Some((|| {
                let argument = node(arguments, 0)?;
                let binding = evaluate_as(&argument, context, &parameter(&parameters, 0)?)?;
                bind(target, s.ferro_property(), &binding, &*argument)
            })());
        }
        // The priority node is not evaluated at all.
        if let Some(s) = any.downcast_ref::<BindingWithPrioritySetter>() {
            return Some((|| {
                let argument = node(arguments, 1)?;
                let binding = evaluate_as(&argument, context, &parameter(&parameters, 1)?)?;
                bind(target, s.ferro_property(), &binding, &*argument)
            })());
        }
        // The value is evaluated before the priority.
        if let Some(s) = any.downcast_ref::<SetValueWithPrioritySetter>() {
            return Some((|| {
                let value_node = node(arguments, 1)?;
                let value = evaluate_as(&value_node, context, &parameter(&parameters, 1)?)?;
                let priority_node = node(arguments, 0)?;
                let priority = evaluate_as(&priority_node, context, &parameter(&parameters, 0)?)?;
                let priority = priority_of(&priority, &*priority_node)?;
                set_value(target, s.ferro_property(), &value, priority, &*value_node)
            })());
        }
        // The argument is not evaluated at all.
        if let Some(s) = any.downcast_ref::<UnsetValueSetter>() {
            return Some(match arguments.first() {
                Some(argument) => unset_value(target, s, &**argument),
                None => unset_value(target, s, &NoLineInfo),
            });
        }
        if let Some(s) = any.downcast_ref::<XamlDirectCallAddHandler>() {
            return Some((|| {
                let argument = node(arguments, 0)?;
                let handler = evaluate_as(&argument, context, &parameter(&parameters, 0)?)?;
                add_handler(s, context, target, handler, &*argument)
            })());
        }
        if let Some(s) = any.downcast_ref::<ResourceAdderSetter>() {
            return Some((|| {
                let key_node = node(arguments, 0)?;
                let value_node = node(arguments, 1)?;
                let key_type = parameter(&parameters, 0)?;
                let value_type = parameter(&parameters, 1)?;
                add_resource(
                    s,
                    context,
                    target,
                    |context| evaluate_as(&key_node, context, &key_type),
                    |context| evaluate_as(&value_node, context, &value_type),
                    &*key_node,
                )
            })());
        }
        None
    }
}

fn bind(
    target: &MarkupValue,
    field: &Rc<dyn xamlx::type_system::IXamlField>,
    binding: &MarkupValue,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let property = load_property(field, line_info)?;
    let binding = binding_of(binding, line_info)?;
    let object = object_of(target, line_info)?;
    let _ = object.bind_binding(property, &*binding);
    Ok(())
}

fn set_value(
    target: &MarkupValue,
    field: &Rc<dyn xamlx::type_system::IXamlField>,
    value: &MarkupValue,
    priority: BindingPriority,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let property = load_property(field, line_info)?;
    let value = exact_property_value(property, value, line_info)?;
    let object = object_of(target, line_info)?;
    let _ = object.set_value_untyped(property, value.as_any(), priority);
    Ok(())
}

/// `target.SetValue(property, FerroProperty.UnsetValue, BindingPriority.LocalValue)`.
fn unset_value(target: &MarkupValue, setter: &UnsetValueSetter, line_info: &dyn IXamlLineInfo) -> XamlResult<()> {
    let property = load_property(setter.ferro_property(), line_info)?;
    // The IL loads the marker from the static field `FerroProperty.UnsetValue`; the
    // marker has one value, so nothing is read here (and the metadata of the property
    // class need not declare the field for a document to load).
    let object = object_of(target, line_info)?;
    let _ = object.set_value_untyped(property, &UnsetValueType, BindingPriority::LocalValue);
    Ok(())
}

/// `target.AddHandler(EventField, handler, RoutingStrategies.Direct | RoutingStrategies.Bubble, false)`.
fn add_handler(
    setter: &XamlDirectCallAddHandler,
    context: &mut EvalContext<'_>,
    target: &MarkupValue,
    handler: MarkupValue,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let event = load_field(&setter.event_field, line_info)?;
    let event = untyped_routed_event(&setter.event_field, event);
    let parameters = setter.add_method.parameters();
    let routes_type = parameters.get(2).cloned().ok_or_else(|| {
        XamlError::load_exception("The AddHandler method doesn't take routing strategies", Some(line_info))
    })?;
    let routes = enum_constant(&routes_type, 5, line_info)?;
    context
        .call_method(&setter.add_method, &[target.clone(), event, handler, routes, boxed(false)], line_info)
        .map(|_| ())
}

/// `var d = target[.Resources]; var k = key; d.Add(k, value);
/// XamlSourceInfo.SetXamlSourceInfo(d, k, info);`.
fn add_resource(
    setter: &ResourceAdderSetter,
    context: &mut EvalContext<'_>,
    target: &MarkupValue,
    key: impl FnOnce(&mut EvalContext<'_>) -> XamlResult<MarkupValue>,
    value: impl FnOnce(&mut EvalContext<'_>) -> XamlResult<MarkupValue>,
    line_info: &dyn IXamlLineInfo,
) -> XamlResult<()> {
    let dictionary = match &setter.getter {
        Some(getter) => context.call_method(getter, std::slice::from_ref(target), line_info)?,
        None => target.clone(),
    };
    let key = key(context)?;
    let value = value(context)?;
    context.call_method(&setter.adder, &[dictionary.clone(), key.clone(), value], line_info)?;
    if setter.emit_source_info {
        let info = create_source_info(
            &setter.types.xaml_source_info_constructor,
            setter.line,
            setter.position,
            setter.document.as_deref(),
            context,
            line_info,
        )?;
        context.call_method(&setter.types.xaml_source_info_dictionary_setter, &[dictionary, key, info], line_info)?;
    }
    Ok(())
}

/// The routed event a field holds as the untyped event handle the object
/// model attaches handlers to. A field declared with its typed handle
/// (`RoutedEvent<TEventArgs>`) that has no registered conversion to the
/// untyped handle is looked up in the registry of routed events by its
/// owner and name.
fn untyped_routed_event(field: &Rc<dyn xamlx::type_system::IXamlField>, value: MarkupValue) -> MarkupValue {
    use ferroui_base::interactivity::{RoutedEvent, RoutedEventRegistry};
    if from_markup_value::<RoutedEvent>(&value).is_some() {
        return value;
    }
    let declaring_type = field.declaring_type();
    let owner = declaring_type
        .as_any()
        .downcast_ref::<crate::runtime::type_system::RuntimeType>()
        .and_then(crate::runtime::type_system::RuntimeType::type_info);
    let field_name = field.name();
    let name = field_name.strip_suffix("Event").unwrap_or(&field_name);
    let found = owner.and_then(|owner| {
        RoutedEventRegistry::instance().get_registered(owner).into_iter().find(|event| event.name() == name)
    });
    match found {
        Some(event) => boxed(event),
        None => value,
    }
}
