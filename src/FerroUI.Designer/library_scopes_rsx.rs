use super::{finish, push, LibraryObject, LibraryScope};
use ferroui_designer_core::{Document, NodeId, NodeKind};
use ferroui_designer_rsx::RsxProjection;
use std::collections::BTreeMap;

pub(super) fn scopes(
    document: &Document,
    projection: &RsxProjection<'_>,
    root: Option<NodeId>,
) -> Vec<LibraryScope> {
    let offsets = projection.preview_markup().controls;
    let ast = document.ast();
    let inside = |node| root.is_none_or(|root| node == root || ast.is_descendant_of(node, root));
    let mut controls = vec![None; offsets.len()];
    for node in ast
        .nodes_in_source_order()
        .filter(|node| node.kind == NodeKind::Element)
    {
        if let Ok(index) = offsets.binary_search(&node.span.start) {
            controls[index] = Some(node.id);
        }
    }
    let mut groups = BTreeMap::new();
    for (index, style) in projection.style_definitions() {
        let Some(owner) = controls[index].filter(|&owner| inside(owner)) else {
            continue;
        };
        push(
            &mut groups,
            owner,
            LibraryObject::local(
                format!("rust-style:{}", style.declaration.start),
                style.selector.clone(),
                "Style",
                style.selector_span,
            ),
        );
        for resource in &style.resources {
            let mut object = LibraryObject::local(
                format!("rust-resource:{}", resource.declaration.start),
                resource.key.clone(),
                "Resource",
                resource.declaration,
            );
            object.detail = Some(style.selector.clone());
            push(&mut groups, owner, object);
        }
    }
    for (owner, property, location) in projection.authored_item_templates(document) {
        if !inside(owner) {
            continue;
        }
        let mut object = LibraryObject::local(
            format!("rust-template:{owner}:{}", property.name()),
            property.name().into(),
            "Template",
            location.span,
        );
        object.node = Some(owner);
        object.source_uri = location.uri;
        push(&mut groups, owner, object);
    }
    finish(
        document,
        groups,
        |node| {
            let index = offsets.binary_search(&node.span.start).ok()?;
            Some(projection.nodes[index].name)
        },
        |_| None,
    )
}
