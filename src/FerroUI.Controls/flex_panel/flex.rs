use super::{FlexAlignItems, FlexBasis};
use ferroui_base::layout::Layoutable;
use ferroui_base::{AttachedProperty, FerroProperty, StyledPropertyOptions};

/// Provides attached properties for controlling the layout behavior of child
/// elements within a `FlexPanel`.
pub struct Flex;

ferroui_base::ferro_static_type!(Flex);

ferroui_base::ferro_properties! {
    impl Flex {
        /// Defines an attached property to control the cross-axis alignment
        /// of a specific child in a flex layout.
        pub fn align_self_property() -> AttachedProperty<Option<FlexAlignItems>> {
            FerroProperty::register_attached::<Flex, Layoutable, _>("AlignSelf", None)
        }

        /// Defines an attached property to control the order of a specific
        /// child in a flex layout.
        pub fn order_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<Flex, Layoutable, _>("Order", 0)
        }

        /// Defines an attached property to control the initial main-axis
        /// size of a specific child in a flex layout.
        pub fn basis_property() -> AttachedProperty<FlexBasis> {
            FerroProperty::register_attached::<Flex, Layoutable, _>("Basis", FlexBasis::AUTO)
        }

        /// Defines an attached property to control the factor by which a
        /// specific child can shrink along the main-axis in a flex layout.
        pub fn shrink_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<Flex, Layoutable, _>(
                "Shrink",
                StyledPropertyOptions::new(1.0).validate(|v| *v >= 0.0),
            )
        }

        /// Defines an attached property to control the factor by which a
        /// specific child can grow along the main-axis in a flex layout.
        pub fn grow_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<Flex, Layoutable, _>(
                "Grow",
                StyledPropertyOptions::new(0.0).validate(|v| *v >= 0.0),
            )
        }

        /// The main-axis length of a child before free space is distributed.
        pub(crate) fn base_length_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Flex, Layoutable, _>("BaseLength", 0.0)
        }

        /// The main-axis length of a child after free space is distributed.
        pub(crate) fn current_length_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<Flex, Layoutable, _>("CurrentLength", 0.0)
        }
    }
}

impl Flex {
    /// Gets the cross-axis alignment override for a child item in a
    /// `FlexPanel`.
    ///
    /// This property is used to override the `AlignItems` property of the
    /// panel for a specific child. When omitted, `AlignItems` is not
    /// overridden. Equivalent to the CSS align-self property.
    pub fn get_align_self(layoutable: &Layoutable) -> Option<FlexAlignItems> {
        layoutable.get_value(Self::align_self_property())
    }

    /// Sets the cross-axis alignment override for a child item in a
    /// `FlexPanel`.
    ///
    /// This property is used to override the `AlignItems` property of the
    /// panel for a specific child. When omitted, `AlignItems` is not
    /// overridden. Equivalent to the CSS align-self property.
    pub fn set_align_self(layoutable: &Layoutable, value: Option<FlexAlignItems>) {
        layoutable.set_value(Self::align_self_property(), value)
    }

    /// Retrieves the order in which a child item appears within the
    /// `FlexPanel`.
    ///
    /// A lower order value means the item will be positioned earlier within
    /// the container. Items with the same order value are laid out in their
    /// source document order. When omitted, it is set to 0. Equivalent to the
    /// CSS order property.
    pub fn get_order(layoutable: &Layoutable) -> i32 {
        layoutable.get_value(Self::order_property())
    }

    /// Sets the order in which a child item appears within the `FlexPanel`.
    ///
    /// A lower order value means the item will be positioned earlier within
    /// the container. Items with the same order value are laid out in their
    /// source document order. When omitted, it is set to 0. Equivalent to the
    /// CSS order property.
    pub fn set_order(layoutable: &Layoutable, value: i32) {
        layoutable.set_value(Self::order_property(), value)
    }

    /// Gets the initial size along the main-axis of an item in a
    /// `FlexPanel`, before free space is distributed according to the flex
    /// factors.
    ///
    /// Either automatic size, a fixed length, or a percentage of the
    /// container's size. When omitted, it is set to [`FlexBasis::AUTO`].
    /// Equivalent to the CSS flex-basis property.
    pub fn get_basis(layoutable: &Layoutable) -> FlexBasis {
        layoutable.get_value(Self::basis_property())
    }

    /// Sets the initial size along the main-axis of an item in a
    /// `FlexPanel`, before free space is distributed according to the flex
    /// factors.
    ///
    /// Either automatic size, a fixed length, or a percentage of the
    /// container's size. When omitted, it is set to [`FlexBasis::AUTO`].
    /// Equivalent to the CSS flex-basis property.
    pub fn set_basis(layoutable: &Layoutable, value: FlexBasis) {
        layoutable.set_value(Self::basis_property(), value)
    }

    /// Gets the factor by which an item can shrink along the main-axis,
    /// relative to other items in a `FlexPanel`.
    ///
    /// When omitted, it is set to 1. Equivalent to the CSS flex-shrink
    /// property.
    pub fn get_shrink(layoutable: &Layoutable) -> f64 {
        layoutable.get_value(Self::shrink_property())
    }

    /// Sets the factor by which an item can shrink along the main-axis,
    /// relative to other items in a `FlexPanel`.
    ///
    /// When omitted, it is set to 1. Equivalent to the CSS flex-shrink
    /// property.
    pub fn set_shrink(layoutable: &Layoutable, value: f64) {
        layoutable.set_value(Self::shrink_property(), value)
    }

    /// Gets the factor by which an item can grow along the main-axis,
    /// relative to other items in a `FlexPanel`.
    ///
    /// When omitted, it is set to 0. Equivalent to the CSS flex-grow
    /// property.
    pub fn get_grow(layoutable: &Layoutable) -> f64 {
        layoutable.get_value(Self::grow_property())
    }

    /// Sets the factor by which an item can grow along the main-axis,
    /// relative to other items in a `FlexPanel`.
    ///
    /// When omitted, it is set to 0. Equivalent to the CSS flex-grow
    /// property.
    pub fn set_grow(layoutable: &Layoutable, value: f64) {
        layoutable.set_value(Self::grow_property(), value)
    }

    pub(crate) fn get_base_length(layoutable: &Layoutable) -> f64 {
        layoutable.get_value(Self::base_length_property())
    }

    pub(crate) fn set_base_length(layoutable: &Layoutable, value: f64) {
        layoutable.set_value(Self::base_length_property(), value)
    }

    pub(crate) fn get_current_length(layoutable: &Layoutable) -> f64 {
        layoutable.get_value(Self::current_length_property())
    }

    pub(crate) fn set_current_length(layoutable: &Layoutable, value: f64) {
        layoutable.set_value(Self::current_length_property(), value)
    }
}
