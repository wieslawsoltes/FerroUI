use crate::programs::{decode_text, encode_text};
use ferroui_designer_core::{Document, NodeId, NodeKind};

const MARKER: &str = "ferroui-designer-projected-state-trigger:";

pub struct ProjectedStateTrigger {
    pub source: NodeId,
    pub event: String,
    pub group: String,
    pub state: String,
}

impl ProjectedStateTrigger {
    pub fn comment(event: &str, group: &str, state: &str) -> String {
        format!(
            "<!--{MARKER}{}:{}:{}-->",
            encode_text(event),
            encode_text(group),
            encode_text(state)
        )
    }

    pub fn scan(document: &Document) -> Vec<Self> {
        if !document.valid_source().contains(MARKER) {
            return Vec::new();
        }
        let ast = document.ast();
        ast.nodes()
            .filter_map(|node| {
                if node.kind != NodeKind::Comment {
                    return None;
                }
                let source = node.parent.filter(|id| {
                    ast.node(*id)
                        .is_some_and(|parent| parent.kind == NodeKind::Element)
                })?;
                let fields = node.text.as_deref()?.trim().strip_prefix(MARKER)?;
                let (event, destination) = fields.split_once(':')?;
                let (group, state) = destination.split_once(':')?;
                Some(Self {
                    source,
                    event: decode_text(event)?,
                    group: decode_text(group)?,
                    state: decode_text(state)?,
                })
            })
            .collect()
    }
}
