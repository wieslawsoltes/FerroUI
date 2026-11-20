//! Retarget retained stage cards before accepting their next source snapshots.
use super::{validate_uri, PreviewStageBoard, PreviewStageStatus, MAX_PREVIEW_STAGES};
use std::collections::BTreeMap;

/// A validated set of source identities for one accepted file operation.
#[derive(Default)]
pub struct PreviewStageSourceMoves(BTreeMap<String, String>);

impl PreviewStageSourceMoves {
    pub fn new(moves: BTreeMap<String, String>) -> Result<Self, String> {
        if moves.len() > MAX_PREVIEW_STAGES {
            return Err("A source move can retarget at most 16 stage sources".into());
        }
        for uri in moves.values() {
            validate_uri(uri)?;
        }
        Ok(Self(moves))
    }
}

impl PreviewStageBoard {
    /// Call after a successful Save As, before synchronizing the new URI snapshots.
    /// The old graph stays visible but immediately loses source-selection authority.
    pub fn retarget_source(&mut self, old_uri: &str, new_uri: &str) -> Result<(), String> {
        validate_uri(new_uri)?;
        if old_uri != new_uri {
            let moves = PreviewStageSourceMoves(BTreeMap::from([(old_uri.into(), new_uri.into())]));
            self.retarget_sources(&moves);
        }
        Ok(())
    }

    /// Keep the native hosts and their presentation state while retiring old frame authority.
    /// Each mapping is resolved against the previous source identity, including moves undone
    /// before the next render.
    pub fn retarget_sources(&mut self, moves: &PreviewStageSourceMoves) {
        let mut changed = Vec::new();
        for (index, stage) in self.stages.iter_mut().enumerate() {
            let source = &mut stage.spec.configuration.source_uri;
            if let Some(destination) = moves.0.get(source) {
                source.clone_from(destination);
                stage.observed = None;
                stage.status = PreviewStageStatus::MissingDocument;
                stage.epoch = stage
                    .epoch
                    .checked_add(1)
                    .expect("Preview stage epoch exhausted");
                changed.push(index);
            }
        }
        if !changed.is_empty() {
            for index in changed {
                self.apply_mode(index);
            }
            self.bump();
        }
    }
}
