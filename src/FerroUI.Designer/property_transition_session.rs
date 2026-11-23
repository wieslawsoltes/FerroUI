//! Apply transition timing through the designer's authored document transaction.
use crate::DesignerSession;
use ferroui_designer_animation::{PropertyTransitionList, PropertyTransitionTiming};
use ferroui_designer_core::NodeId;

impl DesignerSession {
    /// Read the transition collection authored at the given control, style, or resource.
    pub fn property_transitions(&self, owner: NodeId) -> Result<PropertyTransitionList, String> {
        if self.uses_rust_source() {
            return Err("Choose an authored XAML transition collection".into());
        }
        PropertyTransitionList::read(self.document(), owner).map_err(|error| error.to_string())
    }

    /// Change the literal timing of a transition in the accepted XAML preview.
    pub fn edit_property_transition_timing(
        &mut self,
        owner: NodeId,
        transition: NodeId,
        timing: &PropertyTransitionTiming,
    ) -> Result<(), String> {
        if self.uses_rust_source() || !self.can_manipulate() {
            return Err(
                "Wait for the current authored XAML preview before editing a transition".into(),
            );
        }
        let edits = PropertyTransitionList::plan_timing(self.document(), owner, transition, timing)
            .map_err(|error| error.to_string())?;
        if !edits.is_empty() {
            self.apply("Edit property transition timing", edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
