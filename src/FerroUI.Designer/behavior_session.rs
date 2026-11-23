//! Authored behavior edits and native trigger preview for a designer session.
use crate::DesignerSession;
use crate::rsx_behavior_authoring::BehaviorCommand;
use ferroui_base::{BoxedValue, StyledElement};
use ferroui_designer_animation::{BehaviorEdit, BehaviorModel, BehaviorSource};
use ferroui_designer_core::NodeId;
use ferroui_xaml_interactivity::{ActionCollection, EventTriggerBase, ITrigger, Interaction};
use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

impl DesignerSession {
    pub fn behaviors(&self, owner: NodeId) -> Vec<BehaviorModel> {
        BehaviorModel::read(&self.document, owner).unwrap_or_default()
    }
    pub fn upsert_behavior(&mut self, model: BehaviorModel) -> Result<NodeId, String> {
        if self.uses_rust_source() {
            let owner = model.owner;
            let existing = self.behaviors(owner);
            let index = match model.id {
                Some(id) => existing.iter().position(|entry| entry.id == Some(id)).ok_or("Choose a behavior on this Rust control")?,
                None => existing.len(),
            };
            self.apply_rust_behavior(BehaviorCommand::Upsert(model))?;
            return self.behaviors(owner).get(index).and_then(|entry| entry.id)
                .ok_or_else(|| "The authored Rust behavior is absent from the accepted preview".into());
        }
        let before = self
            .behaviors(model.owner)
            .iter()
            .filter_map(|model| model.id)
            .collect::<BTreeSet<_>>();
        let edits = model
            .plan(&self.document)
            .map_err(|error| error.to_string())?;
        self.apply("Edit behavior", edits)
            .map_err(|error| error.to_string())?;
        if let Some(id) = model.id {
            return Ok(id);
        }
        self.behaviors(model.owner)
            .into_iter()
            .filter_map(|model| model.id)
            .find(|id| !before.contains(id))
            .ok_or_else(|| "Created behavior was not found".into())
    }
    pub fn remove_behavior(&mut self, id: NodeId) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.apply_rust_behavior(BehaviorCommand::Remove(id));
        }
        let edits = BehaviorModel::remove(&self.document, id).map_err(|error| error.to_string())?;
        self.apply("Remove behavior", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn behavior_sources(&self, owner: NodeId) -> Result<Vec<BehaviorSource>, String> {
        BehaviorSource::read(&self.document, owner).map_err(|error| error.to_string())
    }

    pub(crate) fn can_attach_behavior(&self, owner: NodeId) -> bool {
        if !self.can_manipulate() {
            return false;
        }
        if let Some(rust) = &self.rust {
            return rust.projection().is_ok_and(|projection| {
                projection.control_index(self.document(), owner)
                    .is_some_and(|index| projection.can_attach_behavior(index))
            });
        }
        BehaviorSource::can_attach(&self.document, owner)
    }

    pub fn edit_behavior(&mut self, id: NodeId, edit: BehaviorEdit) -> Result<(), String> {
        if self.uses_rust_source() {
            return self.apply_rust_behavior(BehaviorCommand::Edit(id, edit));
        }
        if !self.can_manipulate() {
            return Err("Open the current authored XAML preview to edit this behavior".into());
        }
        let edits =
            BehaviorSource::plan(&self.document, id, edit).map_err(|error| error.to_string())?;
        if edits.is_empty() {
            return Ok(());
        }
        self.apply("Edit behavior", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn set_behavior_source(&mut self, id: NodeId, source: &str) -> Result<(), String> {
        if self.uses_rust_source() {
            return Err("Edit the behavior's original Rust constructor in source".into());
        }
        let edits = BehaviorSource::replace_source(&self.document, id, source)
            .map_err(|error| error.to_string())?;
        self.apply("Edit behavior XAML", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn active_behavior_preview(&self) -> Option<NodeId> {
        self.interactions.active_behavior
    }

    pub(crate) fn can_preview_behavior(&self, owner: NodeId, index: usize) -> bool {
        self.behavior_actions(owner, index).is_ok()
    }

    /// Execute the attached trigger's actions in the current native preview.
    pub fn preview_behavior(&mut self, id: NodeId) -> Result<(), String> {
        let owner = BehaviorSource::owner(&self.document, id).map_err(|error| error.to_string())?;
        let index = self
            .behaviors(owner)
            .iter()
            .position(|model| model.id == Some(id))
            .ok_or("Unknown behavior")?;
        let (sender, actions) = self.behavior_actions(owner, index)?;
        self.interactions.behavior_modified = true;
        // Isolate application code in the tool's unwinding builds. Restore the
        // authored graph if a later action fails.
        let execution = catch_unwind(AssertUnwindSafe(|| {
            Interaction::execute_actions(Some(&sender), actions.as_ref(), None);
        }));
        if let Err(error) = execution {
            self.clear_interaction_preview();
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("A native action failed");
            return Err(format!("Could not preview behavior actions. {message}"));
        }
        self.interactions.active_behavior = Some(id);
        for group in self.state_groups() {
            let Some(owner) = self
                .preview
                .visual(group.owner)
                .and_then(|visual| visual.cast::<StyledElement>())
            else {
                continue;
            };
            let key = (group.owner, group.name.clone());
            if let Some(state) = group
                .states
                .iter()
                .find(|state| owner.classes().contains(&group.class_name(&state.name)))
            {
                self.interactions
                    .active_states
                    .insert(key, state.name.clone());
            } else {
                self.interactions.active_states.remove(&key);
            }
        }
        self.import_animations();
        self.generation += 1;
        Ok(())
    }

    fn behavior_actions(
        &self,
        owner: NodeId,
        index: usize,
    ) -> Result<(BoxedValue, Option<ActionCollection>), String> {
        if !self.authored_is_valid()
            || !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document.revision())
        {
            return Err("Wait for the current native preview before running actions".into());
        }
        let visual = self
            .preview
            .visual(owner)
            .ok_or("Behavior owner has no live preview")?;
        if visual
            .cast::<StyledElement>()
            .is_some_and(|element| !element.is_attached_to_logical_tree())
        {
            return Err("Attach the preview to a Window or rooted logical tree before running behavior actions".into());
        }
        let behaviors = visual
            .get_value(Interaction::behaviors_property())
            .ok_or("The behavior is unavailable in the current preview")?;
        let attached = behaviors.snapshot();
        let behavior = attached
            .get(index)
            .ok_or("The behavior is unavailable in the current preview")?;
        let trigger = <dyn ITrigger>::of(behavior)
            .ok_or("Use Interact to exercise this behavior's native event or lifecycle")?;
        let sender = behavior.cast::<EventTriggerBase>().and_then(|event| event.resolved_source())
            .unwrap_or_else(|| Rc::new(visual));
        Ok((sender, trigger.actions()))
    }
}
