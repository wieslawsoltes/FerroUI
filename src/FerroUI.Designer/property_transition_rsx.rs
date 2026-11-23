//! Bridge accepted native transition IDs to edits in their original Rust invocation.
use crate::property_transition_properties::TransitionPropertyChoice;
use crate::property_transition_session::{PropertyTransitionCommand, PropertyTransitionEditor};
use crate::DesignerSession;
use ferroui_designer_animation::PropertyTransitionList;
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_designer_rsx::{RsxProjection, RsxPropertyTransitionEdit};

impl DesignerSession {
    pub(super) fn rust_property_transition_list(
        &self,
        owner: NodeId,
    ) -> Result<PropertyTransitionList, String> {
        let projection = self.rust_transition_projection()?;
        let index = self.rust_transition_owner(owner, &projection)?;
        self.map_rust_transitions(owner, index, &projection)
    }

    pub(crate) fn rust_property_transition_editor(
        &self,
        owner: NodeId,
        projection: &RsxProjection<'_>,
    ) -> Result<PropertyTransitionEditor, String> {
        let index = self.rust_transition_owner(owner, projection)?;
        let list = self.map_rust_transitions(owner, index, projection)?;
        let can_edit = self.can_manipulate() && projection.can_edit_property_transitions(index);
        let choices = if can_edit {
            projection
                .property_transition_choices(index)
                .into_iter()
                .map(|choice| TransitionPropertyChoice {
                    property: choice.getter.into(),
                    label: choice.property,
                    type_name: choice.type_name,
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(PropertyTransitionEditor {
            list,
            choices,
            can_edit,
            requires_target: false,
        })
    }

    pub(super) fn edit_rust_property_transition(
        &mut self,
        owner: NodeId,
        command: PropertyTransitionCommand,
    ) -> Result<(), String> {
        let projection = self.rust_transition_projection()?;
        let (label, patches) = self.rust_property_transition_plan(owner, &projection, command)?;
        if !patches.is_empty() {
            self.commit_rust_patches(label, patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn rust_property_transition_plan(
        &self,
        owner: NodeId,
        projection: &RsxProjection<'_>,
        command: PropertyTransitionCommand,
    ) -> Result<(&'static str, Vec<TextPatch>), String> {
        if !self.can_manipulate() {
            return Err(
                "Wait for the current authored Rust preview before editing a transition".into(),
            );
        }
        let index = self.rust_transition_owner(owner, projection)?;
        let list = self.map_rust_transitions(owner, index, projection)?;
        let source = |transition| {
            list.entries
                .iter()
                .find(|entry| entry.id == transition)
                .map(|entry| entry.source)
                .ok_or_else(|| {
                    "Choose an entry in the current Rust transition collection".to_owned()
                })
        };
        let check_type = |getter: &str, type_name: &Option<String>| {
            if type_name.as_deref().is_some_and(|type_name| {
                !projection
                    .property_transition_choices(index)
                    .iter()
                    .any(|choice| choice.getter == getter && choice.type_name == type_name)
            }) {
                Err("Choose a compatible property and transition class from the current Rust control".to_owned())
            } else {
                Ok(())
            }
        };
        let (label, edit) = match &command {
            PropertyTransitionCommand::Add {
                property,
                type_name,
                timing,
            } => {
                check_type(property, type_name)?;
                (
                    "Add property transition",
                    RsxPropertyTransitionEdit::Add {
                        getter: property,
                        timing,
                    },
                )
            }
            PropertyTransitionCommand::Retarget {
                transition,
                property,
                type_name,
            } => {
                check_type(property, type_name)?;
                (
                    "Retarget property transition",
                    RsxPropertyTransitionEdit::Retarget {
                        source: source(*transition)?,
                        getter: property,
                    },
                )
            }
            PropertyTransitionCommand::Timing { transition, timing } => (
                "Edit property transition timing",
                RsxPropertyTransitionEdit::Timing {
                    source: source(*transition)?,
                    timing,
                },
            ),
            PropertyTransitionCommand::Move { transition, index } => (
                "Reorder property transitions",
                RsxPropertyTransitionEdit::Move {
                    source: source(*transition)?,
                    index: *index,
                },
            ),
            PropertyTransitionCommand::Remove { transition } => (
                "Remove property transition",
                RsxPropertyTransitionEdit::Remove {
                    source: source(*transition)?,
                },
            ),
        };
        projection
            .edit_property_transitions(index, edit)
            .map(|patches| (label, patches))
    }

    fn rust_transition_projection(&self) -> Result<RsxProjection<'_>, String> {
        self.rust
            .as_ref()
            .ok_or("Open a Rust designer session")?
            .projection()
    }

    fn rust_transition_owner(
        &self,
        owner: NodeId,
        projection: &RsxProjection<'_>,
    ) -> Result<usize, String> {
        projection
            .control_index(self.document(), owner)
            .ok_or_else(|| "Select a mapped Rust control".into())
    }

    fn map_rust_transitions(
        &self,
        owner: NodeId,
        index: usize,
        projection: &RsxProjection<'_>,
    ) -> Result<PropertyTransitionList, String> {
        let mut list = PropertyTransitionList::read(self.document(), owner)
            .map_err(|error| error.to_string())?;
        let Some(authored) = projection.property_transitions(index) else {
            list.source = projection.nodes[index]
                .expressions
                .get("Transitions")
                .copied();
            return Ok(list);
        };
        if list.entries.len() != authored.entries.len() {
            return Err("Wait for the accepted Rust transition preview".into());
        }
        list.source = Some(authored.source);
        for (entry, source) in list.entries.iter_mut().zip(&authored.entries) {
            if entry.type_name != source.type_name {
                return Err("Wait for the accepted Rust transition preview".into());
            }
            entry.source = source.source;
            entry.property = Some(source.property.clone());
            entry.easing = source
                .easing
                .as_ref()
                .map(ferroui_designer_animation::Easing::to_xaml);
            entry.delay_seconds = None;
            entry.editable_timing = vec!["Duration", "Easing"];
        }
        Ok(list)
    }
}
