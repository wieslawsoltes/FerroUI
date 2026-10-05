//! Port of `CompilerExtensions/Transformers/FerroXamlIlTransformRoutedEvent.cs`.

use std::any::Any;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlPropertySetter, PropertySetterBinderParameters, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstNamePropertyReference, XamlAstNodeExtensions,
};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlCustomAttribute, IXamlField, IXamlMethod, IXamlType};

use super::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;

/// Resolves a routed event used as an attribute (`<Button Click="OnClick"/>`,
/// `<StackPanel Button.Click="OnClick"/>`): when the declaring type has a static
/// `<Name>Event` field holding a routed event, the reference becomes a property of the target
/// type whose setters add the handler with `Interactive.AddHandler`.
pub struct FerroXamlIlTransformRoutedEvent;

impl IXamlAstTransformer for FerroXamlIlTransformRoutedEvent {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(prop) = node.cast::<XamlAstNamePropertyReference>() else {
            return Ok(node);
        };
        let target_ref = prop.target_type.borrow().clone().cast::<XamlAstClrTypeReference>();
        let declaring_ref = prop
            .declaring_type
            .borrow()
            .clone()
            .cast::<XamlAstClrTypeReference>();
        let (Some(target_ref), Some(declaring_ref)) = (target_ref, declaring_ref) else {
            return Ok(node);
        };

        let xkt = context.try_get_ferro_types()?;
        let interactive_type = &xkt.interactivity.interactive;
        let routed_event_type = &xkt.interactivity.routed_event;

        if !interactive_type.is_assignable_from(&*target_ref.type_) {
            return Ok(node);
        }

        let prop_name = prop.name();
        let event_name = format!("{prop_name}Event");
        let event_field = declaring_ref
            .type_
            .get_all_fields()
            .into_iter()
            .find(|f| f.is_static() && f.name() == event_name);
        let Some(event_field) = event_field else {
            return Ok(node);
        };

        let event_field_type = event_field.field_type();
        if routed_event_type.is_assignable_from(&*event_field_type) {
            let instance =
                XamlAstClrProperty::new(&*prop, &prop_name, target_ref.type_.clone(), None);
            instance
                .setters
                .borrow_mut()
                .push(XamlDirectCallAddHandler::new(
                    event_field.clone(),
                    target_ref.type_.clone(),
                    xkt.interactivity.add_handler.clone(),
                    xkt.interactivity.routed_event_handler.clone(),
                ));
            let generic_arguments = event_field_type.generic_arguments();
            if generic_arguments.len() == 1 {
                let argument = generic_arguments[0].clone();
                if !argument.equals(&*xkt.interactivity.routed_event_args) {
                    instance
                        .setters
                        .borrow_mut()
                        .push(XamlDirectCallAddHandler::new(
                            event_field,
                            target_ref.type_.clone(),
                            xkt.interactivity
                                .add_handler_t
                                .make_generic_method(std::slice::from_ref(&argument))?,
                            xkt.event_handler_t
                                .make_generic_type(std::slice::from_ref(&argument))?,
                        ));
                }
            }

            Ok(instance)
        } else {
            context.report_diagnostic(
                XamlDiagnostic::with_line_info(
                    FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
                    XamlDiagnosticSeverity::Error,
                    format!(
                        "Event definition {prop_name} found, but its type {} is not compatible with RoutedEvent.",
                        event_field_type.get_fqn()
                    ),
                    Some(&*node),
                ),
                true,
            )?;
            Ok(node)
        }
    }
}

/// Adds a handler of a routed event: target type is the type of the object the attribute is
/// set on, parameters `[handler delegate type]`, default binder parameters, no custom
/// attributes. Setters compare by identity.
///
/// # What a back end has to do (upstream IL)
///
/// The setter is an optimized one: the back end is expected to use `EmitWithArguments`.
///
/// * `EmitWithArguments` (stack: target object; `arguments[0]` is the handler node; leaves
///   nothing):
///   1. load the static field [`XamlDirectCallAddHandler::event_field`] (the routed event;
///      upstream loads it with `ldfld` over a null reference, which for a static field is a
///      plain static load);
///   2. emit each argument converted to the matching entry of `parameters` (one argument: the
///      handler delegate, for the port a handler closure over the named method);
///   3. load the `int` constant `5` (`RoutingStrategies.Direct | RoutingStrategies.Bubble`);
///   4. load the `int` constant `0` (`handledEventsToo: false`);
///   5. call [`XamlDirectCallAddHandler::add_method`] on the target and discard a non-void
///      result.
///
///   That is `target.AddHandler(EventField, handler, RoutingStrategies.Direct |
///   RoutingStrategies.Bubble, false)`; `add_method` is either
///   `Interactive.AddHandler(RoutedEvent, Delegate, RoutingStrategies, bool)` or the generic
///   `AddHandler<TEventArgs>(RoutedEvent<TEventArgs>, EventHandler<TEventArgs>,
///   RoutingStrategies, bool)` constructed for the event's argument type.
/// * `Emit` (stack: target object, handler): upstream only calls `add_method`, without loading
///   the routed event, the routing strategies or the flag; it is not reachable because the
///   property assignment emitter prefers `EmitWithArguments`.
pub struct XamlDirectCallAddHandler {
    pub event_field: Rc<dyn IXamlField>,
    pub declaring_type: Rc<dyn IXamlType>,
    pub add_method: Rc<dyn IXamlMethod>,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl XamlDirectCallAddHandler {
    pub fn new(
        event_field: Rc<dyn IXamlField>,
        declaring_type: Rc<dyn IXamlType>,
        add_method: Rc<dyn IXamlMethod>,
        routed_event_handler: Rc<dyn IXamlType>,
    ) -> Rc<Self> {
        Rc::new(Self {
            event_field,
            declaring_type,
            add_method,
            binder_parameters: PropertySetterBinderParameters::default(),
            parameters: vec![routed_event_handler],
        })
    }

    /// The `RoutingStrategies` value the upstream IL passes: `Direct | Bubble`.
    pub const ROUTING_STRATEGIES: i32 = 5;
    /// The `handledEventsToo` value the upstream IL passes.
    pub const HANDLED_EVENTS_TOO: bool = false;
}

impl IXamlPropertySetter for XamlDirectCallAddHandler {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.declaring_type.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlDirectCallAddHandler"
    }
}
