use crate::DesignerSession;
use ferroui_designer_core::{Edit, NodeId};

impl DesignerSession {
    pub(crate) fn apply_rust_placement(
        &mut self,
        revision: u64,
        label: &str,
        edits: Vec<Edit>,
        after: Option<NodeId>,
        moving: &[NodeId],
    ) -> Result<(), String> {
        if self.document().revision() != revision {
            return Err("The Rust source changed during placement".into());
        }
        let rust = self
            .rust
            .as_ref()
            .ok_or("Open a mapped Rust view for this placement")?;
        let Some((plan, basis, inserted)) = rust
            .plan_child_placement(self.document(), label, edits, after)
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let targets = if moving.is_empty() { &inserted } else { moving };
        if targets.iter().any(|node| !plan.controls.contains_key(node)) {
            return Err("The placed control has no Rust source range".into());
        }
        let change = self
            .commit_rust_child_edit(label, plan, &basis)
            .map_err(|error| error.to_string())?;
        if !change.is_empty() {
            self.select(targets.iter().copied());
            self.changed(true);
        }
        Ok(())
    }
}
