//! Selection metadata is fetched independently of the bitmap and authored outline.
use crate::document::DesignDocument;
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
) -> Result<Inspection, String> {
    let stage = renderer.and_then(|renderer| renderer.stage(input.frame).ok());
    match input.target {
        InspectionTarget::Source { node } => {
            let design = DesignDocument::parse(&input.source)?;
            let element = design
                .xaml
                .ast()
                .node(node)
                .ok_or("Unknown source element")?;
            let effective = stage.map_or_else(Vec::new, |stage| {
                stage.session.borrow().authored_preview_properties(node)
            });
            Ok(Inspection {
                properties: property_fields(&design, element, effective),
                events: event_fields(&design, element)?,
            })
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
