use super::AutomationProperty;

/// Contains values used as identifiers by `IScrollProvider`.
pub struct ScrollPatternIdentifiers;

impl ScrollPatternIdentifiers {
    /// Specifies that scrolling should not be performed.
    pub const NO_SCROLL: f64 = -1.0;

    /// Identifies the `IScrollProvider::horizontally_scrollable` automation property.
    pub fn horizontally_scrollable_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IScrollProvider::horizontal_scroll_percent` automation property.
    pub fn horizontal_scroll_percent_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IScrollProvider::horizontal_view_size` automation property.
    pub fn horizontal_view_size_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IScrollProvider::vertically_scrollable` automation property.
    pub fn vertically_scrollable_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IScrollProvider::vertical_scroll_percent` automation property.
    pub fn vertical_scroll_percent_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }

    /// Identifies the `IScrollProvider::vertical_view_size` automation property.
    pub fn vertical_view_size_property() -> &'static AutomationProperty {
        static PROPERTY: AutomationProperty = AutomationProperty::new();
        &PROPERTY
    }
}
