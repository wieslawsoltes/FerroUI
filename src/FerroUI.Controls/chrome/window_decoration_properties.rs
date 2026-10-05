use ferroui_base::input::WindowDecorationsElementRole;
use ferroui_base::{ferro_property, AttachedProperty, FerroProperty, Visual};

/// Provides attached properties for window decoration hit-testing.
pub struct WindowDecorationProperties;

ferroui_base::ferro_static_type!(WindowDecorationProperties);

ferroui_base::ferro_properties! { impl WindowDecorationProperties {
    ferro_property!(
        /// Defines the `ElementRole` attached property. Marks a visual
        /// element with a specific role for non-client hit-testing. Can be
        /// applied to any element in the visual tree, not limited to
        /// decoration children.
        ///
        /// Setting an element role only has an effect if the
        /// `ExtendClientAreaToDecorationsHint` of the window is true.
        pub fn element_role_property() -> AttachedProperty<WindowDecorationsElementRole> {
            FerroProperty::register_attached::<WindowDecorationProperties, Visual, _>(
                "ElementRole",
                WindowDecorationsElementRole::None,
            )
        }
    );
} }

impl WindowDecorationProperties {
    /// Gets the [`WindowDecorationsElementRole`] for the specified element.
    pub fn get_element_role(element: &Visual) -> WindowDecorationsElementRole {
        element.get_value(Self::element_role_property())
    }

    /// Sets the [`WindowDecorationsElementRole`] for the specified element.
    ///
    /// Setting an element role only has an effect if the
    /// `ExtendClientAreaToDecorationsHint` of the window is true.
    pub fn set_element_role(element: &Visual, value: WindowDecorationsElementRole) {
        element.set_value(Self::element_role_property(), value)
    }
}
