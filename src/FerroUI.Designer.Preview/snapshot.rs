use crate::document::DesignDocument;
use crate::edits::{addable_properties, can_insert, can_remove, editable_attribute};
use crate::protocol::{OutlineNode, Property, Render, Snapshot};
use crate::renderer::Renderer;

pub(crate) fn render(renderer: &Renderer, input: &Render) -> Result<Snapshot, String> {
    let design = DesignDocument::parse(&input.document)?;
    let counterpart = design.counterpart();
    let mut notices = Vec::new();
    let (blocks, active_block) = if let Some(rsx) = &design.rsx {
        if rsx.omitted_count != 0 {
            notices.push(format!(
                "Static preview · {} omitted Rust items ({})",
                rsx.omitted_count,
                rsx.omissions.join(", ")
            ));
        }
        (
            rsx.blocks
                .iter()
                .map(|span| design.range(*span))
                .collect::<Result<_, _>>()?,
            Some(design.range(rsx.invocation)?),
        )
    } else {
        (Vec::new(), None)
    };
    let (image, bounds, render_error) = match renderer.render(input, design.xaml.source()) {
        Ok(frame) => {
            notices.extend(frame.notices);
            (Some(frame.image), frame.bounds, None)
        }
        Err(error) => (None, Default::default(), Some(error)),
    };
    let nodes = design
        .xaml
        .ast()
        .nodes_in_source_order()
        .filter(|node| node.is_element())
        .map(|node| {
            let (span, name_span) = match design.rsx_node(node.id) {
                Some((_, rsx)) => (rsx.span, rsx.name),
                None => (node.span, node.name_span),
            };
            let properties = node
                .attributes
                .iter()
                .filter(|attr| !attr.is_namespace())
                .map(|attr| Property {
                    name: attr.name.clone(),
                    value: attr.value.clone(),
                    editable: editable_attribute(&design, node, &attr.name),
                })
                .collect();
            Ok(OutlineNode {
                id: node.id,
                parent: node.parent,
                label: node.name.clone(),
                name: node.xaml_name().map(str::to_owned),
                range: design.range(span)?,
                name_range: design.range(name_span)?,
                bounds: bounds.get(&node.id).copied(),
                properties,
                addable_properties: addable_properties(&design, node),
                can_insert: can_insert(&design, node),
                can_remove: can_remove(&design, node),
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Snapshot {
        nodes,
        blocks,
        active_block,
        counterpart,
        image,
        render_error,
        notices,
    })
}
