//! Port of `AdaptiveClassSetter.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::ComparisonConditionType;

/// Conditional class setter used in [`AdaptiveBehavior`](crate::AdaptiveBehavior) behavior.
#[repr(C)]
pub struct AdaptiveClassSetter {
    base: FerroObject,
}

ferro_class!(AdaptiveClassSetter: FerroObject);
ferro_class_info!(AdaptiveClassSetter {
    new: AdaptiveClassSetter::new,
    markup: {
        content: ClassName,
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AdaptiveClassSetter: FerroObjectImpl);

ferro_properties! {
    impl AdaptiveClassSetter {
        /// Identifies the `MinWidth` property.
        pub fn min_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MinWidth", 0.0)
        }

        /// Identifies the `MinWidthOperator` property.
        pub fn min_width_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AdaptiveClassSetter, _>(
                "MinWidthOperator",
                ComparisonConditionType::GreaterThanOrEqual,
            )
        }

        /// Identifies the `MaxWidth` property.
        pub fn max_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MaxWidth", f64::INFINITY)
        }

        /// Identifies the `MaxWidthOperator` property.
        pub fn max_width_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MaxWidthOperator", ComparisonConditionType::LessThan)
        }

        /// Identifies the `MinHeight` property.
        pub fn min_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MinHeight", 0.0)
        }

        /// Identifies the `MinHeightOperator` property.
        pub fn min_height_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AdaptiveClassSetter, _>(
                "MinHeightOperator",
                ComparisonConditionType::GreaterThanOrEqual,
            )
        }

        /// Identifies the `MaxHeight` property.
        pub fn max_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MaxHeight", f64::INFINITY)
        }

        /// Identifies the `MaxHeightOperator` property.
        pub fn max_height_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AdaptiveClassSetter, _>("MaxHeightOperator", ComparisonConditionType::LessThan)
        }

        /// Identifies the `ClassName` property.
        pub fn class_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<AdaptiveClassSetter, _>("ClassName", None)
        }

        /// Identifies the `IsPseudoClass` property.
        pub fn is_pseudo_class_property() -> StyledProperty<bool> {
            FerroProperty::register::<AdaptiveClassSetter, _>("IsPseudoClass", false)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the setter does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AdaptiveClassSetter, _>("TargetControl", None)
        }
    }
}

impl AdaptiveClassSetter {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a setter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets minimum bounds width value used for property comparison. This is a property
    /// of the object model.
    pub fn min_width(&self) -> f64 {
        self.get_value(Self::min_width_property())
    }

    pub fn set_min_width(&self, value: f64) {
        self.set_value(Self::min_width_property(), value)
    }

    /// Gets or sets minimum bounds width value comparison operator. This is a property of the
    /// object model.
    pub fn min_width_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::min_width_operator_property())
    }

    pub fn set_min_width_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::min_width_operator_property(), value)
    }

    /// Gets or sets maximum width value used for property comparison. This is a property of the
    /// object model.
    pub fn max_width(&self) -> f64 {
        self.get_value(Self::max_width_property())
    }

    pub fn set_max_width(&self, value: f64) {
        self.set_value(Self::max_width_property(), value)
    }

    /// Gets or sets maximum bounds width value comparison operator. This is a property of the
    /// object model.
    pub fn max_width_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::max_width_operator_property())
    }

    pub fn set_max_width_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::max_width_operator_property(), value)
    }

    /// Gets or sets minimum bounds height value used for property comparison. This is a property
    /// of the object model.
    pub fn min_height(&self) -> f64 {
        self.get_value(Self::min_height_property())
    }

    pub fn set_min_height(&self, value: f64) {
        self.set_value(Self::min_height_property(), value)
    }

    /// Gets or sets minimum bounds height value comparison operator. This is a property of the
    /// object model.
    pub fn min_height_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::min_height_operator_property())
    }

    pub fn set_min_height_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::min_height_operator_property(), value)
    }

    /// Gets or sets maximum height value used for property comparison. This is a property of the
    /// object model.
    pub fn max_height(&self) -> f64 {
        self.get_value(Self::max_height_property())
    }

    pub fn set_max_height(&self, value: f64) {
        self.set_value(Self::max_height_property(), value)
    }

    /// Gets or sets maximum bounds height value comparison operator. This is a property of the
    /// object model.
    pub fn max_height_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::max_height_operator_property())
    }

    pub fn set_max_height_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::max_height_operator_property(), value)
    }

    /// Gets or sets the class name that should be added or removed. This is a property of the
    /// object model, and the content of the setter in markup.
    pub fn class_name(&self) -> Option<String> {
        self.get_value(Self::class_name_property())
    }

    pub fn set_class_name(&self, value: Option<String>) {
        self.set_value(Self::class_name_property(), value)
    }

    /// Gets or sets the flag whether `ClassName` is a pseudo class. This is a property of the
    /// object model.
    pub fn is_pseudo_class(&self) -> bool {
        self.get_value(Self::is_pseudo_class_property())
    }

    pub fn set_is_pseudo_class(&self, value: bool) {
        self.set_value(Self::is_pseudo_class_property(), value)
    }

    /// Gets or sets the target control that class name that should be added or removed when
    /// triggered, if not set the associated object of the behavior is used or the
    /// [`target_control`](crate::AdaptiveBehavior::target_control) of the
    /// [`AdaptiveBehavior`](crate::AdaptiveBehavior). This is a property of the object model; markup
    /// resolves a name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }
}
