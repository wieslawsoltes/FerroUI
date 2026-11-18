//! Saved stage sets exercise the same retained stage board and native hotspots as the editor.
use crate::protocol::{Image, Source};
use crate::renderer::Renderer;
use crate::scenario::{existing_workspace_path, read_source, read_text};
use crate::stages::{StageTarget, Stages, Sync};
use crate::stages_prototype::{Command, Control, Scene};
use ferroui_designer::{PreviewStageId, PreviewStageManifest};
use ferroui_designer_tools::scenario::{ScenarioPrototype, ScenarioStep};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct PrototypeRun {
    stages: Stages,
    // Stage windows must be dropped before the native platform and its dispatcher services.
    _platform: Renderer,
}

impl PrototypeRun {
    pub(crate) fn start(
        source: Source,
        spec: &ScenarioPrototype,
        base: &Path,
        workspace: &Path,
    ) -> Result<Self, String> {
        let path = existing_workspace_path(base.join(&spec.stage_set), workspace)?;
        let text = read_text(&path)?;
        let manifest = PreviewStageManifest::from_json(text.trim_start_matches('\u{feff}'))?;
        let input = resolve_sources(source, manifest, spec, base, workspace)?;
        let platform = Renderer::new();
        let mut stages = Stages::default();
        stages.sync(input, &Default::default())?;
        let mut run = Self {
            stages,
            _platform: platform,
        };
        run.control(Command::Start {
            flow: spec.flow.clone(),
        })?;
        Ok(run)
    }

    fn scene(&self) -> Result<Scene, String> {
        self.stages
            .playback
            .scene()
            .ok_or_else(|| "Start a prototype flow first".into())
    }

    fn target(&self, id: PreviewStageId) -> Result<StageTarget, String> {
        let info = self
            .stages
            .board
            .stage(id)
            .ok_or("The prototype stage was removed")?;
        Ok(StageTarget {
            id,
            epoch: info.epoch,
        })
    }

    fn control(&mut self, action: Command) -> Result<(), String> {
        let board = &self.stages.board;
        self.stages.playback.control(
            board,
            Control {
                epoch: board.epoch(),
                action,
            },
        )?;
        Ok(())
    }

    fn activate(&mut self, current: PreviewStageId, name: &str) -> Result<String, String> {
        let graph = self.stages.board.prototype();
        let mut matches = graph
            .links
            .iter()
            .enumerate()
            .filter(|(_, link)| link.source == current && link.name == name);
        let (link, _) = matches
            .next()
            .ok_or_else(|| format!("No interaction named {name:?} on the current stage"))?;
        if matches.next().is_some() {
            return Err(format!(
                "Interaction {name:?} is ambiguous on the current stage"
            ));
        }
        let stage = self.target(current)?;
        self.stages.mounted(&stage)?;
        self.control(Command::Activate { link })?;
        Ok(format!("Activated interaction {name:?}"))
    }

    fn name(&self, id: PreviewStageId) -> Result<String, String> {
        self.stages
            .board
            .stage(id)
            .map(|stage| stage.configuration.name)
            .ok_or_else(|| "The prototype stage was removed".into())
    }

    pub(crate) fn capture(&mut self) -> Result<Image, String> {
        let stage = self.target(self.scene()?.current)?;
        self.stages.image(stage)
    }

    pub(crate) fn inspect(&mut self, step: ScenarioStep) -> Result<String, String> {
        let scene = self.scene()?;
        match step {
            ScenarioStep::ActivateLink { name } => self.activate(scene.current, &name),
            ScenarioStep::ExpectStage { name } => {
                let actual = self.name(scene.screen)?;
                if actual != name {
                    return Err(format!("Expected stage {name:?}, received {actual:?}"));
                }
                Ok(format!("Showing stage {name:?}"))
            }
            ScenarioStep::ExpectOverlay { name } => {
                let actual = scene
                    .overlays
                    .last()
                    .map(|overlay| self.name(overlay.stage))
                    .transpose()?;
                if actual != name {
                    return Err(format!("Expected overlay {name:?}, received {actual:?}"));
                }
                Ok(name.map_or_else(
                    || "No overlay is open".into(),
                    |name| format!("Showing overlay {name:?}"),
                ))
            }
            step => {
                let stage = self.target(scene.current)?;
                let root = self.stages.mounted(&stage)?;
                crate::scenario_input::execute(&root, step)
            }
        }
    }
}

fn resolve_sources(
    active: Source,
    mut manifest: PreviewStageManifest,
    spec: &ScenarioPrototype,
    base: &Path,
    workspace: &Path,
) -> Result<Sync, String> {
    let active_uri = active.uri.clone();
    let mut documents = BTreeMap::from([(active_uri.clone(), active)]);
    let mut remapped = BTreeMap::<String, String>::new();
    let mut carets = BTreeMap::new();
    for stage in &mut manifest.stages {
        if let Some(uri) = remapped.get(&stage.source_uri) {
            stage.source_uri.clone_from(uri);
            continue;
        }
        let binding = spec.sources.get(&stage.source_uri);
        let requested = match binding {
            Some(binding) => base.join(&binding.path),
            None => ferroui_designer_project::file_path(&stage.source_uri).map_err(|_| {
                format!(
                    "Map the saved stage source {:?} to a workspace file",
                    stage.source_uri
                )
            })?,
        };
        let path = existing_workspace_path(requested, workspace)?;
        let caret = binding.and_then(|binding| binding.caret);
        let view = read_source(&path, caret.unwrap_or_default())?;
        let uri = view.uri.clone();
        if let Some(caret) = caret {
            if carets
                .insert(uri.clone(), caret)
                .is_some_and(|previous| previous != caret)
            {
                return Err(
                    "A stage set uses one authored RSX invocation per source document".into(),
                );
            }
            documents.insert(uri.clone(), view);
        } else {
            documents.entry(uri.clone()).or_insert(view);
        }
        remapped.insert(std::mem::replace(&mut stage.source_uri, uri.clone()), uri);
    }
    let source = documents
        .remove(&active_uri)
        .ok_or("The scenario's primary document was removed")?;
    Ok(Sync {
        source,
        manifest: Some(manifest),
        additional: documents.into_values().collect(),
    })
}
