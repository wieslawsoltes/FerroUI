use super::expression_nodes::{ExpressionNode, NodeSource};
use super::{
    impl_untyped_binding_expression, ExpressionError, ObservableSink, Publish, TargetTypeConverter,
    UntypedBindingExpression, UntypedBindingExpressionBase, ValueType, ValueTypes, WeakValue,
};
use crate::data::converters::{composite_format, IValueConverter};
use crate::data::{
    BindingChainException, BindingError, BindingErrorType, BindingMode, BindingNotification, BindingOperations,
    BindingPriority, UpdateSourceTrigger,
};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::input::InputElement;
use crate::interactivity::RoutedEventHandlerToken;
use crate::reactive::IDisposable;
use crate::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref, UnsetValueType};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The options of a [`BindingExpression`].
pub struct BindingExpressionOptions {
    /// The amount of time, in milliseconds, to wait before updating the
    /// binding source after the value on the target changes.
    pub delay: i32,
    /// The fallback value; the unset marker for no fallback.
    pub fallback_value: Option<BoxedValue>,
    pub converter: Option<Rc<dyn IValueConverter>>,
    pub converter_parameter: Option<BoxedValue>,
    pub enable_data_validation: bool,
    pub mode: BindingMode,
    pub priority: BindingPriority,
    pub string_format: Option<String>,
    /// The value to use when the binding produces null; the unset marker (or
    /// null) for none.
    pub target_null_value: Option<BoxedValue>,
    pub target_property: Option<&'static FerroProperty>,
    pub target_type_converter: Option<TargetTypeConverter>,
    pub update_source_trigger: UpdateSourceTrigger,
}

impl Default for BindingExpressionOptions {
    fn default() -> Self {
        Self {
            delay: 0,
            fallback_value: Some(FerroProperty::unset_value()),
            converter: None,
            converter_parameter: None,
            enable_data_validation: false,
            mode: BindingMode::OneWay,
            priority: BindingPriority::LocalValue,
            string_format: None,
            target_null_value: Some(FerroProperty::unset_value()),
            target_property: None,
            target_type_converter: None,
            update_source_trigger: UpdateSourceTrigger::PropertyChanged,
        }
    }
}

/// Uncommonly used fields are separated out to reduce memory usage.
struct UncommonFields {
    delay: Duration,
    delay_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    delay_tick: RefCell<Option<Rc<dyn IDisposable>>>,
    converter: Option<Rc<dyn IValueConverter>>,
    converter_parameter: Option<BoxedValue>,
    fallback_value: Option<BoxedValue>,
    string_format: Option<String>,
    target_null_value: Option<BoxedValue>,
    update_source_trigger: UpdateSourceTrigger,
}

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|v| v.is::<UnsetValueType>())
}

/// A binding expression which accepts and produces (possibly boxed) untyped
/// values: a source, a path of expression nodes read from it, and the
/// conversion of the result to the target.
pub struct BindingExpression {
    this: Weak<BindingExpression>,
    base: UntypedBindingExpressionBase,
    /// `None` when the binding source is null.
    source: Option<WeakValue>,
    mode: BindingMode,
    nodes: Vec<Rc<dyn ExpressionNode>>,
    target_type_converter: Option<TargetTypeConverter>,
    uncommon: Option<Box<UncommonFields>>,
    update_target_depth: Cell<u32>,
    should_update_one_time_binding_target: Cell<bool>,
    target_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    lost_focus_token: RefCell<Option<RoutedEventHandlerToken>>,
}

impl BindingExpression {
    /// Creates a binding expression.
    ///
    /// `source` is the object from which the value is read (null is `None`);
    /// `nodes` represent the binding path.
    pub fn new(
        source: Option<BoxedValue>,
        nodes: Vec<Rc<dyn ExpressionNode>>,
        options: BindingExpressionOptions,
    ) -> Rc<Self> {
        if options.mode == BindingMode::Default {
            panic!("Binding mode cannot be Default.");
        }
        if options.update_source_trigger == UpdateSourceTrigger::Default {
            panic!("UpdateSourceTrigger cannot be Default.");
        }
        let source = source.filter(|s| !s.is::<UnsetValueType>());
        let string_format = options.string_format.filter(|s| !s.trim().is_empty());
        let has_target_null_value = options.target_null_value.as_ref().is_some_and(|v| !v.is::<UnsetValueType>());

        let uncommon = if options.delay > 0
            || options.converter.is_some()
            || options.converter_parameter.is_some()
            || !is_unset(&options.fallback_value)
            || string_format.is_some()
            || has_target_null_value
            || options.update_source_trigger != UpdateSourceTrigger::PropertyChanged
        {
            Some(Box::new(UncommonFields {
                delay: Duration::from_millis(options.delay.max(0) as u64),
                delay_timer: RefCell::new(None),
                delay_tick: RefCell::new(None),
                converter: options.converter,
                converter_parameter: options.converter_parameter,
                fallback_value: options.fallback_value,
                string_format: string_format.map(|s| if s.contains('{') { s } else { format!("{{0:{s}}}") }),
                target_null_value: if has_target_null_value {
                    options.target_null_value
                } else {
                    Some(FerroProperty::unset_value())
                },
                update_source_trigger: options.update_source_trigger,
            }))
        } else {
            None
        };

        let mode = options.mode;
        let this = Rc::new_cyclic(|this: &Weak<BindingExpression>| BindingExpression {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(
                this.clone(),
                options.priority,
                options.target_property,
                options.enable_data_validation,
            ),
            source: source.as_ref().map(WeakValue::new),
            mode,
            nodes,
            target_type_converter: options.target_type_converter,
            uncommon,
            update_target_depth: Cell::new(0),
            should_update_one_time_binding_target: Cell::new(mode == BindingMode::OneTime),
            target_subscription: RefCell::new(None),
            lost_focus_token: RefCell::new(None),
        });

        let mut leaf_accessor = None;
        for (i, node) in this.nodes.iter().enumerate() {
            node.set_owner(Rc::downgrade(&this), i);
            if let Some(n) = node.as_property_accessor_node() {
                leaf_accessor = Some(n);
            }
        }
        if options.enable_data_validation {
            if let Some(leaf) = leaf_accessor {
                leaf.enable_data_validation();
            }
        }
        this
    }

    /// The type of the value accepted by the binding source, if it can be
    /// written.
    pub fn source_type(&self) -> Option<ValueType> {
        self.leaf_node().and_then(|n| n.as_settable().and_then(|s| s.value_type()))
    }

    pub fn converter(&self) -> Option<&Rc<dyn IValueConverter>> {
        self.uncommon.as_ref().and_then(|u| u.converter.as_ref())
    }

    pub fn converter_parameter(&self) -> Option<&BoxedValue> {
        self.uncommon.as_ref().and_then(|u| u.converter_parameter.as_ref())
    }

    /// The fallback value; the unset marker if there is none.
    pub fn fallback_value(&self) -> Option<BoxedValue> {
        match &self.uncommon {
            Some(u) => u.fallback_value.clone(),
            None => Some(FerroProperty::unset_value()),
        }
    }

    pub fn leaf_node(&self) -> Option<&Rc<dyn ExpressionNode>> {
        self.nodes.last()
    }

    pub fn nodes(&self) -> &[Rc<dyn ExpressionNode>] {
        &self.nodes
    }

    pub fn mode(&self) -> BindingMode {
        self.mode
    }

    pub fn string_format(&self) -> Option<&str> {
        self.uncommon.as_ref().and_then(|u| u.string_format.as_deref())
    }

    /// The target null value; the unset marker if there is none.
    pub fn target_null_value(&self) -> Option<BoxedValue> {
        match &self.uncommon {
            Some(u) => u.target_null_value.clone(),
            None => Some(FerroProperty::unset_value()),
        }
    }

    /// The amount of time to wait before updating the binding source after
    /// the value on the target changes.
    pub fn delay(&self) -> Duration {
        self.uncommon.as_ref().map_or(Duration::ZERO, |u| u.delay)
    }

    fn stop_delay_timer(&self) {
        if let Some(u) = &self.uncommon {
            let timer = u.delay_timer.borrow().clone();
            if let Some(timer) = timer {
                timer.stop();
            }
        }
    }

    pub fn update_source_trigger(&self) -> UpdateSourceTrigger {
        self.uncommon.as_ref().map_or(UpdateSourceTrigger::PropertyChanged, |u| u.update_source_trigger)
    }

    /// The current error state of the binding expression.
    pub fn error_type(&self) -> BindingErrorType {
        self.base.error_type()
    }

    /// Produces an observable subject over the expression, see
    /// [`ObservableSink`].
    pub fn to_observable(self: &Rc<Self>, target: Option<&FerroObject>) -> Rc<ObservableSink> {
        ObservableSink::create(self, target)
    }

    /// Called by a node belonging to this binding when its value changes.
    pub(crate) fn on_node_value_changed(
        &self,
        node_index: usize,
        value: Option<BoxedValue>,
        data_validation_error: Option<&BindingError>,
    ) {
        debug_assert!(node_index < self.nodes.len());
        let count = self.nodes.len();

        if node_index == count - 1 {
            if self.mode == BindingMode::OneTime {
                // In OneTime mode, only changing the data context updates the binding.
                if !self.should_update_one_time_binding_target.get() && !self.nodes[node_index].is_data_context_node() {
                    return;
                }
                self.should_update_one_time_binding_target.set(false);
            }

            let error = data_validation_error
                .map(|e| ExpressionError::new(e.clone(), BindingErrorType::DataValidationError));

            // The leaf node has changed. If the binding mode is not
            // OneWayToSource, publish the value to the target.
            if self.mode != BindingMode::OneWayToSource {
                // An explicit target update must reapply the source value
                // even if this expression already has it cached: a two-way
                // target may contain an uncommitted local value.
                let force_update = self.mode == BindingMode::OneWay || self.update_target_depth.get() > 0;
                self.convert_and_publish_value(value, error, force_update);
            } else if self.base.is_data_validation_enabled() {
                // In OneWayToSource mode the value must not be published to
                // the target, but any data validation error produced when
                // writing to the source still has to be published (or
                // cleared) so that it can be displayed.
                self.base.publish_value(Publish::Unchanged, error, false);
            }
        } else if self.mode == BindingMode::OneWayToSource && node_index + 2 == count && value.is_some() {
            // When the binding mode is OneWayToSource, the value is written
            // to the source when the object holding the source property
            // changes; this is the node before the leaf node. First update
            // the leaf node's source, then write the value to its property.
            self.nodes[node_index + 1].set_source(NodeSource::Value(value), data_validation_error);
            self.write_target_value_to_source();
        } else {
            if self.mode == BindingMode::OneTime && self.nodes[node_index].is_data_context_node() {
                self.should_update_one_time_binding_target.set(true);
            }
            self.nodes[node_index + 1].set_source(NodeSource::Value(value), data_validation_error);
        }
    }

    /// Called by a node belonging to this binding when a null-conditional
    /// operator applied to it encounters a null source: the remainder of the
    /// path is short-circuited and null is published as the binding's value.
    pub(crate) fn on_node_null_short_circuit(&self, node_index: usize) {
        for node in &self.nodes[node_index + 1..] {
            node.propagate_null_short_circuit_value();
        }
        // Publish the null value as if it were produced by the leaf node.
        self.on_node_value_changed(self.nodes.len() - 1, None, None);
    }

    /// Called by a node belonging to this binding when an error occurs
    /// reading its value. `node_index` is -1 if the source is null.
    pub(crate) fn on_node_error(&self, node_index: isize, error: &str) {
        // Set the source of all nodes after the one that errored to unset.
        // This needs to be done for each node individually because setting
        // the source to unset does not notify the binding.
        for node in &self.nodes[(node_index + 1) as usize..] {
            node.set_source(NodeSource::Unset, None);
        }

        if self.mode == BindingMode::OneWayToSource {
            return;
        }

        let error_point = self.calculate_error_point(node_index);
        if let Some(target) = self.should_log_error() {
            self.log_at(&target, error, &error_point, LogEventLevel::Warning);
        }

        // Clear the current value and publish the error.
        let binding_error = ExpressionError::new(
            BindingError::new(BindingChainException::with_expression(error, self.description(), error_point)),
            BindingErrorType::Error,
        );
        self.convert_and_publish_value(Some(FerroProperty::unset_value()), Some(binding_error), false);
    }

    pub(crate) fn on_data_validation_error(&self, error: BindingError) {
        let binding_error = ExpressionError::new(error, BindingErrorType::DataValidationError);
        self.base.publish_value(Publish::Unchanged, Some(binding_error), false);
    }

    fn calculate_error_point(&self, node_index: isize) -> String {
        // A string describing the binding chain up to the node that errored.
        let mut result = String::new();
        if node_index >= 0 {
            self.nodes[node_index as usize].build_string(&mut result);
        } else {
            result.push_str("(source)");
        }
        result
    }

    fn log_at(&self, target: &FerroObject, error: &str, error_point: &str, level: LogEventLevel) {
        let Some(log) = Logger::try_get(level, LogArea::BINDING) else { return };
        let property = self.base.target_property().map_or("(unknown)", |p| p.name());
        let description = self.description();
        log.log_with_values(
            Some(target as &dyn Any),
            "An error occurred binding {Property} to {Expression} at {ExpressionErrorPoint}: {Message}",
            &[&property, &description, &error_point, &error],
        );
    }

    fn convert_and_publish_value(
        &self,
        value: Option<BoxedValue>,
        error: Option<ExpressionError>,
        force_update: bool,
    ) {
        let mut value = value;
        let mut error = error;
        let mut is_target_null_value = false;
        let target_type = self.base.target_type();

        // All values other than the unset and do-nothing markers are passed
        // to the converter.
        if let Some(converter) = self.converter() {
            if !is_unset(&value) && !BindingOperations::is_do_nothing(value.as_ref()) {
                value = self.base.convert(
                    self.should_log_error(),
                    &|| self.description(),
                    &**converter,
                    self.converter_parameter(),
                    value.as_ref(),
                    target_type,
                    &mut error,
                );
                // A converter may report its result as a notification.
                let converted = value.clone();
                if let Some(n) = converted.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()) {
                    if let Some(e) = n.error() {
                        error = Some(ExpressionError::new(e, n.error_type()));
                    }
                    value = if n.has_value() { n.value() } else { Some(FerroProperty::unset_value()) };
                }
            }
        }

        // Check this here as the converter may return the do-nothing marker.
        if BindingOperations::is_do_nothing(value.as_ref()) {
            return;
        }

        // The target null value only applies when the value is null: the
        // unset marker indicates a binding error, for which it is not used.
        let target_null_value = self.target_null_value();
        if value.is_none() && !is_unset(&target_null_value) {
            value = self.convert_fallback(target_null_value, "TargetNullValue");
            is_target_null_value = true;
        }

        // If we have a value, try to convert it to the target type.
        if !is_unset(&value) {
            match self.string_format() {
                Some(string_format)
                    if (target_type.is_object() || target_type.is_string()) && !is_target_null_value =>
                {
                    // The string format applies if the target can accept a
                    // string and the value isn't the target null value.
                    value = match composite_format::format_values(string_format, std::slice::from_ref(&value)) {
                        Ok(s) => self.convert_from(Some(Rc::new(s)), &mut error),
                        Err(e) => {
                            error = Some(ExpressionError::new(BindingError::new(e), BindingErrorType::Error));
                            Some(FerroProperty::unset_value())
                        }
                    };
                }
                _ => value = self.convert_from(value, &mut error),
            }
        }

        // The fallback value applies if the result from the binding,
        // converter or target type converter is the unset marker.
        let fallback_value = self.fallback_value();
        if is_unset(&value) && !is_unset(&fallback_value) {
            value = self.convert_fallback(fallback_value, "FallbackValue");
        }

        self.base.publish_value(Publish::Value(value), error, force_update);
    }

    fn write_target_value_to_source(&self) {
        debug_assert!(matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource));
        self.stop_delay_timer();
        if let (Some(target), Some(property)) = (self.base.try_get_target(), self.base.target_property()) {
            let value = ValueTypes::normalize(target.get_value_untyped(property));
            self.write_value_to_source(value);
        }
    }

    fn convert_fallback(&self, fallback: Option<BoxedValue>, fallback_name: &str) -> Option<BoxedValue> {
        let target_type = self.base.target_type();
        if is_unset(&fallback) || (self.base.target_property().is_none() && target_type.is_object()) {
            return fallback;
        }
        if let Some(result) = ValueTypes::try_convert(fallback.as_ref(), target_type) {
            return result;
        }
        if let Some(target) = self.base.try_get_target() {
            self.base.log(
                &target,
                &self.description(),
                &format!(
                    "Could not convert {fallback_name} '{}' to '{target_type}'.",
                    ValueTypes::to_display_string(fallback.as_ref())
                ),
                LogEventLevel::Error,
            );
        }
        Some(FerroProperty::unset_value())
    }

    /// Converts a produced value to the target type. Without a target
    /// property (an expression observed directly) values pass through.
    fn convert_from(&self, value: Option<BoxedValue>, error: &mut Option<ExpressionError>) -> Option<BoxedValue> {
        let target_type = self.base.target_type();
        if self.base.target_property().is_none() && target_type.is_object() {
            return value;
        }
        // The converter of string-path bindings knows conversions the table
        // of value types does not (the delegate of a method to a command).
        let converted = match &self.target_type_converter {
            Some(converter) => converter.try_convert(value.as_ref(), target_type),
            None => ValueTypes::try_convert(value.as_ref(), target_type),
        };
        if let Some(result) = converted {
            return result;
        }
        let message = format!(
            "Could not convert '{}' ({}) to '{}'.",
            ValueTypes::to_display_string(value.as_ref()),
            value.as_ref().map_or("null", |v| (**v).type_name()),
            target_type
        );
        if let Some(target) = self.should_log_error() {
            self.base.log(&target, &self.description(), &message, LogEventLevel::Warning);
        }
        *error = Some(ExpressionError::new(BindingError::message(message), BindingErrorType::Error));
        Some(FerroProperty::unset_value())
    }

    /// Runs the converter on a value on its way to the source. Returns false
    /// if the converter reported an error, in which case the value is not
    /// written.
    fn try_convert_back(&self, value_type: ValueType, value: &mut Option<BoxedValue>) -> bool {
        let Some(converter) = self.converter() else { return true };
        if is_unset(value) || BindingOperations::is_do_nothing(value.as_ref()) {
            return true;
        }
        *value = self.base.convert_back(
            self.should_log_error(),
            &|| self.description(),
            &**converter,
            self.converter_parameter(),
            value.as_ref(),
            value_type,
        );
        let converted = value.clone();
        if let Some(notification) = converted.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()) {
            if let Some(error) = notification.error() {
                match notification.error_type() {
                    BindingErrorType::DataValidationError => {
                        if self.base.is_data_validation_enabled() {
                            self.on_data_validation_error(error);
                        }
                    }
                    _ => {
                        if let Some(target) = self.should_log_error() {
                            self.base.log(&target, &self.description(), &error.to_string(), LogEventLevel::Warning);
                        }
                        self.base.publish_value(
                            Publish::Unchanged,
                            Some(ExpressionError::new(error, BindingErrorType::Error)),
                            false,
                        );
                    }
                }
                return false;
            }
            *value = notification.value();
        }
        true
    }

    fn on_target_property_changed(&self) {
        debug_assert!(matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource));
        let delay = self.delay();
        let Some(uncommon) = self.uncommon.as_ref().filter(|_| !delay.is_zero()) else {
            // The value is read from the target object instead of the change
            // notification because it may have changed again in between.
            self.write_target_value_to_source();
            return;
        };

        let existing = uncommon.delay_timer.borrow().clone();
        let timer = match existing {
            Some(timer) => {
                timer.stop();
                timer
            }
            None => {
                let timer = DispatcherTimer::with_interval(
                    delay,
                    DispatcherPriority::DEFAULT,
                    &Dispatcher::current_dispatcher(),
                );
                let weak = self.this.clone();
                let tick = timer.tick(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.write_target_value_to_source();
                    }
                });
                uncommon.delay_tick.replace(Some(tick));
                uncommon.delay_timer.replace(Some(timer.clone()));
                timer
            }
        };
        timer.start();
    }
}

impl UntypedBindingExpression for BindingExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        let mut b = String::new();
        if let Some(leaf) = self.leaf_node() {
            leaf.build_string_with_nodes(&mut b, &self.nodes);
        }
        b
    }

    fn update_source_core(&self) {
        if matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource) {
            self.write_target_value_to_source();
        }
    }

    fn update_target_core(&self) {
        let Some(first) = self.nodes.first() else { return };
        let source = first.state().source();
        self.update_target_depth.set(self.update_target_depth.get() + 1);
        for node in &self.nodes {
            node.set_source(NodeSource::Unset, None);
        }
        // A dead or null source reads as null, as in the managed
        // implementation.
        first.set_source(NodeSource::Value(source), None);
        self.update_target_depth.set(self.update_target_depth.get() - 1);
    }

    fn write_value_to_source(&self, value: Option<BoxedValue>) -> bool {
        self.stop_delay_timer();
        let mut value = value;
        let Some(leaf) = self.leaf_node() else { return false };
        let Some(setter) = leaf.as_settable() else { return false };
        let Some(type_) = setter.value_type() else { return false };

        // Invoke any converter on the value before writing it to the source.
        // If the converter returns an error the value is not written.
        if !self.try_convert_back(type_, &mut value) {
            return false;
        }

        // A converter may return the do-nothing marker.
        if BindingOperations::is_do_nothing(value.as_ref()) {
            return true;
        }

        // Use the target type converter to convert the value to the source
        // type if necessary.
        if let Some(converter) = &self.target_type_converter {
            if let Some(converted) = converter.try_convert(value.as_ref(), type_) {
                value = converted;
            } else if !is_unset(&self.fallback_value()) {
                value = self.fallback_value();
            } else if self.base.is_data_validation_enabled() {
                self.on_data_validation_error(BindingError::message(format!(
                    "Could not convert '{}' ({}) to {}.",
                    ValueTypes::to_display_string(value.as_ref()),
                    value.as_ref().map_or("null", |v| (**v).type_name()),
                    type_
                )));
                return false;
            } else {
                return false;
            }
        }

        // Don't set the value if it's unchanged. If there is a binding
        // error, the value still has to be set in order to clear the error.
        let current = leaf.state().value();
        let current = current.and_then(ValueTypes::normalize);
        let new = value.clone().and_then(ValueTypes::normalize);
        if ValueTypes::identity_equals(current.as_ref(), new.as_ref())
            && self.base.error_type() == BindingErrorType::None
        {
            return true;
        }

        setter.write_value_to_source(value.as_ref(), &self.nodes).unwrap_or(false)
    }

    fn should_log_error(&self) -> Option<Ref<FerroObject>> {
        let target = self.base.try_get_target()?;
        if let Some(first) = self.nodes.first() {
            if let Some(source_node) = first.as_source_node() {
                if !source_node.should_log_errors(first.state(), &target) {
                    return None;
                }
            }
        }
        Some(target)
    }

    fn start_core(&self) {
        let source = self.source.as_ref().and_then(WeakValue::upgrade);
        let Some(source) = source else {
            self.on_node_error(-1, "Binding Source is null.");
            return;
        };

        match self.nodes.first() {
            Some(first) => first.set_source(NodeSource::Value(Some(source)), None),
            None => self.convert_and_publish_value(Some(source), None, false),
        }

        if matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource) {
            if let (Some(target), Some(property)) = (self.base.try_get_target(), self.base.target_property()) {
                if self.update_source_trigger() == UpdateSourceTrigger::PropertyChanged {
                    let weak = self.this.clone();
                    let id = property.id();
                    let subscription = target.property_changed(move |e| {
                        if e.property().id() == id {
                            if let Some(this) = weak.upgrade() {
                                this.on_target_property_changed();
                            }
                        }
                    });
                    self.target_subscription.replace(Some(subscription));
                } else if self.update_source_trigger() == UpdateSourceTrigger::LostFocus {
                    if let Some(element) = target.downcast_ref::<InputElement>() {
                        let weak = self.this.clone();
                        let token = element.add_handler(InputElement::lost_focus_event(), move |_, _| {
                            if let Some(this) = weak.upgrade() {
                                this.write_target_value_to_source();
                            }
                        });
                        self.lost_focus_token.replace(Some(token));
                    }
                }
            }
        }
    }

    fn stop_core(&self) {
        self.stop_delay_timer();
        let token = self.lost_focus_token.borrow_mut().take();
        if let Some(token) = token {
            if let Some(target) = self.base.try_get_target() {
                if let Some(element) = target.downcast_ref::<InputElement>() {
                    element.remove_handler(InputElement::lost_focus_event(), token);
                }
            }
        }
        for node in &self.nodes {
            node.set_source(NodeSource::Unset, None);
        }
        let subscription = self.target_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }
}

impl_untyped_binding_expression!(BindingExpression);

impl Drop for BindingExpression {
    fn drop(&mut self) {
        if let Some(s) = self.target_subscription.get_mut().take() {
            s.dispose();
        }
    }
}
