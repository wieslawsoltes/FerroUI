//! Apply typed transition collection edits through the authored document transaction.
use crate::native_types::registered_xml_type;
use crate::property_literals::{element_type, registered_type, setter_target_type};
use crate::DesignerSession;
use ferroui_base::animation::Animatable;
use ferroui_base::TypeInfo;
use ferroui_designer_animation::native::{
    choose_transition, transition_choices, NativeTransitionChoice, TransitionPropertyChoice,
};
use ferroui_designer_animation::{
    PropertyTransitionCollectionEdit, PropertyTransitionList, PropertyTransitionTiming,
};
use ferroui_designer_core::{Edit, MarkupValue, NodeId};

pub(crate) struct PropertyTransitionEditor {
    pub list: PropertyTransitionList,
    pub choices: Vec<TransitionPropertyChoice>,
    pub can_edit: bool,
    pub requires_target: bool,
}

pub(crate) enum PropertyTransitionCommand {
    Timing {
        transition: NodeId,
        timing: PropertyTransitionTiming,
    },
    Add {
        property: String,
        type_name: Option<String>,
        timing: PropertyTransitionTiming,
    },
    Retarget {
        transition: NodeId,
        property: String,
        type_name: Option<String>,
    },
    Move {
        transition: NodeId,
        index: usize,
    },
    Remove {
        transition: NodeId,
    },
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
            self.transition_target_type(&list, resource_target)?
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
        let document = self.document();
        let list = self.property_transitions(owner)?;
        let collection_edit = |edit| {
            PropertyTransitionList::plan_collection_edit(document, owner, edit)
                .map_err(|error| error.to_string())
        };
        let (label, edits): (&str, Vec<Edit>) = match command {
            PropertyTransitionCommand::Timing { transition, timing } => (
                "Edit property transition timing",
                PropertyTransitionList::plan_timing(document, owner, transition, &timing)
                    .map_err(|error| error.to_string())?,
            ),
            PropertyTransitionCommand::Add {
                property,
                type_name,
                timing,
            } => {
                let at = list.collection.unwrap_or(owner);
                let choice = self.choose_transition_property(
                    &list,
                    resource_target,
                    &property,
                    type_name.as_deref(),
                    None,
                )?;
                let target = choice.xaml_target(
                    document,
                    at,
                    list.target_owner.is_none(),
                    registered_xml_type,
                );
                (
                    "Add property transition",
                    collection_edit(PropertyTransitionCollectionEdit::Add {
                        target: &target,
                        timing: &timing,
                    })?,
                )
            }
            PropertyTransitionCommand::Retarget {
                transition,
                property,
                type_name,
            } => {
                let entry = list
                    .entries
                    .iter()
                    .find(|entry| entry.id == transition)
                    .ok_or("The transition is outside the selected collection")?;
                let choice = self.choose_transition_property(
                    &list,
                    resource_target,
                    &property,
                    type_name.as_deref(),
                    Some(&entry.type_name),
                )?;
                let has_static_property = document
                    .ast()
                    .node(transition)
                    .and_then(|node| node.attribute("Property"))
                    .is_some_and(|value| {
                        matches!(
                            MarkupValue::parse(value.trim()),
                            Ok(MarkupValue::Extension(_))
                        )
                    });
                let target = choice.xaml_target(
                    document,
                    transition,
                    list.target_owner.is_none() || has_static_property,
                    registered_xml_type,
                );
                (
                    "Retarget property transition",
                    collection_edit(PropertyTransitionCollectionEdit::Retarget {
                        transition,
                        target: &target,
                    })?,
                )
            }
            PropertyTransitionCommand::Move { transition, index } => (
                "Reorder property transitions",
                collection_edit(PropertyTransitionCollectionEdit::Move { transition, index })?,
            ),
            PropertyTransitionCommand::Remove { transition } => (
                "Remove property transition",
                collection_edit(PropertyTransitionCollectionEdit::Remove { transition })?,
            ),
        };
        if !edits.is_empty() {
            self.apply(label, edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn choose_transition_property(
        &self,
        list: &PropertyTransitionList,
        resource_target: Option<&str>,
        property: &str,
        requested: Option<&str>,
        preferred: Option<&str>,
    ) -> Result<NativeTransitionChoice, String> {
        let type_ = self
            .transition_target_type(list, resource_target)?
            .ok_or("Choose a registered target for this transition collection")?;
        choose_transition(type_, property, requested, preferred).ok_or_else(|| {
            "Choose a compatible property and transition class from the current target".into()
        })
    }

    fn transition_target_type(
        &self,
        list: &PropertyTransitionList,
        resource_target: Option<&str>,
    ) -> Result<Option<&'static TypeInfo>, String> {
        let document = self.document();
        let target = if let Some(owner) = list.target_owner {
            if resource_target.is_some() {
                return Err("The authored control or style supplies this transition target".into());
            }
            setter_target_type(document, owner).or_else(|| {
                self.preview
                    .visual(owner)
                    .map(|visual| visual.get_type())
                    .or_else(|| element_type(document, owner))
            })
        } else {
            let Some(token) = resource_target else {
                return Ok(None);
            };
            Some(
                registered_type(document, list.owner, token.trim())
                    .ok_or("Choose a registered XAML target type in the resource scope")?,
            )
        };
        Ok(target.filter(|target| {
            target
                .find_property("Transitions")
                .is_some_and(|property| property.id() == Animatable::transitions_property().id())
        }))
    }
}
