//! Selection metadata is fetched independently of the bitmap and authored outline.
use crate::document::DesignDocument;
use crate::projects::Projects;
use crate::event_fields::{EventField, event_fields};
use crate::property_fields::{PropertyField, property_fields};
use crate::protocol::Source;
use crate::renderer::Renderer;
use ferroui_designer_core::NodeId;
use ferroui_inspect::PropertyInfo;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(crate) struct InspectionRequest {
    pub(crate) frame: u64,
    #[serde(flatten)]
    pub(crate) source: Source,
    pub(crate) target: InspectionTarget,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum InspectionTarget {
    Source { node: NodeId },
    Selection { nodes: Vec<NodeId> },
    Runtime { id: u64 },
}

#[derive(Serialize)]
pub(crate) struct Inspection {
    pub(crate) properties: Vec<PropertyField>,
    pub(crate) events: Vec<EventField>,
}

pub(crate) fn inspect(
    input: &InspectionRequest,
    renderer: Option<&Renderer>,
    projects: &Projects,
) -> Result<Inspection, String> {
    let stage = renderer.and_then(|renderer| renderer.stage(input.frame).ok());
    match input.target {
        InspectionTarget::Source { node } => {
            let design = DesignDocument::parse(&input.source, projects.source(&input.source)?)?;
            let element = design
                .xaml
                .ast()
                .node(node)
                .ok_or("Unknown source element")?;
            let session = stage.map(|stage| stage.session.borrow());
            let effective = session.as_ref().map_or_else(Vec::new, |session| session.authored_preview_properties(node));
            Ok(Inspection {
                properties: property_fields(&design, element, effective,
                    |property| session.as_ref().is_some_and(|session| session.can_record_property(node, property))),
                events: event_fields(&design, element)?,
            })
        }
        InspectionTarget::Selection { ref nodes } => {
            let distinct: std::collections::BTreeSet<_> = nodes.iter().copied().collect();
            if !(2..=128).contains(&nodes.len()) || distinct.len() != nodes.len() {
                return Err("Inspect between two and 128 distinct source elements".into());
            }
            let design = DesignDocument::parse(&input.source, projects.source(&input.source)?)?;
            let session = stage.ok_or("Wait for the current preview")?.session.borrow();
            let mut fields = nodes.iter().rev().map(|id| {
                let node = design.xaml.ast().node(*id).ok_or("Unknown selected element")?;
                Ok::<_, &str>(property_fields(&design, node, session.authored_preview_properties(*id),
                    |property| session.can_record_property(*id, property)))
            });
            let mut properties = fields.next().expect("selection length validated")?;
            for fields in fields {
                crate::property_fields::intersect(&mut properties, fields?);
            }
            Ok(Inspection { properties, events: Vec::new() })
        }
        InspectionTarget::Runtime { id } => {
            let properties = stage
                .ok_or("Wait for the current preview")?
                .session
                .borrow()
                .inspected_preview_properties(id)?;
            Ok(Inspection {
                properties: properties.iter().map(runtime_property).collect(),
                events: Vec::new(),
            })
        }
    }
}

fn runtime_property(info: &PropertyInfo) -> PropertyField {
    PropertyField::runtime(
        &info.name,
        &info.priority_name(),
        ferroui_inspect::display_value(Some(&info.value)),
    )
}
