//! Prepare authored stages with the same bounded local include resolver as the canvas.
use crate::{document::DesignDocument, projects::Projects, protocol::Source};
use ferroui_designer::{PreviewHostOptions, PreviewStageBoard, PreviewStageSource, PreviewStageSourceMoves};
use ferroui_designer_project::{local_preview_sources, LocalPreviewSources, PreviewSourceBuffer};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

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
        let includes = local_preview_sources(
            workspace_root.as_deref(),
            &authored.uri,
            &preview.document,
            buffers,
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
        0,
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
