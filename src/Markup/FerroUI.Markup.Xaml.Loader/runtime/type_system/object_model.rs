//! The members of the object model the compiler resolves on the root
//! classes and that metadata cannot declare: the generic methods
//! (`FerroObject.SetValue<T>`, `Interactive.AddHandler<TEventArgs>`) and the
//! untyped accessors of the property system next to them (`SetValue`,
//! `GetValue`, `Bind`, `AddHandler`). They are projected with the signatures
//! of the managed original and invokers over the untyped API of the object
//! model; a member metadata declares itself is not projected a second time.
//!
//! The members return what the object model returns (the `IDisposable` of
//! `SetValue`, the binding expression of `Bind`) as untyped handles.

use std::rc::Rc;

use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingBase, BindingPriority};
use ferroui_base::interactivity::Interactive;
use ferroui_base::metadata::{from_markup_value, MarkupInvokeError, MarkupValue};
use ferroui_base::{FerroObject, FerroProperty, TypeInfo};
use xamlx::type_system::{IXamlType, IXamlTypeSystem as _};

use super::runtime_type::{
    to_exact, RuntimeInvoker, RuntimeMembers, RuntimeType, RuntimeTypeKind, RuntimeTypeOrigin,
    RuntimeTypeSpec,
};
use super::runtime_type_system::{RuntimeTypeSystem, PROPERTY_NAMESPACE};
use super::values::to_untyped;

fn argument<T: Clone + 'static>(arguments: &[MarkupValue], index: usize) -> Result<T, MarkupInvokeError> {
    let value = arguments
        .get(index)
        .ok_or(MarkupInvokeError::ArgumentCount { expected: index + 1, actual: arguments.len() })?;
    from_markup_value::<T>(value).ok_or_else(|| MarkupInvokeError::Argument {
        index,
        expected: std::any::type_name::<T>(),
        actual: match value {
            Some(value) => value.type_name().to_string(),
            None => "null".to_string(),
        },
    })
}

fn object(arguments: &[MarkupValue]) -> Result<ferroui_base::Ref<FerroObject>, MarkupInvokeError> {
    let object = arguments
        .first()
        .and_then(|a| a.as_ref())
        .and_then(|a| ferroui_base::data::core::ValueTypes::as_object(&**a));
    object.ok_or_else(|| MarkupInvokeError::Argument {
        index: 0,
        expected: "ferroui_base::Ref<ferroui_base::FerroObject>",
        actual: match arguments.first() {
            Some(Some(value)) => value.type_name().to_string(),
            _ => "null".to_string(),
        },
    })
}

fn dynamic(invoke: impl Fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> + 'static) -> RuntimeInvoker {
    RuntimeInvoker::Dynamic(Rc::new(invoke))
}

fn generic_parameter(system: &RuntimeTypeSystem, owner: &str, name: &str) -> Rc<RuntimeType> {
    RuntimeType::create(
        &system.weak(),
        RuntimeTypeSpec::new(
            format!("{owner}!!{name}"),
            "",
            name,
            RuntimeTypeKind::GenericParameter,
            RuntimeTypeOrigin::GenericParameter,
        ),
    )
}

/// `target.SetValue(property, value, priority)` over the untyped setter.
fn set_value(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    let target = object(arguments)?;
    let property = argument::<&'static FerroProperty>(arguments, 1)?;
    let priority = argument::<BindingPriority>(arguments, 3)?;
    let unset = arguments.get(2).and_then(|v| v.as_ref()).is_some_and(|v| v.downcast_ref::<ferroui_base::UnsetValueType>().is_some());
    if unset {
        return Ok(disposable(target.set_value_untyped(property, &ferroui_base::UnsetValueType, priority)));
    }
    let value = to_exact(
        arguments.get(2).unwrap_or(&None),
        ValueType::new(property.property_type(), property.property_type_name()),
        2,
    )?;
    Ok(disposable(target.set_value_untyped(property, value.as_any(), priority)))
}

fn disposable(result: Option<Rc<dyn ferroui_base::reactive::IDisposable>>) -> MarkupValue {
    result.map(|disposable| Rc::new(disposable) as ferroui_base::BoxedValue)
}

pub(crate) fn project_object_model_members(
    system: &RuntimeTypeSystem,
    type_: &Rc<RuntimeType>,
    type_info: &'static TypeInfo,
    members: &mut RuntimeMembers,
) {
    if std::ptr::eq(type_info, FerroObject::TYPE) {
        project_ferro_object(system, type_, members);
    } else if std::ptr::eq(type_info, Interactive::TYPE) {
        project_interactive(system, type_, members);
    }
}

fn declared(members: &RuntimeMembers, name: &str, parameters: usize, first: impl Fn(&Rc<dyn IXamlType>) -> bool) -> bool {
    members.methods.iter().any(|m| {
        m.name == name && !m.is_static && m.parameters.len() == parameters && m.parameters.first().is_some_and(&first)
    })
}

fn project_ferro_object(system: &RuntimeTypeSystem, type_: &Rc<RuntimeType>, members: &mut RuntimeMembers) {
    let name = |name: &str| format!("{PROPERTY_NAMESPACE}.{name}");
    let (Some(property), Some(styled), Some(priority)) = (
        system.find_type(&name("FerroProperty")),
        system.find_type(&name("StyledProperty`1")),
        system.find_type("FerroUI.Data.BindingPriority"),
    ) else {
        return;
    };
    let object_type = system.get("System.Object");
    let disposable = system.get("System.IDisposable");
    let no_handles = |count: usize| vec![None; count];

    // IDisposable SetValue<T>(StyledProperty<T> property, T value, BindingPriority priority)
    if !declared(members, "SetValue", 3, |p| p.name() == "StyledProperty`1") {
        let t = generic_parameter(system, "FerroUI.FerroObject.SetValue", "T");
        let t_type: Rc<dyn IXamlType> = t.clone();
        if let Ok(styled_of_t) = styled.make_generic_type(std::slice::from_ref(&t_type)) {
            let method = system.method(
                type_,
                "SetValue".to_string(),
                false,
                disposable.clone(),
                (vec![styled_of_t, t_type, priority.clone()], no_handles(3)),
                dynamic(set_value),
                Vec::new(),
            );
            if let Ok(method) = Rc::try_unwrap(method) {
                members.methods.push(Rc::new(method.into_generic_definition(vec![t])));
            }
        }
    }
    // IDisposable SetValue(FerroProperty property, object value, BindingPriority priority)
    if !declared(members, "SetValue", 3, |p| p.equals(&*property)) {
        members.methods.push(system.method(
            type_,
            "SetValue".to_string(),
            false,
            disposable,
            (vec![property.clone(), object_type.clone(), priority], no_handles(3)),
            dynamic(set_value),
            Vec::new(),
        ));
    }
    // object GetValue(FerroProperty property)
    if !declared(members, "GetValue", 1, |p| p.equals(&*property)) {
        members.methods.push(system.method(
            type_,
            "GetValue".to_string(),
            false,
            object_type,
            (vec![property.clone()], no_handles(1)),
            dynamic(|arguments| {
                let target = object(arguments)?;
                let property = argument::<&'static FerroProperty>(arguments, 1)?;
                Ok(to_untyped(target.get_value_untyped(property)))
            }),
            Vec::new(),
        ));
    }
    // BindingExpressionBase Bind(FerroProperty property, BindingBase binding)
    if let (Some(binding), Some(expression)) =
        (system.find_type("FerroUI.Data.BindingBase"), system.find_type("FerroUI.Data.BindingExpressionBase"))
    {
        if !declared(members, "Bind", 2, |p| p.equals(&*property)) {
            members.methods.push(system.method(
                type_,
                "Bind".to_string(),
                false,
                expression,
                (vec![property, binding], no_handles(2)),
                dynamic(|arguments| {
                    let target = object(arguments)?;
                    let property = argument::<&'static FerroProperty>(arguments, 1)?;
                    let binding = argument::<Rc<dyn BindingBase>>(arguments, 2)?;
                    let expression = target.bind_binding(property, &*binding);
                    Ok(Some(Rc::new(expression) as ferroui_base::BoxedValue))
                }),
                Vec::new(),
            ));
        }
    }
}

/// `target.AddHandler(routedEvent, handler, routes, handledEventsToo)`: the
/// handler is called with the element it is attached to and a shared handle
/// to the event arguments.
fn add_handler(arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
    let target = object(arguments)?;
    let target = target.cast::<Interactive>().ok_or_else(|| MarkupInvokeError::Argument {
        index: 0,
        expected: "ferroui_base::Ref<ferroui_base::interactivity::Interactive>",
        actual: target.get_type().full_name(),
    })?;
    let event = argument::<ferroui_base::interactivity::RoutedEvent>(arguments, 1)?;
    let handler = argument::<ferroui_base::metadata::MarkupDelegate>(arguments, 2)?;
    let routes = argument::<ferroui_base::interactivity::RoutingStrategies>(arguments, 3)?;
    let handled_events_too = argument::<bool>(arguments, 4)?;
    let _ = target.add_handler_untyped(&event, handler, routes, handled_events_too);
    Ok(None)
}

fn project_interactive(system: &RuntimeTypeSystem, type_: &Rc<RuntimeType>, members: &mut RuntimeMembers) {
    const NAMESPACE: &str = "FerroUI.Interactivity";
    let (Some(routed_event), Some(routes)) = (
        system.find_type(&format!("{NAMESPACE}.RoutedEvent")),
        system.find_type(&format!("{NAMESPACE}.RoutingStrategies")),
    ) else {
        return;
    };
    let delegate = system.get("System.Delegate");
    let boolean = system.get("System.Boolean");
    let void = system.void();
    let no_handles = |count: usize| vec![None; count];

    // void AddHandler(RoutedEvent routedEvent, Delegate handler, RoutingStrategies routes, bool handledEventsToo)
    if !declared(members, "AddHandler", 4, |p| p.equals(&*routed_event)) {
        members.methods.push(system.method(
            type_,
            "AddHandler".to_string(),
            false,
            void.clone(),
            (vec![routed_event.clone(), delegate, routes.clone(), boolean.clone()], no_handles(4)),
            dynamic(add_handler),
            Vec::new(),
        ));
    }
    // void AddHandler<TEventArgs>(RoutedEvent<TEventArgs> routedEvent, EventHandler<TEventArgs> handler,
    //     RoutingStrategies routes, bool handledEventsToo)
    let generic_event = system.find_runtime_type(&format!("{NAMESPACE}.RoutedEvent`1"));
    let handler = system.find_type("System.EventHandler`1");
    if let (Some(generic_event), Some(handler)) = (generic_event, handler) {
        if !declared(members, "AddHandler", 4, |p| p.generic_arguments().len() == 1) {
            let t = generic_parameter(system, "FerroUI.Interactivity.Interactive.AddHandler", "TEventArgs");
            let t_type: Rc<dyn IXamlType> = t.clone();
            let event_of_t =
                system.instantiate_open(&generic_event, std::slice::from_ref(&t_type), routed_event.clone());
            let handler_of_t = handler.make_generic_type(std::slice::from_ref(&t_type));
            if let Ok(handler_of_t) = handler_of_t {
                let method = system.method(
                    type_,
                    "AddHandler".to_string(),
                    false,
                    void,
                    (vec![event_of_t, handler_of_t, routes, boolean], no_handles(4)),
                    dynamic(add_handler),
                    Vec::new(),
                );
                if let Ok(method) = Rc::try_unwrap(method) {
                    members.methods.push(Rc::new(method.into_generic_definition(vec![t])));
                }
            }
        }
    }
}
