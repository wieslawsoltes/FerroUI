//! Port of `AspectRatioClassSetter.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::ComparisonConditionType;

/// Conditional class setter based on aspect ratio used in
/// [`AspectRatioBehavior`](crate::AspectRatioBehavior).
#[repr(C)]
pub struct AspectRatioClassSetter {
    base: FerroObject,
}

ferro_class!(AspectRatioClassSetter: FerroObject);
ferro_class_info!(AspectRatioClassSetter {
    new: AspectRatioClassSetter::new,
    markup: {
        content: ClassName,
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AspectRatioClassSetter: FerroObjectImpl);

ferro_properties! {
    impl AspectRatioClassSetter {
        /// Identifies the `MinRatio` property.
        pub fn min_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<AspectRatioClassSetter, _>("MinRatio", 0.0)
        }

        /// Identifies the `MinRatioOperator` property.
        pub fn min_ratio_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AspectRatioClassSetter, _>(
                "MinRatioOperator",
                ComparisonConditionType::GreaterThanOrEqual,
            )
        }

        /// Identifies the `MaxRatio` property.
        pub fn max_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<AspectRatioClassSetter, _>("MaxRatio", f64::INFINITY)
        }

        /// Identifies the `MaxRatioOperator` property.
        pub fn max_ratio_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<AspectRatioClassSetter, _>("MaxRatioOperator", ComparisonConditionType::LessThan)
        }

        /// Identifies the `ClassName` property.
        pub fn class_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<AspectRatioClassSetter, _>("ClassName", None)
        }

        /// Identifies the `IsPseudoClass` property.
        pub fn is_pseudo_class_property() -> StyledProperty<bool> {
            FerroProperty::register::<AspectRatioClassSetter, _>("IsPseudoClass", false)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the setter does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AspectRatioClassSetter, _>("TargetControl", None)
        }
    }
}

impl AspectRatioClassSetter {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a setter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets minimum aspect ratio used for comparison. This is a property of the object
    /// model.
    pub fn min_ratio(&self) -> f64 {
        self.get_value(Self::min_ratio_property())
    }

    pub fn set_min_ratio(&self, value: f64) {
        self.set_value(Self::min_ratio_property(), value)
    }

    /// Gets or sets minimum aspect ratio comparison operator. This is a property of the object
    /// model.
    pub fn min_ratio_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::min_ratio_operator_property())
    }

    pub fn set_min_ratio_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::min_ratio_operator_property(), value)
    }

    /// Gets or sets maximum aspect ratio used for comparison. This is a property of the object
    /// model.
    pub fn max_ratio(&self) -> f64 {
        self.get_value(Self::max_ratio_property())
    }

    pub fn set_max_ratio(&self, value: f64) {
        self.set_value(Self::max_ratio_property(), value)
    }

    /// Gets or sets maximum aspect ratio comparison operator. This is a property of the object
    /// model.
    pub fn max_ratio_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::max_ratio_operator_property())
    }

    pub fn set_max_ratio_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::max_ratio_operator_property(), value)
    }

    /// Gets or sets the class name that should be added or removed. This is a property of the
    /// object model, and the content of the setter in markup.
    pub fn class_name(&self) -> Option<String> {
        self.get_value(Self::class_name_property())
    }

    pub fn set_class_name(&self, value: Option<String>) {
        self.set_value(Self::class_name_property(), value)
    }

    /// Gets or sets the flag whether [`class_name`](Self::class_name) is a pseudo class. This is
    /// a property of the object model.
    pub fn is_pseudo_class(&self) -> bool {
        self.get_value(Self::is_pseudo_class_property())
    }

    pub fn set_is_pseudo_class(&self, value: bool) {
        self.set_value(Self::is_pseudo_class_property(), value)
    }

    /// Gets or sets the target control that class name should be added or removed from when
    /// triggered. This is a property of the object model; markup resolves a name given for it to
    /// the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }
}
