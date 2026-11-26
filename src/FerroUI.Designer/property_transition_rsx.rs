//! Bridge accepted native transition IDs to edits in their original Rust invocation.
use crate::property_transition_session::PropertyTransitionEditor;
use crate::DesignerSession;
use ferroui_designer_animation::native::TransitionPropertyChoice;
use ferroui_designer_animation::{Easing, PropertyTransitionCommand, PropertyTransitionList};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_rsx::{RsxProjection, RsxPropertyTransitionEdit};

impl DesignerSession {
    pub(super) fn rust_property_transition_list(
        &self,
        owner: NodeId,
    ) -> Result<PropertyTransitionList, String> {
        let projection = self.rust_transition_projection()?;
        let index = rust_transition_owner(self.document(), owner, &projection)?;
        map_rust_transitions(self.document(), owner, index, &projection)
    }

    pub(crate) fn rust_property_transition_editor(
        &self,
        owner: NodeId,
        projection: &RsxProjection<'_>,
    ) -> Result<PropertyTransitionEditor, String> {
        rust_property_transition_editor(self.document(), owner, projection, self.can_manipulate(), None)
    }

    pub(super) fn edit_rust_property_transition(
        &mut self,
        owner: NodeId,
        command: PropertyTransitionCommand,
    ) -> Result<(), String> {
        let projection = self.rust_transition_projection()?;
        let patches = self.rust_property_transition_plan(owner, &projection, &command)?;
        if !patches.is_empty() {
            self.commit_rust_patches(command.label(), patches)
                .map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }

    pub(crate) fn rust_property_transition_plan(
        &self,
        owner: NodeId,
        projection: &RsxProjection<'_>,
        command: &PropertyTransitionCommand,
    ) -> Result<Vec<TextPatch>, String> {
        if !self.can_manipulate() {
            return Err("Wait for the current authored Rust preview before editing a transition".into());
        }
        plan_rust_property_transition(self.document(), owner, projection, command, None)
    }

    fn rust_transition_projection(&self) -> Result<RsxProjection<'_>, String> {
        self.rust
            .as_ref()
            .ok_or("Open a Rust designer session")?
            .projection()
    }
}

pub(crate) fn rust_property_transition_editor(
    document: &Document,
    owner: NodeId,
    projection: &RsxProjection<'_>,
    can_edit: bool,
    native_options: Option<&[TransitionPropertyChoice]>,
) -> Result<PropertyTransitionEditor, String> {
    let index = rust_transition_owner(document, owner, projection)?;
    let list = map_rust_transitions(document, owner, index, projection)?;
    let can_edit = can_edit && projection.can_edit_property_transitions(index);
    let choices = if can_edit {
        rust_transition_choices(projection, index, native_options)
    } else {
        Vec::new()
    };
    Ok(PropertyTransitionEditor { list, choices, can_edit, requires_target: false })
}

pub(crate) fn plan_rust_property_transition(
    document: &Document,
    owner: NodeId,
    projection: &RsxProjection<'_>,
    command: &PropertyTransitionCommand,
    native_options: Option<&[TransitionPropertyChoice]>,
) -> Result<Vec<TextPatch>, String> {
    let index = rust_transition_owner(document, owner, projection)?;
    let list = map_rust_transitions(document, owner, index, projection)?;
    let source = |transition| {
        list.entries.iter().find(|entry| entry.id == transition)
            .map(|entry| entry.source)
            .ok_or("Choose an entry in the current Rust transition collection")
    };
    let check_type = |getter: &str, type_name: &Option<String>| {
        rust_transition_choices(projection, index, native_options).iter()
            .any(|choice| choice.property == getter && type_name.as_deref().is_none_or(|kind| kind == choice.type_name))
            .then_some(())
            .ok_or("Choose a compatible property and transition class from the current Rust control")
    };
    let edit = match command {
        PropertyTransitionCommand::Add { property, type_name, timing } => {
            check_type(property, type_name)?;
            RsxPropertyTransitionEdit::Add { getter: property, timing }
        }
        PropertyTransitionCommand::Retarget { transition, property, type_name } => {
            check_type(property, type_name)?;
            RsxPropertyTransitionEdit::Retarget { source: source(*transition)?, getter: property }
        }
        PropertyTransitionCommand::Timing { transition, timing } =>
            RsxPropertyTransitionEdit::Timing { source: source(*transition)?, timing },
        PropertyTransitionCommand::Move { transition, index } =>
            RsxPropertyTransitionEdit::Move { source: source(*transition)?, index: *index },
        PropertyTransitionCommand::Remove { transition } =>
            RsxPropertyTransitionEdit::Remove { source: source(*transition)? },
    };
    projection.edit_property_transitions(index, edit)
}

fn rust_transition_choices(
    projection: &RsxProjection<'_>,
    index: usize,
    native_options: Option<&[TransitionPropertyChoice]>,
) -> Vec<TransitionPropertyChoice> {
    projection.property_transition_choices(index).into_iter()
        .filter(|choice| native_options.is_none_or(|options| {
            options.iter().filter(|option| option.label == choice.property && option.type_name == choice.type_name)
                .count() == 1
        }))
        .map(|choice| TransitionPropertyChoice {
            property: choice.getter.into(),
            label: choice.property,
            type_name: choice.type_name,
        })
        .collect()
}

fn rust_transition_owner(
    document: &Document,
    owner: NodeId,
    projection: &RsxProjection<'_>,
) -> Result<usize, String> {
    projection.control_index(document, owner)
        .ok_or_else(|| "Select a mapped Rust control".into())
}

fn map_rust_transitions(
    document: &Document,
    owner: NodeId,
    index: usize,
    projection: &RsxProjection<'_>,
) -> Result<PropertyTransitionList, String> {
    let mut list = PropertyTransitionList::read(document, owner).map_err(|error| error.to_string())?;
    let Some(authored) = projection.property_transitions(index) else {
        list.source = projection.nodes[index].expressions.get("Transitions").copied();
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
        entry.easing = source.easing.as_ref().map(Easing::to_xaml);
        entry.delay_seconds = None;
        entry.editable_timing = vec!["Duration", "Easing"];
    }
    Ok(list)
}
