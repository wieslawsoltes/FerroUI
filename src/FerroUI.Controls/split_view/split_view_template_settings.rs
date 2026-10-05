use crate::GridLength;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};

/// Provides calculated values for use with the control theme or template of
/// the [`SplitView`](crate::SplitView).
///
/// This class is NOT intended for general use outside of control templates.
#[repr(C)]
pub struct SplitViewTemplateSettings {
    base: FerroObject,
}

ferro_class!(SplitViewTemplateSettings: FerroObject);
// The constructor is internal upstream: no default constructor is declared.
ferro_class_info!(SplitViewTemplateSettings {});

impl FerroObjectImpl for SplitViewTemplateSettings {}

ferro_properties! {
    impl SplitViewTemplateSettings {
        /// Defines the `ClosedPaneWidth` property.
        pub fn closed_pane_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewTemplateSettings, _>("ClosedPaneWidth", 0.0)
        }

        /// Defines the `PaneColumnGridLength` property.
        pub fn pane_column_grid_length_property() -> StyledProperty<GridLength> {
            FerroProperty::register::<SplitViewTemplateSettings, _>("PaneColumnGridLength", GridLength::default())
        }

        /// Defines the `ClosedPaneHeight` property.
        pub fn closed_pane_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<SplitViewTemplateSettings, _>("ClosedPaneHeight", 0.0)
        }

        /// Defines the `PaneRowGridLength` property.
        pub fn pane_row_grid_length_property() -> StyledProperty<GridLength> {
            FerroProperty::register::<SplitViewTemplateSettings, _>("PaneRowGridLength", GridLength::default())
        }
    }
}

impl SplitViewTemplateSettings {
    /// Creates the settings. Only the split view creates them.
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }

    /// The width of the pane when it is closed.
    pub fn closed_pane_width(&self) -> f64 {
        self.get_value(Self::closed_pane_width_property())
    }

    pub(crate) fn set_closed_pane_width(&self, value: f64) {
        self.set_value(Self::closed_pane_width_property(), value)
    }

    /// The length of the grid column of the pane.
    pub fn pane_column_grid_length(&self) -> GridLength {
        self.get_value(Self::pane_column_grid_length_property())
    }

    pub(crate) fn set_pane_column_grid_length(&self, value: GridLength) {
        self.set_value(Self::pane_column_grid_length_property(), value)
    }

    /// The height of the pane when it is closed.
    pub fn closed_pane_height(&self) -> f64 {
        self.get_value(Self::closed_pane_height_property())
    }

    pub(crate) fn set_closed_pane_height(&self, value: f64) {
        self.set_value(Self::closed_pane_height_property(), value)
    }

    /// The length of the grid row of the pane.
    pub fn pane_row_grid_length(&self) -> GridLength {
        self.get_value(Self::pane_row_grid_length_property())
    }

    pub(crate) fn set_pane_row_grid_length(&self, value: GridLength) {
        self.set_value(Self::pane_row_grid_length_property(), value)
    }
}
