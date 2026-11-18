use super::{PreviewSourceAnchor, PreviewStageBoard, PreviewStageId, TreeKind};
use crate::PreviewPrototypeElementChoice;
use std::collections::BTreeSet;

impl PreviewStageBoard {
    /// Authored hotspot choices from an accepted native stage, in visual order.
    pub fn prototype_elements(
        &self,
        id: PreviewStageId,
    ) -> Result<Vec<PreviewPrototypeElementChoice>, String> {
        let stage = &self.stages[self.index(id)?];
        let document = &self.current(stage)?.document;
        let line_breaks = document
            .source()
            .match_indices('\n')
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        Ok(self
            .runtime_tree(id, TreeKind::Visual)?
            .into_iter()
            .filter_map(|row| {
                let node = row
                    .source_id
                    .filter(|id| seen.insert(*id))
                    .and_then(|id| document.ast().node(id))?;
                let anchor = PreviewSourceAnchor::capture(document, node.id).ok()?;
                let label = match node.xaml_name() {
                    Some(name) => format!("{} · {name}", node.local_name()),
                    None => {
                        let line =
                            line_breaks.partition_point(|&index| index < node.span.start) + 1;
                        format!("{} · source line {line}", node.local_name())
                    }
                };
                Some(PreviewPrototypeElementChoice {
                    stage: id,
                    label,
                    anchor,
                })
            })
            .take(4096)
            .collect())
    }
}
