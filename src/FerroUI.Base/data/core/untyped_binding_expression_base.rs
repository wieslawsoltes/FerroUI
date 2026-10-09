use super::{ExpressionError, IBindingExpressionSink, SinkRef, ValueType, ValueTypes};
use crate::data::converters::IValueConverter;
use crate::data::core::plugins::property_value_type;
use crate::data::{
    BindingError, BindingErrorType, BindingExpressionBase, BindingNotification, BindingPriority, BindingValueType,
};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::property_store::ImmediateValueFrame;
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver};
use crate::utilities::CultureInfo;
use crate::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, UnsetValueType, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A value to publish to the target of a binding expression.
pub enum Publish {
    /// Leave the current value untouched (only the error state changes).
    Unchanged,
    Value(Option<BoxedValue>),
}

/// The state shared by untyped binding expressions.
pub struct UntypedBindingExpressionBase {
    this: Weak<dyn BindingExpressionBase>,
    default_priority: BindingPriority,
    priority: Cell<BindingPriority>,
    target_property: Cell<Option<&'static FerroProperty>>,
    target_type: Cell<ValueType>,
    is_data_validation_enabled: Cell<bool>,
    default_value: RefCell<Option<Option<BoxedValue>>>,
    error: RefCell<Option<ExpressionError>>,
    frame: RefCell<Option<Weak<ImmediateValueFrame>>>,
    is_running: Cell<bool>,
    produce_value: Cell<bool>,
    sink: RefCell<Option<SinkRef>>,
    observable: RefCell<Weak<ObservableSink>>,
    target: RefCell<Option<WeakRef<FerroObject>>>,
    value: RefCell<Option<BoxedValue>>,
}

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|v| v.is::<UnsetValueType>())
}

impl UntypedBindingExpressionBase {
    /// Creates the state. `this` is the expression the state belongs to.
    pub fn new(
        this: Weak<dyn BindingExpressionBase>,
        default_priority: BindingPriority,
        target_property: Option<&'static FerroProperty>,
        is_data_validation_enabled: bool,
    ) -> Self {
        crate::perf_count!(BindingExpressionsCreated);
        Self {
            this,
            default_priority,
            priority: Cell::new(default_priority),
            target_property: Cell::new(target_property),
            target_type: Cell::new(target_property.map_or_else(ValueType::object, property_value_type)),
            is_data_validation_enabled: Cell::new(is_data_validation_enabled),
            default_value: RefCell::new(None),
            error: RefCell::new(None),
            frame: RefCell::new(None),
            is_running: Cell::new(false),
            produce_value: Cell::new(true),
            sink: RefCell::new(None),
            observable: RefCell::new(Weak::new()),
            target: RefCell::new(None),
            value: RefCell::new(Some(FerroProperty::unset_value())),
        }
    }

    pub fn priority(&self) -> BindingPriority {
        self.priority.get()
    }

    pub fn default_priority(&self) -> BindingPriority {
        self.default_priority
    }

    pub fn target_property(&self) -> Option<&'static FerroProperty> {
        self.target_property.get()
    }

    pub fn is_data_validation_enabled(&self) -> bool {
        self.is_data_validation_enabled.get()
    }

    /// The current error state of the binding expression.
    pub fn error_type(&self) -> BindingErrorType {
        self.error.borrow().as_ref().map_or(BindingErrorType::None, |e| e.error_type)
    }

    pub(crate) fn error(&self) -> Option<ExpressionError> {
        self.error.borrow().clone()
    }

    /// Whether the binding expression is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.get()
    }

    /// The type that values produced by the expression are converted to.
    pub fn target_type(&self) -> ValueType {
        self.target_type.get()
    }

    /// The current value of the binding expression, or the unset marker if
    /// the binding was unable to read a value. Panics if the expression has
    /// not been started.
    pub fn get_value(&self) -> Option<BoxedValue> {
        if !self.is_running() {
            panic!("BindingExpression has not been started.");
        }
        self.value.borrow().clone()
    }

    /// The current value of the binding expression or the default value of
    /// the target property.
    pub fn get_value_or_default(&self) -> Option<BoxedValue> {
        let result = self.get_value();
        if is_unset(&result) {
            self.get_cached_default_value()
        } else {
            result
        }
    }

    fn get_cached_default_value(&self) -> Option<BoxedValue> {
        if let Some(value) = &*self.default_value.borrow() {
            return value.clone();
        }
        if let (Some(property), Some(target)) = (self.target_property(), self.try_get_target()) {
            // The unset value of a direct property, the default value of a
            // styled property.
            let value = Some(property.routes().route_get_default_value(target.get_type()));
            self.default_value.replace(Some(value.clone()));
            return value;
        }
        Some(FerroProperty::unset_value())
    }

    #[doc(hidden)]
    pub fn data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        if !self.is_data_validation_enabled() {
            return None;
        }
        Some(match &*self.error.borrow() {
            Some(error) => (
                match error.error_type {
                    BindingErrorType::Error => BindingValueType::BINDING_ERROR,
                    BindingErrorType::DataValidationError => BindingValueType::DATA_VALIDATION_ERROR,
                    BindingErrorType::None => panic!("Invalid BindingErrorType."),
                },
                Some(error.exception.clone()),
            ),
            None => (BindingValueType::VALUE, None),
        })
    }

    #[doc(hidden)]
    pub fn attach(
        &self,
        sink: SinkRef,
        frame: Option<Weak<ImmediateValueFrame>>,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        priority: BindingPriority,
    ) {
        if self.sink.borrow().is_some() {
            panic!("BindingExpression was already attached.");
        }
        if let (Some(existing), Some(new)) = (self.target_property(), target_property) {
            if existing.id() != new.id() {
                panic!("BindingExpression was already attached to a different property.");
            }
        }
        self.sink.replace(Some(sink));
        self.frame.replace(frame);
        self.target.replace(Some(target.to_weak()));
        self.target_property.set(target_property);
        self.target_type.set(target_property.map_or_else(ValueType::object, property_value_type));
        self.priority.set(priority);
    }

    /// The target of the binding expression, if it is attached and alive.
    pub fn try_get_target(&self) -> Option<Ref<FerroObject>> {
        self.target.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Converts a value using a value converter, logging a warning if the
    /// converter fails; the result is then the unset marker and `error`
    /// holds the failure.
    ///
    /// `log_target` is the target to log the failure against, if it should
    /// be logged (the `should_log_error` of the expression). It is asked
    /// only when the converter has failed, and after it has.
    pub fn convert(
        &self,
        log_target: &dyn Fn() -> Option<Ref<FerroObject>>,
        description: &dyn Fn() -> String,
        converter: &dyn IValueConverter,
        converter_culture: Option<&CultureInfo>,
        converter_parameter: Option<&BoxedValue>,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        error: &mut Option<ExpressionError>,
    ) -> Option<BoxedValue> {
        let culture = converter_culture.cloned().unwrap_or_else(CultureInfo::current_culture);
        match converter.convert(value, target_type, converter_parameter, &culture) {
            Ok(v) => v,
            Err(e) => {
                let message = Self::conversion_message(value, target_type);
                if let Some(target) = log_target() {
                    self.log(&target, &description(), &format!("{message}: {e}"), LogEventLevel::Warning);
                }
                *error = Some(ExpressionError::new(
                    BindingError::message(format!("{message}.")),
                    BindingErrorType::Error,
                ));
                Some(FerroProperty::unset_value())
            }
        }
    }

    /// Converts a value using a value converter's `convert_back`, logging a
    /// warning if the converter fails; the result is then the unset marker.
    ///
    /// `log_target` is asked as in [`convert`](Self::convert): only when the
    /// converter has failed.
    pub fn convert_back(
        &self,
        log_target: &dyn Fn() -> Option<Ref<FerroObject>>,
        description: &dyn Fn() -> String,
        converter: &dyn IValueConverter,
        converter_culture: Option<&CultureInfo>,
        converter_parameter: Option<&BoxedValue>,
        value: Option<&BoxedValue>,
        target_type: ValueType,
    ) -> Option<BoxedValue> {
        let culture = converter_culture.cloned().unwrap_or_else(CultureInfo::current_culture);
        match converter.convert_back(value, target_type, converter_parameter, &culture) {
            Ok(v) => v,
            Err(e) => {
                let message = Self::conversion_message(value, target_type);
                if let Some(target) = log_target() {
                    self.log(&target, &description(), &format!("{message}: {e}"), LogEventLevel::Warning);
                }
                Some(FerroProperty::unset_value())
            }
        }
    }

    fn conversion_message(value: Option<&BoxedValue>, target_type: ValueType) -> String {
        format!(
            "Could not convert '{}' ({}) to '{}' using the value converter",
            ValueTypes::to_display_string(value),
            value.map_or("null", |v| (**v).type_name()),
            target_type
        )
    }

    /// Logs a binding error.
    pub fn log(&self, target: &FerroObject, description: &str, error: &str, level: LogEventLevel) {
        let Some(log) = Logger::try_get(level, LogArea::BINDING) else { return };
        let property = self.target_property().map_or("(unknown)", |p| p.name());
        log.log_with_values(
            Some(target as &dyn Any),
            "An error occurred binding {Property} to {Expression}: {Message}",
            &[&property, &description, &error],
        );
    }

    /// Publishes a new value and/or error state to the target.
    pub fn publish_value(&self, value: Publish, error: Option<ExpressionError>, force_update: bool) {
        if !self.is_running() {
            return;
        }
        crate::perf_count!(BindingValuesPublished);

        // When binding to the data context and the expression results in a
        // binding error, the expression produces null rather than the unset
        // marker in order to not propagate incorrect data contexts from
        // parent elements while things are being set up.
        let mut value = value;
        if let Publish::Value(v) = &value {
            let is_data_context = self
                .target_property()
                .is_some_and(|p| p.id() == StyledElement::data_context_property().id());
            if is_data_context
                && is_unset(v)
                && error.as_ref().is_some_and(|e| e.error_type == BindingErrorType::Error)
            {
                value = Publish::Value(Some(Rc::new(Option::<BoxedValue>::None)));
            }
        }

        let has_value_changed = match &value {
            Publish::Unchanged => force_update,
            Publish::Value(v) => {
                force_update || !ValueTypes::identity_equals(v.as_ref(), self.value.borrow().as_ref())
            }
        };
        let has_error_changed = error.is_some() || self.error.borrow().is_some();

        if has_value_changed {
            if let Publish::Value(v) = value {
                self.value.replace(v);
            }
        }
        self.error.replace(error);

        if !self.produce_value.get() {
            return;
        }
        let sink = self.sink.borrow().clone();
        let (Some(sink), Some(this)) = (sink, self.this.upgrade()) else { return };

        // Expressions, their sources and their targets belong to one thread,
        // so the sink is always notified directly.
        sink.on_changed(&this, has_value_changed, has_error_changed);
    }
}

/// The overridable members of an untyped binding expression, together with
/// the operations built on them.
pub trait UntypedBindingExpression: 'static {
    fn base(&self) -> &UntypedBindingExpressionBase;

    /// A description of the binding expression.
    fn description(&self) -> String;

    /// Starts the binding expression. Do not call directly; call
    /// [`start`](Self::start).
    fn start_core(&self);

    /// Stops the binding expression. Do not call directly; call
    /// [`stop`](Self::stop).
    fn stop_core(&self);

    /// Writes the specified value to the binding source if possible. Returns
    /// true if the value could be written.
    fn write_value_to_source(&self, _value: Option<BoxedValue>) -> bool {
        false
    }

    /// The target to log errors against, if an error should be logged given
    /// the current state of the binding expression.
    fn should_log_error(&self) -> Option<Ref<FerroObject>> {
        self.base().try_get_target()
    }

    fn update_source_core(&self) {}

    fn update_target_core(&self) {}

    /// Starts the binding expression.
    fn start(&self, produce_value: bool) {
        let base = self.base();
        if base.is_running.get() {
            return;
        }
        base.is_running.set(true);
        base.produce_value.set(produce_value);
        self.start_core();
        base.produce_value.set(true);
    }

    /// Stops the binding expression.
    fn stop(&self) {
        let base = self.base();
        if !base.is_running.get() {
            return;
        }
        self.stop_core();
        base.is_running.set(false);
        base.value.replace(Some(FerroProperty::unset_value()));
    }

    /// Terminates the binding.
    fn dispose_expression(&self) {
        let base = self.base();
        if base.sink.borrow().is_none() {
            return;
        }
        self.stop();
        let sink = base.sink.borrow_mut().take();
        let frame = base.frame.borrow_mut().take();
        let this = base.this.upgrade();
        if let (Some(sink), Some(this)) = (sink, &this) {
            sink.on_completed(this);
        }
        if let (Some(frame), Some(property)) = (frame.and_then(|f| f.upgrade()), base.target_property()) {
            frame.on_entry_disposed(property);
        }
    }

    /// Logs a binding error against the target, if there is one.
    fn log_error(&self, error: &str, level: LogEventLevel) {
        if let Some(target) = self.base().try_get_target() {
            self.base().log(&target, &self.description(), error, level);
        }
    }
}

/// Implements the value entry, disposable and binding expression traits for
/// a type that implements [`UntypedBindingExpression`]: what makes the type
/// a binding expression class deriving from
/// [`UntypedBindingExpressionBase`].
///
/// The type holds the base (`UntypedBindingExpressionBase::new(this, ..)`,
/// created with `Rc::new_cyclic` so that the base knows its expression),
/// returns it from [`UntypedBindingExpression::base`], and overrides
/// `description`, `start_core`, `stop_core` and, where needed, the other
/// members of that trait. It can be defined in any crate:
///
/// ```ignore
/// pub struct ConstantExpression { base: UntypedBindingExpressionBase, value: BoxedValue }
///
/// impl UntypedBindingExpression for ConstantExpression {
///     fn base(&self) -> &UntypedBindingExpressionBase { &self.base }
///     fn description(&self) -> String { "Constant".to_string() }
///     fn start_core(&self) { self.base.publish_value(Publish::Value(Some(self.value.clone())), None, false) }
///     fn stop_core(&self) {}
/// }
///
/// ferroui_base::impl_untyped_binding_expression!(ConstantExpression);
/// ```
#[macro_export]
macro_rules! impl_untyped_binding_expression {
    ($ty:ty) => {
        impl $crate::reactive::IDisposable for $ty {
            fn dispose(&self) {
                $crate::data::core::UntypedBindingExpression::dispose_expression(self)
            }
        }

        impl $crate::property_store::IValueEntry for $ty {
            fn property(&self) -> &'static $crate::FerroProperty {
                $crate::data::core::UntypedBindingExpression::base(self)
                    .target_property()
                    .expect("The binding expression is not attached.")
            }

            fn has_value(&self) -> bool {
                $crate::data::core::UntypedBindingExpression::start(self, false);
                true
            }

            fn try_get_value(&self, _out: &mut dyn ::std::any::Any) -> bool {
                false
            }

            fn get_value_boxed(&self) -> ::std::option::Option<$crate::BoxedValue> {
                $crate::data::core::UntypedBindingExpression::start(self, false);
                $crate::data::core::UntypedBindingExpression::base(self).get_value_or_default()
            }

            fn get_data_validation_state(
                &self,
            ) -> ::std::option::Option<(
                $crate::data::BindingValueType,
                ::std::option::Option<$crate::data::BindingError>,
            )> {
                $crate::data::core::UntypedBindingExpression::base(self).data_validation_state()
            }

            fn unsubscribe(&self) {
                $crate::data::core::UntypedBindingExpression::stop(self)
            }

            fn as_binding_expression(
                self: ::std::rc::Rc<Self>,
            ) -> ::std::option::Option<::std::rc::Rc<dyn $crate::data::BindingExpressionBase>> {
                ::std::option::Option::Some(self)
            }
        }

        #[allow(private_interfaces)]
        impl $crate::data::BindingExpressionBase for $ty {
            fn priority(&self) -> $crate::data::BindingPriority {
                $crate::data::core::UntypedBindingExpression::base(self).priority()
            }

            fn target_property(&self) -> ::std::option::Option<&'static $crate::FerroProperty> {
                $crate::data::core::UntypedBindingExpression::base(self).target_property()
            }

            fn default_priority(&self) -> $crate::data::BindingPriority {
                $crate::data::core::UntypedBindingExpression::base(self).default_priority()
            }

            fn is_data_validation_enabled(&self) -> bool {
                $crate::data::core::UntypedBindingExpression::base(self).is_data_validation_enabled()
            }

            fn update_source(&self) {
                $crate::data::core::UntypedBindingExpression::update_source_core(self)
            }

            fn update_target(&self) {
                $crate::data::core::UntypedBindingExpression::update_target_core(self)
            }

            fn attach(
                &self,
                sink: $crate::data::core::SinkRef,
                frame: ::std::option::Option<::std::rc::Weak<$crate::property_store::ImmediateValueFrame>>,
                target: &$crate::FerroObject,
                target_property: &'static $crate::FerroProperty,
                priority: $crate::data::BindingPriority,
            ) {
                $crate::data::core::UntypedBindingExpression::base(self).attach(
                    sink,
                    frame,
                    target,
                    ::std::option::Option::Some(target_property),
                    priority,
                )
            }

            fn start(&self, produce_value: bool) {
                $crate::data::core::UntypedBindingExpression::start(self, produce_value)
            }

            fn as_untyped(&self) -> ::std::option::Option<&dyn $crate::data::core::UntypedBindingExpression> {
                ::std::option::Option::Some(self)
            }

            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
        }
    };
}
pub(crate) use impl_untyped_binding_expression;

/// Adapts a binding expression to an observable subject: observers receive
/// the values of the expression, and values pushed into the subject are
/// written to the binding source.
///
/// This preserves the semantics of binding expressions as observed through
/// an observable (mostly for unit tests), not necessarily those of an
/// expression instantiated on an object. An expression may act either as an
/// observable or as a binding on an object, not both.
pub struct ObservableSink {
    this: Weak<ObservableSink>,
    expression: Rc<dyn UntypedBindingExpression>,
    value: RefCell<Option<BoxedValue>>,
    observers: RefCell<Vec<(u64, Rc<dyn IObserver<Option<BoxedValue>>>)>>,
    next_id: Cell<u64>,
}

impl ObservableSink {
    /// Produces an observable which can be used to observe the value of
    /// `expression`. Panics if the expression is already instantiated on an
    /// object.
    pub fn create<E: UntypedBindingExpression>(expression: &Rc<E>, target: Option<&FerroObject>) -> Rc<ObservableSink> {
        let base = expression.base();
        if let Some(existing) = base.observable.borrow().upgrade() {
            return existing;
        }
        if base.sink.borrow().is_some() {
            panic!(
                "Cannot observe a binding expression which is already instantiated on an object."
            );
        }
        let expression: Rc<dyn UntypedBindingExpression> = expression.clone();
        let sink = Rc::new_cyclic(|this: &Weak<ObservableSink>| ObservableSink {
            this: this.clone(),
            expression,
            value: RefCell::new(Some(FerroProperty::unset_value())),
            observers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
        });
        let weak: Weak<dyn IBindingExpressionSink> = Rc::downgrade(&sink) as Weak<dyn IBindingExpressionSink>;
        base.sink.replace(Some(SinkRef::Other(weak)));
        base.observable.replace(Rc::downgrade(&sink));
        base.target.replace(target.map(FerroObject::to_weak));
        sink
    }

    /// The expression behind the subject.
    pub fn expression(&self) -> &Rc<dyn UntypedBindingExpression> {
        &self.expression
    }

    /// Writes a value to the binding source.
    pub fn on_next(&self, value: Option<BoxedValue>) {
        self.expression.write_value_to_source(value);
    }

    fn publish_next(&self, value: Option<BoxedValue>) {
        self.value.replace(value.clone());
        let observers: Vec<_> = self.observers.borrow().iter().map(|(_, o)| o.clone()).collect();
        for observer in observers {
            observer.on_next(value.clone());
        }
    }
}

impl IBindingExpressionSink for ObservableSink {
    fn on_changed(&self, _instance: &Rc<dyn BindingExpressionBase>, has_value_changed: bool, _has_error_changed: bool) {
        let base = self.expression.base();
        let error = base.error();
        let value = base.get_value_or_default();

        if base.is_data_validation_enabled() || error.is_some() {
            let notification = match error {
                Some(e) => BindingNotification::with_error_and_fallback(e.exception, e.error_type, value),
                None => BindingNotification::new(value),
            };
            self.publish_next(Some(Rc::new(notification)));
        } else if has_value_changed {
            self.publish_next(value);
        }
    }

    fn on_completed(&self, _instance: &Rc<dyn BindingExpressionBase>) {
        let observers: Vec<_> = self.observers.borrow().iter().map(|(_, o)| o.clone()).collect();
        for observer in observers {
            observer.on_completed();
        }
    }
}

impl IObservable<Option<BoxedValue>> for ObservableSink {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<BoxedValue>>>) -> Rc<dyn IDisposable> {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let first = self.observers.borrow().is_empty();
        self.observers.borrow_mut().push((id, observer.clone()));
        if first {
            self.expression.start(true);
        } else {
            let value = self.value.borrow().clone();
            if !is_unset(&value) {
                observer.on_next(value);
            }
        }
        // The subscription keeps the subject (and with it the expression)
        // alive.
        let this = self.this.upgrade().expect("subject is alive");
        Disposable::create(move || {
            let last = {
                let mut observers = this.observers.borrow_mut();
                observers.retain(|(i, _)| *i != id);
                observers.is_empty()
            };
            if last {
                this.expression.stop();
            }
        })
    }
}
