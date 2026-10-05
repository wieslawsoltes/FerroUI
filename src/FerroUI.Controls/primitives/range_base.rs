use super::{RangeBaseValueChangedEventArgs, TemplatedControl, TemplatedControlImpl};
use crate::ControlImpl;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, StyledElementImpl, StyledElementImplExt,
    StyledProperty, StyledPropertyOptions, VisualImpl,
};
use std::cell::Cell;

/// Base class for controls that display a value within a range.
///
/// This class is abstract: only its subclasses can be created.
#[repr(C)]
pub struct RangeBase {
    base: TemplatedControl,
    is_data_context_changing: Cell<bool>,
}

ferro_class!(RangeBase: TemplatedControl);
ferro_impl_classes!(
    RangeBase: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for RangeBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::minimum_property().as_property() {
            this.on_minimum_changed();
        } else if change.property() == Self::maximum_property().as_property() {
            this.on_maximum_changed();
        } else if change.property() == Self::value_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<f64>();
            let value_changed_event_args =
                RangeBaseValueChangedEventArgs::new(old_value, new_value, Some(Self::value_changed_event()));
            this.raise_event(&value_changed_event_args);
        }
    }
}

impl StyledElementImpl for RangeBase {
    fn on_initialized(this: &Self) {
        Self::parent_on_initialized(this);

        this.coerce_value(Self::maximum_property().as_property());
        this.coerce_value(Self::value_property().as_property());
    }

    fn on_data_context_begin_update(this: &Self) {
        this.is_data_context_changing.set(true);
        Self::parent_on_data_context_begin_update(this);
    }

    fn on_data_context_end_update(this: &Self) {
        Self::parent_on_data_context_end_update(this);
        this.is_data_context_changing.set(false);
    }
}

ferroui_base::ferro_properties! { impl RangeBase {
    ferro_property!(
        /// Defines the `Minimum` property.
        pub fn minimum_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RangeBase, _>(
                "Minimum",
                StyledPropertyOptions::new(0.0).coerce(RangeBase::coerce_minimum),
            )
        }
    );

    ferro_property!(
        /// Defines the `Maximum` property.
        pub fn maximum_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RangeBase, _>(
                "Maximum",
                StyledPropertyOptions::new(100.0).coerce(RangeBase::coerce_maximum),
            )
        }
    );

    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RangeBase, _>(
                "Value",
                StyledPropertyOptions::new(0.0)
                    .default_binding_mode(BindingMode::TwoWay)
                    .coerce(RangeBase::coerce_value_core),
            )
        }
    );

    ferro_property!(
        /// Defines the `SmallChange` property.
        pub fn small_change_property() -> StyledProperty<f64> {
            FerroProperty::register::<RangeBase, _>("SmallChange", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `LargeChange` property.
        pub fn large_change_property() -> StyledProperty<f64> {
            FerroProperty::register::<RangeBase, _>("LargeChange", 10.0)
        }
    );
} }

impl RangeBase {
    ferro_routed_event!(
        /// Defines the `ValueChanged` event.
        pub fn value_changed_event() -> RoutedEvent<RangeBaseValueChangedEventArgs> {
            RoutedEvent::register::<RangeBase, _>("ValueChanged", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct(), is_data_context_changing: Cell::new(false) }
    }

    /// Occurs when the `Value` property changes.
    pub fn value_changed(
        &self,
        handler: impl Fn(&Interactive, &RangeBaseValueChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::value_changed_event(), handler)
    }

    /// The minimum possible value.
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }

    pub fn set_minimum(&self, value: f64) {
        self.set_value(Self::minimum_property(), value)
    }

    fn coerce_minimum(sender: &FerroObject, value: f64) -> f64 {
        if Self::validate_double(value) {
            value
        } else {
            sender.get_value(Self::minimum_property())
        }
    }

    fn on_minimum_changed(&self) {
        if self.is_initialized() && !self.is_data_context_changing.get() {
            self.coerce_value(Self::maximum_property().as_property());
            self.coerce_value(Self::value_property().as_property());
        }
    }

    /// The maximum possible value.
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: f64) {
        self.set_value(Self::maximum_property(), value)
    }

    fn coerce_maximum(sender: &FerroObject, value: f64) -> f64 {
        if Self::validate_double(value) {
            f64::max(value, sender.get_value(Self::minimum_property()))
        } else {
            sender.get_value(Self::maximum_property())
        }
    }

    fn on_maximum_changed(&self) {
        if self.is_initialized() && !self.is_data_context_changing.get() {
            self.coerce_value(Self::minimum_property().as_property());
            self.coerce_value(Self::value_property().as_property());
        }
    }

    /// The current value.
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    pub fn set_range_value(&self, value: f64) {
        self.set_value(Self::value_property(), value)
    }

    fn coerce_value_core(sender: &FerroObject, value: f64) -> f64 {
        if Self::validate_double(value) {
            MathUtilities::clamp(
                value,
                sender.get_value(Self::minimum_property()),
                sender.get_value(Self::maximum_property()),
            )
        } else {
            sender.get_value(Self::value_property())
        }
    }

    /// The small increment value added to or subtracted from the value.
    pub fn small_change(&self) -> f64 {
        self.get_value(Self::small_change_property())
    }

    pub fn set_small_change(&self, value: f64) {
        self.set_value(Self::small_change_property(), value)
    }

    /// The large increment value added to or subtracted from the value.
    pub fn large_change(&self) -> f64 {
        self.get_value(Self::large_change_property())
    }

    pub fn set_large_change(&self, value: f64) {
        self.set_value(Self::large_change_property(), value)
    }

    /// Checks that the value is neither infinity nor NaN.
    fn validate_double(value: f64) -> bool {
        !value.is_infinite() && !value.is_nan()
    }
}
