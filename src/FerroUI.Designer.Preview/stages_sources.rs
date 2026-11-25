//! Prepare authored stages with the same bounded local include resolver as the canvas.
use crate::{document::DesignDocument, projects::Projects, protocol::Source};
use ferroui_designer::{
    PreviewHostOptions, PreviewStageBoard, PreviewStageId, PreviewStageSource, PreviewStageSourceMoves,
};
use ferroui_designer_project::{
    local_preview_sources_with_resources, LocalPreviewSources, PreviewSourceBuffer,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

const OPTIONS_EPOCH: u64 = 0;

#[derive(Deserialize)]
pub(crate) struct StageDocument {
    #[serde(flatten)]
    pub(crate) authored: Source,
    #[serde(default, rename = "workspaceRoot")]
    pub(crate) workspace_root: Option<PathBuf>,
}

pub(crate) struct StageSource {
    pub(crate) authored: Source,
    preview: PreviewStageSource,
    includes: LocalPreviewSources,
}

impl StageSource {
    pub(crate) fn document(&self) -> &ferroui_designer_core::Document {
        &self.preview.document
    }

    pub(crate) fn frame_is_current(&self, board: &PreviewStageBoard, stage: PreviewStageId) -> bool {
        board.frame_is_current(stage, &self.preview.uri, self.document(), self.preview.pending, OPTIONS_EPOCH)
    }

    pub(crate) fn parse(
        input: StageDocument,
        existing: Option<&Self>,
        projects: &Projects,
        buffers: &[PreviewSourceBuffer],
    ) -> Result<Self, String> {
        let StageDocument {
            authored,
            workspace_root,
        } = input;
        let project = projects.source(&authored)?;
        let preview = match existing.filter(|source| source.authored == authored) {
            Some(source) => source.preview.clone(),
            None => PreviewStageSource {
                uri: authored.uri.clone(),
                document: DesignDocument::parse(&authored, project)?.xaml,
                pending: false,
            },
        };
        let includes = local_preview_sources_with_resources(
            workspace_root.as_deref(),
            &authored.uri,
            &preview.document,
            buffers,
            project.map(|project| &project.resources),
        );
        Ok(Self {
            authored,
            preview,
            includes,
        })
    }
}

pub(crate) fn sync_board<'a>(
    board: &mut PreviewStageBoard,
    active: &str,
    sources: impl IntoIterator<Item = (&'a str, &'a StageSource)>,
    moves: &PreviewStageSourceMoves,
) -> Result<(), String> {
    let mut documents = Vec::new();
    let mut includes = BTreeMap::new();
    for (uri, source) in sources {
        documents.push(source.preview.clone());
        includes.insert(uri, &source.includes);
    }
    board.sync_sources_after_move(
        active,
        documents,
        PreviewHostOptions::default(),
        OPTIONS_EPOCH,
        &includes,
        moves,
    )
}

pub(crate) fn watch_paths(sources: &BTreeMap<String, StageSource>) -> BTreeSet<String> {
    sources
        .values()
        .flat_map(|source| source.includes.dependencies.iter().cloned())
        .collect()
}
