use crate::data::converters::IValueConverter;
use crate::data::core::plugins::{get_property_value, set_property_value};
use crate::data::core::{
    impl_untyped_binding_expression, ExpressionError, Publish, TargetTypeConverter, UntypedBindingExpression,
    UntypedBindingExpressionBase, ValueTypes,
};
use crate::data::{BindingMode, BindingOperations, BindingPriority};
use crate::logging::LogEventLevel;
use crate::utilities::{CultureInfo, OptionCell};
use crate::{BoxedValue, FerroObject, FerroPropertyChangedEventArgs, FerroProperty, Ref, StyledElement, WeakRef};
use std::cell::{Cell, OnceCell};
use std::rc::{Rc, Weak};

/// A binding expression that reads a property of the templated parent of
/// its target.
pub struct TemplateBindingExpression {
    this: Weak<TemplateBindingExpression>,
    base: UntypedBindingExpressionBase,
    /// The converter and what it is called with, of an expression that has a converter. Few
    /// have one, and a template makes an expression for every binding of every control it
    /// builds, so the three fields are apart from the expression.
    conversion: Option<Box<Conversion>>,
    mode: BindingMode,
    property: Option<&'static FerroProperty>,
    has_published_value: Cell<bool>,
    /// What an expression of a mode that writes to the source keeps for its target
    /// (`TargetLink`), made when such an expression starts for the first time.
    target_link: OnceCell<Box<TargetLink>>,
    /// The subscription to the property changes of the templated parent. The templated
    /// parent of the target is told by the store of the target
    /// (`UntypedBindingExpression::templated_parent_changed`).
    parent_subscription: OptionCell<Subscription>,
    /// The handler of the property changes of the templated parent, made once and added
    /// again at every subscription.
    parent_handler: OnceCell<Rc<dyn Fn(&FerroPropertyChangedEventArgs<'_>)>>,
}

/// The converter of a template binding, its culture and its parameter.
struct Conversion {
    converter: Rc<dyn IValueConverter>,
    culture: Option<CultureInfo>,
    parameter: Option<BoxedValue>,
}

/// The handler of the property changes of the target, made once and added again at every
/// subscription, and the subscription: a change of the target property is written to the
/// templated parent, in the modes that write to the source.
struct TargetLink {
    handler: Rc<dyn Fn(&FerroPropertyChangedEventArgs<'_>)>,
    subscription: OptionCell<Subscription>,
}

/// A subscription to the property changes of an object.
struct Subscription {
    object: WeakRef<FerroObject>,
    token: u64,
}

impl Subscription {
    fn dispose(self) {
        if let Some(object) = self.object.upgrade() {
            object.remove_property_changed_handler(self.token);
        }
    }
}

impl TemplateBindingExpression {
    pub fn new(
        property: Option<&'static FerroProperty>,
        converter: Option<Rc<dyn IValueConverter>>,
        converter_culture: Option<CultureInfo>,
        converter_parameter: Option<BoxedValue>,
        mode: BindingMode,
    ) -> Rc<Self> {
        crate::perf_count!(TemplateBindingExpressionsCreated);
        // The culture and the parameter are read for a converter alone.
        let conversion = converter.map(|converter| {
            Box::new(Conversion { converter, culture: converter_culture, parameter: converter_parameter })
        });
        Rc::new_cyclic(|this: &Weak<TemplateBindingExpression>| Self {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(this.clone(), BindingPriority::Template, None, false),
            conversion,
            mode,
            property,
            has_published_value: Cell::new(false),
            target_link: OnceCell::new(),
            parent_subscription: OptionCell::new(),
            parent_handler: OnceCell::new(),
        })
    }

    fn try_get_templated_parent(&self) -> Option<Ref<FerroObject>> {
        let target = self.base.try_get_target()?;
        target.downcast_ref::<StyledElement>()?.templated_parent()
    }

    fn convert_to_target_type(&self, value: Option<BoxedValue>) -> Option<BoxedValue> {
        let target_type = self.base.target_type();
        if self.base.target_property().is_none() && target_type.is_object() {
            return value;
        }
        match TargetTypeConverter::get_default_converter().try_convert(
            value.as_ref(),
            target_type,
            &CultureInfo::invariant_culture(),
        ) {
            Some(result) => result,
            None => {
                let message = format!(
                    "Could not convert '{}' ({}) to '{}'.",
                    ValueTypes::to_display_string(value.as_ref()),
                    value.as_ref().map_or("null", |v| (**v).type_name()),
                    target_type
                );
                self.log_error(&message, LogEventLevel::Warning);
                Some(FerroProperty::unset_value())
            }
        }
    }

    fn publish(&self) {
        if self.mode == BindingMode::OneWayToSource {
            return;
        }

        if let Some(templated_parent) = self.try_get_templated_parent() {
            let mut value = match self.property {
                Some(property) => get_property_value(&templated_parent, property),
                None => Some(Rc::new(templated_parent.clone()) as BoxedValue),
            };
            let mut error: Option<ExpressionError> = None;
            if let Some(conversion) = &self.conversion {
                value = self.base.convert(
                    &|| self.should_log_error(),
                    &|| self.description(),
                    &*conversion.converter,
                    conversion.culture.as_ref(),
                    conversion.parameter.as_ref(),
                    value.as_ref(),
                    self.base.target_type(),
                    &mut error,
                );
            }
            let value = self.convert_to_target_type(value);
            self.base.publish_value(Publish::Value(value), error, false);
            self.has_published_value.set(true);

            if self.mode == BindingMode::OneTime {
                self.stop();
            }
        } else if self.has_published_value.get() {
            self.base.publish_value(Publish::Value(Some(FerroProperty::unset_value())), None, false);
        }
    }

    fn unsubscribe_parent(&self) {
        if let Some(s) = self.parent_subscription.take() {
            s.dispose();
        }
    }

    fn on_templated_parent_changed(&self) {
        self.unsubscribe_parent();
        if let Some(templated_parent) = self.try_get_templated_parent() {
            let handler = self.parent_handler.get_or_init(|| {
                let weak = self.this.clone();
                let id = self.property.map(FerroProperty::id);
                Rc::new(move |e: &FerroPropertyChangedEventArgs<'_>| {
                    if Some(e.property().id()) == id {
                        if let Some(this) = weak.upgrade() {
                            this.publish();
                        }
                    }
                })
            });
            let token = templated_parent.add_property_changed_handler(handler.clone());
            self.parent_subscription.set(Some(Subscription { object: templated_parent.downgrade(), token }));
        }
        self.publish();
    }

    fn on_target_property_changed(&self, property_id: u32) {
        if self.base.target_property().is_some_and(|p| p.id() == property_id) {
            if let (Some(target), Some(property)) = (self.base.try_get_target(), self.base.target_property()) {
                self.write_value_to_source(get_property_value(&target, property));
            }
        }
    }
}

impl UntypedBindingExpression for TemplateBindingExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        format!("{{TemplateBinding {}}}", self.property.map_or("", |p| p.name()))
    }

    fn start_core(&self) {
        self.has_published_value.set(false);
        // The store of the target tells the expression when the templated parent changes.
        if let Some(target) = self.base.try_get_target() {
            target.values().note_templated_parent_reader();
        }
        self.on_templated_parent_changed();
        if !matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource) {
            return;
        }
        if let Some(target) = self.base.try_get_target() {
            let link = self.target_link.get_or_init(|| {
                let weak = self.this.clone();
                let handler = Rc::new(move |e: &FerroPropertyChangedEventArgs<'_>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_target_property_changed(e.property().id());
                    }
                });
                Box::new(TargetLink { handler, subscription: OptionCell::new() })
            });
            let token = target.add_property_changed_handler(link.handler.clone());
            // As before, a subscription that was left is dropped without being disposed.
            link.subscription.set(Some(Subscription { object: target.downgrade(), token }));
        }
    }

    fn templated_parent_changed(&self) {
        if self.base.is_running() {
            self.on_templated_parent_changed();
        }
    }

    fn stop_core(&self) {
        self.unsubscribe_parent();
        if let Some(s) = self.target_link.get().and_then(|link| link.subscription.take()) {
            s.dispose();
        }
    }

    fn write_value_to_source(&self, value: Option<BoxedValue>) -> bool {
        let (Some(property), Some(templated_parent)) = (self.property, self.try_get_templated_parent()) else {
            return false;
        };
        let mut value = value;
        if let Some(conversion) = &self.conversion {
            value = self.base.convert_back(
                &|| self.should_log_error(),
                &|| self.description(),
                &*conversion.converter,
                conversion.culture.as_ref(),
                conversion.parameter.as_ref(),
                value.as_ref(),
                self.base.target_type(),
            );
        }
        if !BindingOperations::is_do_nothing(value.as_ref()) {
            // Written with the semantics of "set current value": the
            // templated parent's own bindings and styles stay in place.
            let target_type = crate::data::core::plugins::property_value_type(property);
            match ValueTypes::try_convert(value.as_ref(), target_type) {
                Some(Some(v)) if BindingOperations::is_unset(Some(&v)) || property.is_valid_value((*v).as_any()) => {
                    templated_parent.set_current_value_untyped(property, (*v).as_any());
                }
                _ => {
                    if BindingOperations::is_unset(value.as_ref()) {
                        let _ = set_property_value(&templated_parent, property, value.as_ref(), BindingPriority::LocalValue);
                    }
                }
            }
        }
        true
    }
}

impl_untyped_binding_expression!(TemplateBindingExpression);

impl Drop for TemplateBindingExpression {
    fn drop(&mut self) {
        if let Some(s) = self.parent_subscription.take() {
            s.dispose();
        }
        if let Some(s) = self.target_link.get().and_then(|link| link.subscription.take()) {
            s.dispose();
        }
    }
}
