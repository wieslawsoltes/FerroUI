use crate::document::DesignDocument;
use ferroui_designer_core::Node;
use ferroui_designer_language::{Range, event_handler_name, event_object_span, routed_events};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct EventField {
    name: String,
    #[serde(rename = "argumentType")]
    argument_type: String,
    handler: Option<String>,
    range: Option<Range>,
    editable: bool,
}

pub(crate) fn event_fields(
    design: &DesignDocument<'_>,
    node: &Node,
) -> Result<Vec<EventField>, String> {
    let rsx = design.rsx_node(node.id).map(|(_, node)| node);
    routed_events(design.schema(), &design.xaml, node)
        .into_iter()
        .map(|(name, info)| {
            let (handler, span, editable) = if let Some(rsx) = rsx {
                let setter = ferroui_designer_rsx::event_setter(&name);
                rsx.events.get(&setter).map_or((None, None, rsx.supports_events()), |event| {
                    (
                        design.source.get(event.value.range()).map(str::to_owned),
                        Some(event.value),
                        event.editable() && rsx.supports_events(),
                    )
                })
            } else {
                let object = event_object_span(&design.xaml, node, &name);
                node.attribute_info(&name)
                    .map_or((None, object, object.is_none()), |attr| {
                        (
                            Some(attr.value.clone()),
                            Some(attr.value_span),
                            event_handler_name(&attr.value),
                        )
                    })
            };
            Ok(EventField {
                name,
                argument_type: info.type_name,
                handler,
                range: span.map(|span| design.range(span)).transpose()?,
                editable,
            })
        })
        .collect()
}
