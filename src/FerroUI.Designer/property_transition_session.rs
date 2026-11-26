//! Apply typed transition collection edits through the authored document transaction.
use crate::DesignerSession;
use ferroui_designer_animation::native::{
    plan_xaml_property_transition, transition_choices, transition_target_type, TransitionPropertyChoice,
};
use ferroui_designer_animation::{PropertyTransitionCommand, PropertyTransitionList, PropertyTransitionTiming};
use ferroui_designer_core::NodeId;

pub(crate) struct PropertyTransitionEditor {
    pub list: PropertyTransitionList,
    pub choices: Vec<TransitionPropertyChoice>,
    pub can_edit: bool,
    pub requires_target: bool,
}

impl DesignerSession {
    /// Read the transition collection authored at the given control, style, or resource.
    pub fn property_transitions(&self, owner: NodeId) -> Result<PropertyTransitionList, String> {
        if self.uses_rust_source() {
            return self.rust_property_transition_list(owner);
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
        self.edit_property_transition(
            owner,
            None,
            PropertyTransitionCommand::Timing {
                transition,
                timing: timing.clone(),
            },
        )
    }

    pub(crate) fn property_transition_editor(
        &self,
        owner: NodeId,
        resource_target: Option<&str>,
    ) -> Result<PropertyTransitionEditor, String> {
        if self.uses_rust_source() {
            if resource_target.is_some() {
                return Err("The mapped Rust control supplies this transition target".into());
            }
            let rust = self.rust.as_ref().ok_or("Open a Rust designer session")?;
            return self.rust_property_transition_editor(owner, &rust.projection()?);
        }
        let list = self.property_transitions(owner)?;
        let can_edit = self.can_manipulate();
        let requires_target = list.target_owner.is_none();
        let choices = if can_edit {
            transition_target_type(self.document(), &list, resource_target, |owner| {
                self.preview.visual(owner).map(|visual| visual.get_type())
            })?
                .map(transition_choices)
                .unwrap_or_default()
                .into_iter()
                .map(|choice| choice.description)
                .collect()
        } else {
            Vec::new()
        };
        Ok(PropertyTransitionEditor {
            list,
            choices,
            can_edit,
            requires_target,
        })
    }

    pub(crate) fn edit_property_transition(
        &mut self,
        owner: NodeId,
        resource_target: Option<&str>,
        command: PropertyTransitionCommand,
    ) -> Result<(), String> {
        if !self.can_manipulate() {
            return Err("Wait for the current authored preview before editing a transition".into());
        }
        if self.uses_rust_source() {
            if resource_target.is_some() {
                return Err("The mapped Rust control supplies this transition target".into());
            }
            return self.edit_rust_property_transition(owner, command);
        }
        let list = self.property_transitions(owner)?;
        let edits = plan_xaml_property_transition(
            self.document(), &list, resource_target,
            |owner| self.preview.visual(owner).map(|visual| visual.get_type()),
            &command,
        )?;
        if !edits.is_empty() {
            self.apply(command.label(), edits).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
