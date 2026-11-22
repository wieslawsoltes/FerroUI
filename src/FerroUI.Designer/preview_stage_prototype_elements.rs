use super::{PreviewSourceAnchor, PreviewStageBoard, PreviewStageId, PreviewStageMode, TreeKind};
use crate::{PreviewPrototypeElementChoice, PreviewPrototypeSession};
use std::collections::{BTreeMap, BTreeSet};

/// Accepted authoring inputs for a play session. Realize these after releasing the editor board.
pub struct PreviewPrototypeSources {
    board: PreviewStageBoard,
    originals: Vec<PreviewStageId>,
    specs: Vec<super::PreviewStageSpec>,
}

impl PreviewPrototypeSources {
    pub fn realize(mut self) -> Result<(PreviewStageBoard, BTreeMap<PreviewStageId, PreviewStageId>), String> {
        let ids = self.board.add_specs(self.specs)?;
        Ok((self.board, self.originals.into_iter().zip(ids).collect()))
    }
}

impl PreviewStageBoard {
    /// Collect source and settings for the reachable stages of an accepted flow.
    pub fn prototype_sources(&self, session: &PreviewPrototypeSession)
        -> Result<PreviewPrototypeSources, String>
    {
        let graph = session.graph();
        graph.validate(&self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        let reachable = graph.reachable(session.screen());
        let mut specs = Vec::new();
        let mut originals = Vec::new();
        for stage in self.stages.iter().filter(|stage| reachable.contains(&stage.id)) {
            let config = &stage.spec.configuration;
            let source = self.sources.get(&config.source_uri)
                .ok_or("Open every screen source in the selected flow before playing it")?;
            if source.pending || !source.document.is_valid() {
                return Err("Apply or repair the flow's pending source before playing it".into());
            }
            for link in graph.links.iter().filter(|link| link.source == stage.id) {
                link.trigger.resolve(&source.document)?;
            }
            let mut spec = stage.spec.clone();
            spec.configuration.mode = PreviewStageMode::Live;
            spec.configuration.auto_update = true;
            spec.configuration.pinned = true;
            originals.push(stage.id);
            specs.push(spec);
        }
        let mut board = Self::new();
        board.active_uri = self.active_uri.clone();
        board.sources = self.sources.clone();
        board.includes = self.includes.clone();
        board.options = self.options.clone();
        board.options_epoch = self.options_epoch;
        Ok(PreviewPrototypeSources { board, originals, specs })
    }

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
