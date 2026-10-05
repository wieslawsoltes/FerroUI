use crate::tool_tip::ToolTip;
use ferroui_base::{AttachedProperty, Ref};

/// Helper class to provide diagnostics information for [`ToolTip`].
pub struct ToolTipDiagnostics;

impl ToolTipDiagnostics {
    /// Provides access to the internal property that holds the tooltip
    /// instance of a control, for use in the dev tools.
    pub fn tool_tip_property() -> &'static AttachedProperty<Option<Ref<ToolTip>>> {
        ToolTip::tool_tip_property()
    }
}
