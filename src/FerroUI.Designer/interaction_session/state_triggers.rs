use crate::DesignerSession;
use ferroui_designer_animation::native::StateTriggerCapture;
use ferroui_designer_core::NodeId;
use ferroui_designer_rsx::StateTriggerReport;

impl DesignerSession {
    /// Record state changes from one input so a snapshot host can render them.
    pub fn capture_state_trigger_input(&self) -> StateTriggerCapture {
        self.preview.state_triggers.capture_input()
    }

    pub(crate) fn rust_state_triggers(&self, source: NodeId) -> Option<StateTriggerReport> {
        if !self.authored_is_valid() {
            return None;
        }
        let rust = self.rust.as_ref()?;
        let offset = self.document.ast().node(source)?.span.start;
        let (ordinal, _) = rust.map()?.control(offset)?;
        rust.projection().ok()?.state_trigger_report(ordinal)
    }

    pub(crate) fn set_rust_state_trigger(
        &mut self,
        source: NodeId,
        event: &str,
        group: &str,
        state: Option<&str>,
    ) -> Result<(), String> {
        if !self.can_manipulate() {
            return Err("Open a current, editable Rust view".into());
        }
        let rust = self.rust.as_ref().ok_or("Open the current Rust view")?;
        let offset = self
            .document
            .ast()
            .node(source)
            .ok_or("Choose an authored Rust control")?
            .span
            .start;
        let (ordinal, _) = rust
            .map()
            .and_then(|map| map.control(offset))
            .ok_or("Choose a mapped Rust control")?;
        let patches = rust
            .projection()?
            .plan_state_trigger(ordinal, event, group, state)?;
        if !patches.is_empty() {
            self.commit_rust_patches("Edit Rust state trigger", patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }
}
