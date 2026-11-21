use crate::document::DesignDocument;
use ferroui_designer_core::{MarkupValue, Node, NodeKind};

pub(crate) fn can_set_image_source(design: &DesignDocument<'_>, node: &Node) -> bool {
    if let Some((_, rsx)) = design.rsx_node(node.id) {
        return rsx.can_set_image_source();
    }
    if node.local_name() != "Image" || !matches!(node.namespace_uri.as_deref(), None | Some("https://github.com/ferroui")) {
        return false;
    }
    // Preserve bindings and property elements unless they are edited explicitly in source.
    let object_source = node.children.iter().filter_map(|id| design.xaml.ast().node(*id)).any(|child| {
        child.kind == NodeKind::PropertyElement && child.local_name().ends_with(".Source")
    });
    !object_source && node.attributes.iter().filter(|attribute| {
        attribute.name == "Source" || attribute.name.ends_with(".Source")
    }).all(|attribute| attribute.name == "Source" && matches!(attribute.markup(), Ok(MarkupValue::Literal(_))))
}
