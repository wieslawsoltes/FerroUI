//! Resolve native behavior and action IDs in the accepted RSX projection.
use crate::DesignerSession;
use ferroui_designer_animation::{BehaviorEdit, BehaviorModel, BehaviorSource};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_rsx::{RsxBehaviorEdit, RsxProjection};

pub(crate) enum BehaviorCommand {
    Upsert(BehaviorModel),
    Remove(NodeId),
    Edit(NodeId, BehaviorEdit),
}

impl DesignerSession {
    pub(super) fn apply_rust_behavior(&mut self, command: BehaviorCommand) -> Result<(), String> {
        let projection = self
            .rust
            .as_ref()
            .ok_or("Open the current Rust view")?
            .projection()?;
        let patches = rust_behavior_plan(self, &projection, command)?;
        if !patches.is_empty() {
            self.commit_rust_patches("Edit Rust behavior", patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }
}

pub(crate) fn rust_behavior_plan(
    session: &DesignerSession,
    projection: &RsxProjection<'_>,
    command: BehaviorCommand,
) -> Result<Vec<TextPatch>, String> {
    if !session.can_manipulate() {
        return Err("Wait for the current editable Rust behavior preview".into());
    }
    let document = session.document();
    let owner = match &command {
        BehaviorCommand::Upsert(model) => model.owner,
        BehaviorCommand::Remove(id) | BehaviorCommand::Edit(id, _) => {
            BehaviorSource::owner(document, *id).map_err(|error| error.to_string())?
        }
    };
    let (index, entries) = mapped_behaviors(document, projection, owner)?;
    let source = |id: NodeId| {
        entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.node == id)
            .ok_or("Choose an authored Rust behavior in this control")
    };
    let edit = match command {
        BehaviorCommand::Upsert(model) => match model.id {
            Some(id) => RsxBehaviorEdit::Replace {
                behavior: source(id)?.0,
                model,
            },
            None => RsxBehaviorEdit::Attach(model),
        },
        BehaviorCommand::Remove(id) => RsxBehaviorEdit::Remove(source(id)?.0),
        BehaviorCommand::Edit(id, edit) => {
            let (behavior, entry) = source(id)?;
            let action = |node| {
                entry
                    .actions
                    .iter()
                    .position(|action| action.node == node)
                    .ok_or("Choose an action owned by this behavior")
            };
            match edit {
                BehaviorEdit::SetTrigger(value) => RsxBehaviorEdit::SetTrigger { behavior, value },
                BehaviorEdit::AppendAction(value) => {
                    RsxBehaviorEdit::AppendAction { behavior, value }
                }
                BehaviorEdit::SetAction { node, value } => RsxBehaviorEdit::SetAction {
                    behavior,
                    action: action(node)?,
                    value,
                },
                BehaviorEdit::MoveAction { node, index } => RsxBehaviorEdit::MoveAction {
                    behavior,
                    action: action(node)?,
                    index,
                },
                BehaviorEdit::RemoveAction(node) => RsxBehaviorEdit::RemoveAction {
                    behavior,
                    action: action(node)?,
                },
            }
        }
    };
    projection.edit_behavior(index, edit)
}

pub(super) fn mapped_behaviors(
    document: &Document,
    projection: &RsxProjection<'_>,
    owner: NodeId,
) -> Result<(usize, Vec<BehaviorSource>), String> {
    let index = projection
        .control_index(document, owner)
        .ok_or("Select a mapped Rust control")?;
    let entries = BehaviorSource::read(document, owner).map_err(|error| error.to_string())?;
    let mapped = projection.behaviors(index);
    if mapped.len() != entries.len()
        || mapped.iter().zip(&entries).any(|(rust, native)| {
            rust.model().trigger != native.model.trigger
                || rust.model().actions != native.model.actions
                || rust.model().actions.len() != native.actions.len()
        })
    {
        return Err("Wait for the accepted Rust behavior preview".into());
    }
    Ok((index, entries))
}
