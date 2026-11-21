//! Restores local layer presentation across separately constructed native previews.
use super::LayerFlags;
use crate::{DesignerSession, PreviewSourceAnchor};
use ferroui_designer_core::NodeKind;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

const MAX_LOCATORS: usize = 4096;
const MAX_ANCHOR_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerViewState {
    source_stamp: String,
    locked: Vec<Locator>,
    hidden: Vec<Locator>,
    isolation: Vec<Isolation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Locator {
    start: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anchor: Option<PreviewSourceAnchor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Isolation {
    scope: Option<Locator>,
    roots: Vec<Locator>,
}

impl LayerViewState {
    pub fn is_empty(&self) -> bool {
        self.locked.is_empty() && self.hidden.is_empty() && self.isolation.is_empty()
    }

    /// Identifies the projected markup in which the layer offsets were captured.
    pub fn stamp_source(source: &str) -> String {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl DesignerSession {
    pub fn persist_layer_view(&self) -> LayerViewState {
        let document = self.document();
        let mut remaining = MAX_LOCATORS;
        let mut anchor_bytes = MAX_ANCHOR_BYTES;
        let mut locate = |id| {
            if remaining == 0 {
                return None;
            }
            let node = document
                .ast()
                .node(id)
                .filter(|node| node.kind == NodeKind::Element)?;
            remaining -= 1;
            let anchor = PreviewSourceAnchor::capture(document, id)
                .ok()
                .filter(|anchor| {
                    let size = anchor.text_size();
                    if size > anchor_bytes {
                        return false;
                    }
                    anchor_bytes -= size;
                    true
                });
            Some(Locator {
                start: node.span.start,
                anchor,
            })
        };
        let flags = &self.layers.flags;
        LayerViewState {
            source_stamp: LayerViewState::stamp_source(document.source()),
            locked: flags
                .locked
                .iter()
                .copied()
                .filter_map(&mut locate)
                .collect(),
            hidden: flags
                .hidden
                .iter()
                .copied()
                .filter_map(&mut locate)
                .collect(),
            isolation: flags
                .isolation
                .iter()
                .filter_map(|(&scope, roots)| {
                    let scope = match scope {
                        Some(node) => Some(locate(node)?),
                        None => None,
                    };
                    let roots: Vec<_> = roots.iter().copied().filter_map(&mut locate).collect();
                    (!roots.is_empty()).then_some(Isolation { scope, roots })
                })
                .collect(),
        }
    }

    pub fn restore_persisted_layer_view(&mut self, state: &LayerViewState) {
        let document = self.document();
        if !document.is_valid() {
            return;
        }
        let exact = state.source_stamp == LayerViewState::stamp_source(document.source());
        let mut by_offset = BTreeMap::new();
        if exact {
            by_offset.extend(
                document
                    .ast()
                    .elements()
                    .map(|node| (node.span.start, node.id)),
            );
        }
        let mut remaining = MAX_LOCATORS;
        let mut resolve = |target: &Locator| {
            if remaining == 0 {
                return None;
            }
            remaining -= 1;
            if exact {
                return by_offset.get(&target.start).copied();
            }
            target
                .anchor
                .as_ref()
                .and_then(|anchor| anchor.resolve(document).ok())
        };
        self.layers.flags = LayerFlags {
            locked: state.locked.iter().filter_map(&mut resolve).collect(),
            hidden: state.hidden.iter().filter_map(&mut resolve).collect(),
            isolation: state
                .isolation
                .iter()
                .filter_map(|entry| {
                    let scope = match &entry.scope {
                        Some(locator) => Some(resolve(locator)?),
                        None => None,
                    };
                    let roots: Vec<_> = entry.roots.iter().filter_map(&mut resolve).collect();
                    (!roots.is_empty()).then_some((scope, roots))
                })
                .collect(),
        };
        self.layer_view_changed();
    }
}
