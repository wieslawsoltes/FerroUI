use crate::document::DesignDocument;
use crate::protocol::DragOrigin;
use ferroui_designer_core::{ChangeSet, Document, NodeId, NodeKind, TextPatch};
use ferroui_designer_language::{LineIndex, Range};

pub(crate) fn dropped_node(
    document: &Document,
    change: &ChangeSet,
    origin: &DragOrigin,
) -> Option<NodeId> {
    match origin {
        DragOrigin::Node { node, .. } => Some(*node),
        DragOrigin::Nodes { anchor, .. } => Some(*anchor),
        DragOrigin::Asset { .. } | DragOrigin::Image { .. } => change.inserted.iter().copied().find(|id| {
            document.ast().node(*id).is_some_and(|node| {
                node.kind == NodeKind::Element
                    && node
                        .parent
                        .is_none_or(|parent| !change.inserted.contains(&parent))
            })
        }),
    }
}

pub(crate) fn authored_range(
    design: &DesignDocument<'_>,
    draft: &Document,
    patches: &[TextPatch],
    node: NodeId,
) -> Option<Range> {
    let ast = draft.ast();
    if let Some(rsx) = &design.rsx {
        let index = ast
            .nodes_in_source_order()
            .filter(|node| node.is_element())
            .position(|entry| entry.id == node)?;
        let mut edited = design.source.to_owned();
        let mut ordered: Vec<_> = patches.iter().collect();
        ordered.sort_by_key(|patch| std::cmp::Reverse(patch.span.start));
        for patch in ordered {
            edited.replace_range(patch.span.range(), &patch.replacement);
        }
        let projection = design.rsx_catalog().project(&edited, rsx.invocation.start).ok()?;
        LineIndex::new(&edited)
            .range(projection.nodes.get(index)?.name)
            .ok()
    } else {
        LineIndex::new(draft.source())
            .range(ast.node(node)?.name_span)
            .ok()
    }
}
