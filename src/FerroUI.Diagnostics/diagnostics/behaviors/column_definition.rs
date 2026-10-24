//! Port of `Diagnostics/Behaviors/ColumnDefinition.cs`.

use ferroui_base::{ferro_properties, ferro_static_type, AttachedProperty, FerroObject, FerroProperty, StyledPropertyOptions};
use ferroui_controls::{GridLength, GridUnitType};

/// The attached `IsVisible` property of a column definition of a grid: a hidden column has a
/// width of zero, and gets back the width it had when it is shown again.
///
/// See the discussion 6773 of the upstream project.
pub struct ColumnDefinition;

ferro_static_type!(ColumnDefinition);

ferro_properties! {
    impl ColumnDefinition {
        /// The width of the column before it was hidden.
        fn last_width_property() -> AttachedProperty<Option<GridLength>> {
            FerroProperty::register_attached::<ColumnDefinition, ferroui_controls::ColumnDefinition, Option<GridLength>>(
                "LastWidth",
                None,
            )
        }

        /// Defines the `IsVisible` attached property: whether the column is shown.
        pub fn is_visible_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<ColumnDefinition, ferroui_controls::ColumnDefinition, bool>(
                "IsVisible",
                StyledPropertyOptions::new(true).coerce(ColumnDefinition::coerce_is_visible),
            )
        }
    }
}

impl ColumnDefinition {
    /// `ZeroWidth`.
    fn zero_width() -> GridLength {
        GridLength::new(0.0, GridUnitType::Pixel)
    }

    fn coerce_is_visible(element: &FerroObject, visibility: bool) -> bool {
        let last_width = element.get_value(Self::last_width_property());
        if visibility {
            if let Some(last_width) = last_width {
                element.set_value(ferroui_controls::ColumnDefinition::width_property(), last_width);
            }
        } else {
            element.set_value(
                Self::last_width_property(),
                Some(element.get_value(ferroui_controls::ColumnDefinition::width_property())),
            );
            element.set_value(ferroui_controls::ColumnDefinition::width_property(), Self::zero_width());
        }
        visibility
    }

    /// Gets the value of the attached [`is_visible_property`](Self::is_visible_property).
    pub fn get_is_visible(column_definition: &ferroui_controls::ColumnDefinition) -> bool {
        column_definition.get_value(Self::is_visible_property())
    }

    /// Sets the value of the attached [`is_visible_property`](Self::is_visible_property).
    pub fn set_is_visible(column_definition: &ferroui_controls::ColumnDefinition, visibility: bool) {
        column_definition.set_value(Self::is_visible_property(), visibility);
    }
}
