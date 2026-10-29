//! Port of `SplitView/SplitViewStateSetter.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledProperty,
};
use ferroui_controls::{SplitView, SplitViewDisplayMode, SplitViewPanePlacement};
use ferroui_xaml_interactivity::ComparisonConditionType;

/// Describes a state for [`SplitViewStateBehavior`](crate::SplitViewStateBehavior) using size
/// conditions.
#[repr(C)]
pub struct SplitViewStateSetter {
    base: FerroObject,
}

ferro_class!(SplitViewStateSetter: FerroObject);
ferro_class_info!(SplitViewStateSetter {
    new: SplitViewStateSetter::new,
    markup: {
        property_attributes: [
            TargetSplitView: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SplitViewStateSetter: FerroObjectImpl);

ferro_properties! {
    impl SplitViewStateSetter {
        /// Identifies the `MinWidth` property.
        pub fn min_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewStateSetter, _>("MinWidth", 0.0)
        }

        /// Identifies the `MinWidthOperator` property.
        pub fn min_width_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<SplitViewStateSetter, _>(
                "MinWidthOperator",
                ComparisonConditionType::GreaterThanOrEqual,
            )
        }

        /// Identifies the `MaxWidth` property.
        pub fn max_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewStateSetter, _>("MaxWidth", f64::INFINITY)
        }

        /// Identifies the `MaxWidthOperator` property.
        pub fn max_width_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<SplitViewStateSetter, _>("MaxWidthOperator", ComparisonConditionType::LessThan)
        }

        /// Identifies the `MinHeight` property.
        pub fn min_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewStateSetter, _>("MinHeight", 0.0)
        }

        /// Identifies the `MinHeightOperator` property.
        pub fn min_height_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<SplitViewStateSetter, _>(
                "MinHeightOperator",
                ComparisonConditionType::GreaterThanOrEqual,
            )
        }

        /// Identifies the `MaxHeight` property.
        pub fn max_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewStateSetter, _>("MaxHeight", f64::INFINITY)
        }

        /// Identifies the `MaxHeightOperator` property.
        pub fn max_height_operator_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<SplitViewStateSetter, _>("MaxHeightOperator", ComparisonConditionType::LessThan)
        }

        /// Identifies the `DisplayMode` property.
        pub fn display_mode_property() -> StyledProperty<SplitViewDisplayMode> {
            FerroProperty::register::<SplitViewStateSetter, _>("DisplayMode", SplitViewDisplayMode::Inline)
        }

        /// Identifies the `PanePlacement` property.
        pub fn pane_placement_property() -> StyledProperty<SplitViewPanePlacement> {
            FerroProperty::register::<SplitViewStateSetter, _>("PanePlacement", SplitViewPanePlacement::Left)
        }

        /// Identifies the `IsPaneOpen` property.
        pub fn is_pane_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<SplitViewStateSetter, _>("IsPaneOpen", false)
        }

        /// Identifies the `TargetSplitView` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the setter does not own the split view, so the
        /// value is an element reference.
        pub fn target_split_view_property() -> StyledProperty<Option<ElementRef<SplitView>>> {
            ValueTypes::register_element_ref::<SplitView>();
            FerroProperty::register::<SplitViewStateSetter, _>("TargetSplitView", None)
        }
    }
}

impl SplitViewStateSetter {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a setter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets minimum bounds width. This is a property of the object model.
    pub fn min_width(&self) -> f64 {
        self.get_value(Self::min_width_property())
    }

    pub fn set_min_width(&self, value: f64) {
        self.set_value(Self::min_width_property(), value)
    }

    /// Gets or sets minimum width comparison operator. This is a property of the object model.
    pub fn min_width_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::min_width_operator_property())
    }

    pub fn set_min_width_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::min_width_operator_property(), value)
    }

    /// Gets or sets maximum bounds width. This is a property of the object model.
    pub fn max_width(&self) -> f64 {
        self.get_value(Self::max_width_property())
    }

    pub fn set_max_width(&self, value: f64) {
        self.set_value(Self::max_width_property(), value)
    }

    /// Gets or sets maximum width comparison operator. This is a property of the object model.
    pub fn max_width_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::max_width_operator_property())
    }

    pub fn set_max_width_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::max_width_operator_property(), value)
    }

    /// Gets or sets minimum bounds height. This is a property of the object model.
    pub fn min_height(&self) -> f64 {
        self.get_value(Self::min_height_property())
    }

    pub fn set_min_height(&self, value: f64) {
        self.set_value(Self::min_height_property(), value)
    }

    /// Gets or sets minimum height comparison operator. This is a property of the object model.
    pub fn min_height_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::min_height_operator_property())
    }

    pub fn set_min_height_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::min_height_operator_property(), value)
    }

    /// Gets or sets maximum bounds height. This is a property of the object model.
    pub fn max_height(&self) -> f64 {
        self.get_value(Self::max_height_property())
    }

    pub fn set_max_height(&self, value: f64) {
        self.set_value(Self::max_height_property(), value)
    }

    /// Gets or sets maximum height comparison operator. This is a property of the object model.
    pub fn max_height_operator(&self) -> ComparisonConditionType {
        self.get_value(Self::max_height_operator_property())
    }

    pub fn set_max_height_operator(&self, value: ComparisonConditionType) {
        self.set_value(Self::max_height_operator_property(), value)
    }

    /// Gets or sets the display mode to apply. This is a property of the object model.
    pub fn display_mode(&self) -> SplitViewDisplayMode {
        self.get_value(Self::display_mode_property())
    }

    pub fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.set_value(Self::display_mode_property(), value)
    }

    /// Gets or sets the pane placement to apply. This is a property of the object model.
    pub fn pane_placement(&self) -> SplitViewPanePlacement {
        self.get_value(Self::pane_placement_property())
    }

    pub fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.set_value(Self::pane_placement_property(), value)
    }

    /// Gets or sets a value indicating whether the pane is open. This is a property of the object
    /// model.
    pub fn is_pane_open(&self) -> bool {
        self.get_value(Self::is_pane_open_property())
    }

    pub fn set_is_pane_open(&self, value: bool) {
        self.set_value(Self::is_pane_open_property(), value)
    }

    /// Gets or sets the target [`SplitView`] to apply the state to. This is a property of the
    /// object model; markup resolves a name given for it to the named split view.
    pub fn target_split_view(&self) -> Option<Ref<SplitView>> {
        ElementRef::resolve(&self.get_value(Self::target_split_view_property()))
    }

    pub fn set_target_split_view(&self, value: impl Into<Nullable<SplitView>>) {
        self.set_value(Self::target_split_view_property(), ElementRef::from_nullable(value.into().0))
    }
}
