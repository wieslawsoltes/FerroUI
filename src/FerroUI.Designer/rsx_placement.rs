use crate::DesignerSession;
use ferroui_designer_core::{Edit, NodeId};
use std::collections::{HashMap, HashSet};

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
        let (plan, inserted) = rust
            .plan_child_placement(self.document(), label, edits, after)
            .map_err(|error| error.to_string())?;
        if plan.patches.is_empty() {
            return Ok(());
        }
        let targets = if moving.is_empty() { &inserted } else { moving };
        let ranges = targets
            .iter()
            .map(|node| {
                plan.controls
                    .get(node)
                    .copied()
                    .ok_or("The placed control has no Rust source range")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let change = self
            .commit_rust_patches(label, plan.patches)
            .map_err(|error| error.to_string())?;
        if !change.is_empty() {
            let wanted = ranges.iter().copied().collect::<HashSet<_>>();
            let selected = self
                .document()
                .ast()
                .elements()
                .filter_map(|node| {
                    let (_, name) = self.rust_map()?.control(node.span.start)?;
                    wanted.contains(&name).then_some((name, node.id))
                })
                .collect::<HashMap<_, _>>();
            self.select(
                ranges
                    .iter()
                    .filter_map(|range| selected.get(range).copied()),
            );
            self.changed(true);
        }
        Ok(())
    }
}
