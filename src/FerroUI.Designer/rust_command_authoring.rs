use crate::{DesignerSession, NodeId};
use ferroui_designer_core::TextPatch;
use ferroui_designer_rsx::{RsxCommandEdit, RsxCommandSlot, RsxProjection};

impl DesignerSession {
    pub(crate) fn has_rust_command_slots(&self, node: NodeId) -> bool {
        self.document().ast().node(node).is_some_and(|node| {
            self.rust_map()
                .is_some_and(|map| map.supports_commands(node.span.start))
        })
    }

    pub(crate) fn rust_command_slots(
        &self,
        projection: &RsxProjection<'_>,
        owner: NodeId,
    ) -> Vec<RsxCommandSlot> {
        let mut slots = projection.command_slots(self.document(), owner);
        if !self.can_manipulate() || self.ensure_layer_manipulable(owner).is_err() {
            for slot in &mut slots {
                slot.binding.editable = false;
                if let Some(parameter) = &mut slot.parameter {
                    parameter.editable = false;
                }
            }
        }
        slots
    }

    pub(crate) fn rust_command_plan(
        &self,
        projection: &RsxProjection<'_>,
        owner: NodeId,
        change: &RsxCommandEdit,
    ) -> Result<Vec<TextPatch>, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() {
            return Err("Wait for the current editable Rust preview".into());
        }
        self.ensure_layer_manipulable(owner)?;
        projection.plan_command(self.document(), owner, change)
    }

    pub(crate) fn apply_rust_command(&mut self, patches: Vec<TextPatch>) -> Result<(), String> {
        if !self
            .commit_rust_patches("Edit Rust command binding", patches)
            .map_err(|error| error.to_string())?
            .is_empty()
        {
            self.changed(true);
        }
        Ok(())
    }
}
