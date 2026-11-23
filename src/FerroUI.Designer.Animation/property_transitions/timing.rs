use super::{children, PropertyTransitionList};
use crate::programs::native_fragment;
use crate::xaml::{native_ticks, parse_time, time_string};
use crate::{require, AnimationError, Easing, Result};
use ferroui_designer_core::{Document, Edit, Node, NodeId};
use serde::{Deserialize, Serialize};

pub(super) const FIELDS: [&str; 3] = ["Duration", "Delay", "Easing"];

/// Changes to literal transition timing. An omitted field retains its authored form.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyTransitionTiming {
    pub duration: Option<f64>,
    pub delay: Option<f64>,
    pub easing: Option<Easing>,
}

impl PropertyTransitionList {
    pub fn plan_timing(
        document: &Document,
        owner: NodeId,
        transition: NodeId,
        timing: &PropertyTransitionTiming,
    ) -> Result<Vec<Edit>> {
        require(
            document.is_valid(),
            "Repair the XAML before editing transition timing",
        )?;
        let source = Self::read(document, owner)?;
        let entry = source
            .entries
            .iter()
            .find(|entry| entry.id == transition)
            .ok_or_else(|| {
                AnimationError("The transition is outside the selected collection".into())
            })?;
        let node = document
            .ast()
            .node(transition)
            .expect("an authored transition exists in the document");
        let mut edits = Vec::new();
        for (field, seconds) in [("Duration", timing.duration), ("Delay", timing.delay)] {
            if let Some(seconds) = seconds {
                require(
                    entry.editable_timing.contains(&field),
                    format!("Edit {field} at its source"),
                )?;
                let accepted = crate::xaml::native_key_time(seconds)?;
                if node
                    .attribute(field)
                    .and_then(|value| parse_time(value).ok())
                    != Some(accepted)
                {
                    edits.push(set(transition, field, time_string(seconds)));
                }
            }
        }
        validate_total_time(document, node, timing)?;

        if let Some(easing) = &timing.easing {
            require(
                entry.editable_timing.contains(&"Easing"),
                "Edit Easing at its source",
            )?;
            easing.validate()?;
            let current = node
                .attribute("Easing")
                .and_then(|value| Easing::parse(value).ok());
            if current.as_ref() != Some(easing) {
                edits.extend(easing_edits(document, node, easing)?);
            }
        }
        Ok(edits)
    }
}

pub(super) fn editable(document: &Document, node: &Node, field: &str) -> bool {
    !property_element(document, node, field)
        && node.attribute(field).is_none_or(|value| match field {
            "Easing" => Easing::parse(value).is_ok(),
            _ => parse_time(value).is_ok(),
        })
}

fn property_element(document: &Document, node: &Node, field: &str) -> bool {
    children(document, node).any(|child| {
        child
            .local_name()
            .split_once('.')
            .is_some_and(|(_, property)| property == field)
    })
}

fn validate_total_time(
    document: &Document,
    node: &Node,
    timing: &PropertyTransitionTiming,
) -> Result<()> {
    if timing.duration.is_none() && timing.delay.is_none() {
        return Ok(());
    }
    let ticks = |value: Option<f64>, field| match value {
        Some(seconds) => native_ticks(seconds).ok(),
        None if property_element(document, node, field) => None,
        None => node.attribute(field).map_or(Some(0), |value| {
            parse_time(value).and_then(native_ticks).ok()
        }),
    };
    if let (Some(duration), Some(delay)) = (
        ticks(timing.duration, "Duration"),
        ticks(timing.delay, "Delay"),
    ) {
        require(
            duration + delay <= i64::MAX as u64,
            "Transition delay and duration exceed the native clock range",
        )?;
    }
    Ok(())
}

fn easing_edits(document: &Document, node: &Node, easing: &Easing) -> Result<Vec<Edit>> {
    let Easing::Spring {
        mass,
        stiffness,
        damping,
        initial_velocity,
    } = easing
    else {
        return Ok(vec![set(node.id, "Easing", easing.to_xaml())]);
    };
    let property = format!("{}.Easing", node.name);
    let spring = native_fragment(document, node.id, &format!(
        "<SpringEasing Mass=\"{mass}\" Stiffness=\"{stiffness}\" Damping=\"{damping}\" InitialVelocity=\"{initial_velocity}\"/>"
    ))?;
    let mut edits = Vec::new();
    if node.attribute("Easing").is_some() {
        edits.push(Edit::RemoveAttribute {
            node: node.id,
            name: "Easing".into(),
        });
    }
    edits.push(Edit::InsertChild {
        parent: node.id,
        index: document.ast().element_children(node.id).len(),
        source: format!("<{property}>{spring}</{property}>"),
    });
    Ok(edits)
}

fn set(node: NodeId, field: &str, value: String) -> Edit {
    Edit::SetAttribute {
        node,
        name: field.into(),
        value,
    }
}
