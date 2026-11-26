use crate::RsxProjection;
use ferroui_designer_core::NodeId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct RemovedSubtrees {
    pub(super) indices: BTreeSet<usize>,
    pub(super) omitted: usize,
    pub(super) animation_issues: usize,
    pub(super) style_issues: usize,
}

impl RemovedSubtrees {
    pub(super) fn record(
        projection: &RsxProjection<'_>,
        removals: BTreeMap<usize, NodeId>,
        departures: &mut BTreeMap<NodeId, Vec<usize>>,
    ) -> Self {
        let mut removed = Self::default();
        for (index, parent) in removals {
            if removed.indices.contains(&index) {
                continue;
            }
            let subtree = &projection.nodes[index].subtree;
            removed.indices.extend(index..subtree.end);
            removed.omitted += subtree.omitted;
            removed.animation_issues += subtree.animation_issues;
            removed.style_issues += subtree.style_issues;
            departures.entry(parent).or_default().push(index);
        }
        removed
    }
}
