//! The binding `{x:Bind}` gives a target.

use super::event::{SubscribeEvent, XBindEvent};
use super::root_node::XBindRootNode;
use super::XBindings;
use crate::XamlLoadError;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::expression_nodes::ExpressionNode;
use ferroui_base::data::core::{BindingExpression, BindingExpressionOptions, TargetTypeConverter};
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingMode, CompiledBindingPath, UpdateSourceTrigger};
use ferroui_base::data::core::WeakValue;
use ferroui_base::interactivity::{Interactive, RoutedEvent};
use ferroui_base::metadata::{MarkupEvent, MarkupValue};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref, UnsetValueType};
use ferroui_controls::TextBox;
use std::rc::Rc;

/// What `{x:Bind}` does with the value of its path on the way to the target and back (step X3):
/// the converter with its parameter and culture, and the values that stand for a path that
/// cannot be read and for a null value. They are the options of the engine's expression, as for
/// `{Binding}`.
#[derive(Clone)]
pub(super) struct XBindValueOptions {
    pub converter: Option<Rc<dyn IValueConverter>>,
    pub converter_parameter: Option<BoxedValue>,
    pub converter_culture: Option<CultureInfo>,
    /// The unset marker for none.
    pub fallback_value: Option<BoxedValue>,
    /// The unset marker for none.
    pub target_null_value: Option<BoxedValue>,
}

impl XBindValueOptions {
    /// Whether the binding has a fallback value: a path that cannot be read then gives the
    /// target the fallback, as WinUI's generated code does, and is not a failure of the binding.
    pub fn has_fallback_value(&self) -> bool {
        !self.fallback_value.as_ref().is_some_and(|value| value.is::<UnsetValueType>())
    }
}

/// The binding of one target property to an `{x:Bind}` path: the compiled path, read from the
/// root source of the [`XBindings`] it belongs to, in the mode the markup gave it.
///
/// Its expression is the ordinary binding expression of the engine over the nodes of the path,
/// after a root node that publishes the root source only while the bindings are initialized.
/// `OneWay` and `TwoWay` follow change notification (registered properties,
/// `INotifyPropertyChanged`); `OneTime` reads when the bindings are initialized or updated.
///
/// The converter, the fallback value and the target null value of the markup are the options of
/// the expression (WinUI's `BindAssignment`, `GetConverterExpression`, `FallbackValueExpression`,
/// `TargetNullValueExpression`): the value of the path goes through `Convert` (and `ConvertBack`
/// when a `TwoWay` binding writes the target back), a path that cannot be read gives the target
/// the fallback value, a null value the target null value.
///
/// On an event (step X4) the path ends with the method the event calls; the compiler gives the
/// binding to [`bind_routed_event`](Self::bind_routed_event) or
/// [`bind_markup_event`](Self::bind_markup_event) instead of binding a property. The handler is
/// subscribed while the bindings of the root are initialized, and it calls the method on the
/// value of the path before it, read when the event is raised.
pub struct XBinding {
    bindings: Rc<XBindings>,
    path: Option<CompiledBindingPath>,
    mode: BindingMode,
    update_source_trigger: UpdateSourceTrigger,
    /// The `x:Phase` of the element of the target (step X5).
    phase: i32,
    options: XBindValueOptions,
}

impl XBinding {
    pub(super) fn new(
        bindings: Rc<XBindings>,
        path: Option<CompiledBindingPath>,
        mode: BindingMode,
        update_source_trigger: UpdateSourceTrigger,
        phase: i32,
        options: XBindValueOptions,
    ) -> Rc<Self> {
        Rc::new(Self { bindings, path, mode, update_source_trigger, phase, options })
    }

    /// The bindings of the root this binding reads from.
    pub fn bindings(&self) -> &Rc<XBindings> {
        &self.bindings
    }

    pub fn path(&self) -> Option<CompiledBindingPath> {
        self.path.clone()
    }

    pub fn mode(&self) -> BindingMode {
        self.mode
    }

    pub fn update_source_trigger(&self) -> UpdateSourceTrigger {
        self.update_source_trigger
    }

    /// The phase in which a container reads the binding (`x:Phase` of the element of its
    /// target); `0` without one.
    pub fn phase(&self) -> i32 {
        self.phase
    }

    /// The event binding of the path, registered with the bindings of its root: `subscribe`
    /// attaches its handler to the event.
    fn bind_event(&self, event: String, subscribe: SubscribeEvent) -> Result<(), XamlLoadError> {
        let path = self.path.clone().ok_or_else(|| {
            XamlLoadError::with_message(format!("x:Bind of the event '{event}' needs the path of a method"))
        })?;
        let handler = path.event_handler().ok_or_else(|| {
            XamlLoadError::with_message(format!(
                "x:Bind of the event '{event}': the path '{path}' does not end with the method the event calls"
            ))
        })?;
        let binding = XBindEvent::new(&self.bindings, event, path, handler, subscribe);
        self.bindings.register_event(binding);
        Ok(())
    }

    /// `target.Event += handler` of a routed event (`Click="{x:Bind OnClick}"`): the handler is
    /// added with the routes of a handler markup adds (direct and bubble, unhandled events) when
    /// the bindings of the root are initialized, and removed when they stop tracking. The target
    /// is held weakly.
    pub fn bind_routed_event(&self, target: &Ref<Interactive>, event: &RoutedEvent) -> Result<(), XamlLoadError> {
        let target = target.downgrade();
        let routed_event = event.clone();
        let subscribe: SubscribeEvent = Box::new(move |handler| {
            let target = target.upgrade().ok_or_else(|| "the target of the event has been released".to_string())?;
            let token = target.add_handler_untyped(&routed_event, handler, Interactive::DEFAULT_ROUTES, false);
            let weak = target.downgrade();
            let routed_event = routed_event.clone();
            Ok(Some(Subscription::new(Disposable::create(move || {
                if let Some(target) = weak.upgrade() {
                    target.remove_handler(&routed_event, token);
                }
            }))))
        });
        self.bind_event(event.name().to_string(), subscribe)
    }

    /// `target.Event += handler` of an event that is not a routed event, which metadata
    /// declares with its subscription (`events:`): subscribed when the bindings of the root
    /// are initialized; the subscription the event returns is released when they stop tracking
    /// (an event without one keeps the handler, which then does nothing). The target is held
    /// weakly.
    pub fn bind_markup_event(&self, event: &'static MarkupEvent, target: MarkupValue) -> Result<(), XamlLoadError> {
        let target = target
            .as_ref()
            .map(WeakValue::new)
            .ok_or_else(|| XamlLoadError::with_message(format!("x:Bind of the event '{}': the target is null", event.name)))?;
        let subscribe: SubscribeEvent = Box::new(move |handler| {
            let target = target.upgrade().ok_or_else(|| "the target of the event has been released".to_string())?;
            let subscription = event.subscribe(Some(target), handler).map_err(|error| error.to_string())?;
            Ok(subscription.map(Subscription::new))
        });
        self.bind_event(event.name.to_string(), subscribe)
    }

    /// The update trigger of a `TwoWay` binding to `target_property` of `target`: `Default` is
    /// `LostFocus` for `TextBox.Text` and `PropertyChanged` for every other property
    /// (`BindAssignment.NeedsLostFocusForTwoWay` of WinUI's compiler).
    fn trigger_for(&self, target: &FerroObject, target_property: &'static FerroProperty) -> UpdateSourceTrigger {
        match self.update_source_trigger {
            UpdateSourceTrigger::Default
                if target.to_ref().cast::<TextBox>().is_some()
                    && target_property.id() == TextBox::text_property().id() =>
            {
                UpdateSourceTrigger::LostFocus
            }
            UpdateSourceTrigger::Default => UpdateSourceTrigger::PropertyChanged,
            trigger => trigger,
        }
    }

    /// The expression of the binding on `target_property` of `target`, registered with the
    /// bindings of its root.
    pub fn try_create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
    ) -> Result<Rc<BindingExpression>, XamlLoadError> {
        let target_property = target_property.ok_or_else(|| {
            XamlLoadError::with_message(
                "x:Bind needs a registered target property or an event",
            )
        })?;
        let path_text = self.path.as_ref().map(ToString::to_string).unwrap_or_default();
        let root = XBindRootNode::new(&self.bindings, path_text, self.options.has_fallback_value(), self.phase, target);
        let mut nodes: Vec<Rc<dyn ExpressionNode>> = vec![root.clone()];
        if let Some(path) = &self.path {
            // A path that starts at a named element is rooted at that element: its first node
            // finds the element by name once the root node publishes the root.
            path.build_expression(&mut nodes);
        }
        root.set_path_nodes(&nodes[1..]);
        let expression = BindingExpression::new(
            Some(Rc::new(target.to_ref())),
            nodes,
            BindingExpressionOptions {
                mode: self.mode,
                update_source_trigger: self.trigger_for(target, target_property),
                target_property: Some(target_property),
                target_type_converter: Some(TargetTypeConverter::get_default_converter()),
                converter: self.options.converter.clone(),
                converter_parameter: self.options.converter_parameter.clone(),
                converter_culture: self.options.converter_culture.clone(),
                fallback_value: self.options.fallback_value.clone(),
                target_null_value: self.options.target_null_value.clone(),
                ..Default::default()
            },
        );
        self.bindings.register(&root);
        Ok(expression)
    }
}

impl BindingBase for XBinding {
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let expression = match self.try_create_instance(target, target_property) {
            Ok(expression) => expression,
            Err(error) => crate::xaml_error_scope::throw(error),
        };
        expression
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

crate::identity_eq!(XBinding);

ferro_markup_type!(class XBinding {
    this: Rc<XBinding>,
    handles: [Rc<XBinding>, Option<Rc<XBinding>>],
    base: Rc<dyn BindingBase>,
});
