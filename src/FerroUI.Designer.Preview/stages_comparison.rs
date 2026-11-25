use super::{StageTarget, Stages};
use ferroui_designer::{
    PreviewStageComparison, PreviewStageMatrix, PreviewStageSpec, MAX_PREVIEW_STAGES,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    base: StageTarget,
    comparison: Comparison,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Comparison {
    Preset { preset: PreviewStageComparison },
    Matrix { matrix: PreviewStageMatrix },
}

#[derive(Serialize)]
pub(crate) struct Plan {
    epoch: u64,
    available: usize,
    pub(super) configurations: Vec<PreviewStageSpec>,
}

impl Stages {
    pub(crate) fn plan_comparison(&self, request: Request) -> Result<Plan, String> {
        let stage = self.stage_info(&request.base)?;
        let source = self
            .sources
            .get(&stage.configuration.source_uri)
            .ok_or("Open the base stage's source document first")?;
        if !source.frame_is_current(&self.board, stage.id) {
            return Err(
                "Refresh the base stage to its current source before comparing variants.".into(),
            );
        }
        let available = MAX_PREVIEW_STAGES.saturating_sub(self.board.stages().len());
        let configurations = match request.comparison {
            Comparison::Preset { preset } => self
                .board
                .plan_variants(stage.id, preset.variants(&stage.configuration))?,
            Comparison::Matrix { matrix } => {
                let spec = self
                    .board
                    .stage_spec(stage.id)
                    .ok_or("The base stage was removed")?;
                matrix
                    .expand(&spec.configuration, source.document(), available)?
                    .into_iter()
                    .map(|configuration| PreviewStageSpec {
                        configuration,
                        design_fixture: spec.design_fixture.clone(),
                    })
                    .collect()
            }
        };
        Ok(Plan {
            epoch: self.board.epoch(),
            available,
            configurations,
        })
    }
}
