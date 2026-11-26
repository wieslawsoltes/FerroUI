//! Native type checks for edits to an authored XAML transition collection.
use super::{
    choose_transition, element_type, registered_type, registered_xml_type, setter_target_type,
};
use crate::{PropertyTransitionCollectionEdit, PropertyTransitionCommand, PropertyTransitionList};
use ferroui_base::{animation::Animatable, TypeInfo};
use ferroui_designer_core::{Document, Edit, MarkupValue, NodeId};

/// Resolve the property scope of a native collection. An explicit type applies
/// to standalone resource collections, whose consumers can have different types.
pub fn transition_target_type(
    document: &Document,
    list: &PropertyTransitionList,
    resource_target: Option<&str>,
    live_type: impl Fn(NodeId) -> Option<&'static TypeInfo>,
) -> Result<Option<&'static TypeInfo>, String> {
    let target = if let Some(owner) = list.target_owner {
        if resource_target.is_some() {
            return Err("The authored control or style supplies this transition target".into());
        }
        setter_target_type(document, owner)
            .or_else(|| live_type(owner))
            .or_else(|| element_type(document, owner))
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

/// Plan source edits against the same registered types used to load the preview.
pub fn plan_xaml_property_transition(
    document: &Document,
    list: &PropertyTransitionList,
    resource_target: Option<&str>,
    live_type: impl Fn(NodeId) -> Option<&'static TypeInfo>,
    command: &PropertyTransitionCommand,
) -> Result<Vec<Edit>, String> {
    let owner = list.owner;
    let choose = |property: &str, requested: Option<&str>, preferred: Option<&str>| {
        let target = transition_target_type(document, list, resource_target, &live_type)?
            .ok_or("Choose a registered target for this transition collection")?;
        choose_transition(target, property, requested, preferred).ok_or_else(|| {
            "Choose a compatible property and transition class from the current target".to_owned()
        })
    };
    let collection_edit = |edit| {
        PropertyTransitionList::plan_collection_edit(document, owner, edit)
            .map_err(|error| error.to_string())
    };
    match command {
        PropertyTransitionCommand::Timing { transition, timing } => {
            PropertyTransitionList::plan_timing(document, owner, *transition, timing)
                .map_err(|error| error.to_string())
        }
        PropertyTransitionCommand::Add {
            property,
            type_name,
            timing,
        } => {
            let choice = choose(property, type_name.as_deref(), None)?;
            let target = choice.xaml_target(
                document,
                list.collection.unwrap_or(owner),
                list.target_owner.is_none(),
                registered_xml_type,
            );
            collection_edit(PropertyTransitionCollectionEdit::Add {
                target: &target,
                timing,
            })
        }
        PropertyTransitionCommand::Retarget {
            transition,
            property,
            type_name,
        } => {
            let entry = list
                .entries
                .iter()
                .find(|entry| entry.id == *transition)
                .ok_or("The transition is outside the selected collection")?;
            let choice = choose(property, type_name.as_deref(), Some(&entry.type_name))?;
            let has_static_property = document
                .ast()
                .node(*transition)
                .and_then(|node| node.attribute("Property"))
                .is_some_and(|value| {
                    matches!(
                        MarkupValue::parse(value.trim()),
                        Ok(MarkupValue::Extension(_))
                    )
                });
            let target = choice.xaml_target(
                document,
                *transition,
                list.target_owner.is_none() || has_static_property,
                registered_xml_type,
            );
            collection_edit(PropertyTransitionCollectionEdit::Retarget {
                transition: *transition,
                target: &target,
            })
        }
        PropertyTransitionCommand::Move { transition, index } => {
            collection_edit(PropertyTransitionCollectionEdit::Move {
                transition: *transition,
                index: *index,
            })
        }
        PropertyTransitionCommand::Remove { transition } => {
            collection_edit(PropertyTransitionCollectionEdit::Remove {
                transition: *transition,
            })
        }
    }
}
