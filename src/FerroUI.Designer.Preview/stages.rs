use crate::stages_prototype::Playback;
use crate::{
    document::DesignDocument,
    projects::Projects,
    protocol::{Image, Source},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ferroui_base::{Point, Ref, threading::Dispatcher};
use ferroui_controls::{Control, SizeToContent, Window, WindowDecorations};
use ferroui_designer::{
    PreviewEnvironment, PreviewFinding, PreviewHostOptions, PreviewPrototypeElementChoice,
    PreviewPrototypeManifest, PreviewStageBoard, PreviewStageConfiguration, PreviewStageId,
    PreviewStageInfo, PreviewStageManifest, PreviewStageMode, PreviewStageSource, PreviewStageStateOptions,
    PreviewStageStatus, PreviewTheme, capture_preview_stage_png,
};
use ferroui_designer_language::Range;
use ferroui_inspect::{Inspector, Permissions};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Default)]
pub(crate) struct Stages {
    pub(crate) board: PreviewStageBoard,
    windows: BTreeMap<PreviewStageId, StageWindow>,
    sources: BTreeMap<String, StageSource>,
    active: Option<String>,
    pub(crate) playback: Playback,
}

struct StageSource {
    authored: Source,
    preview: PreviewStageSource,
}

impl StageSource {
    fn parse(authored: Source, existing: Option<&Self>, projects: &Projects) -> Result<Self, String> {
        let project = projects.source(&authored)?;
        let preview = match existing.filter(|source| source.authored == authored) {
            Some(source) => source.preview.clone(),
            None => PreviewStageSource {
                uri: authored.uri.clone(),
                document: DesignDocument::parse(&authored, project)?.xaml,
                pending: false,
            },
        };
        Ok(Self { authored, preview })
    }
}

#[derive(Deserialize)]
pub(crate) struct Sync {
    pub(crate) source: Source,
    pub(crate) manifest: Option<PreviewStageManifest>,
    #[serde(default)]
    pub(crate) additional: Vec<Source>,
}

#[derive(Deserialize)]
pub(crate) struct Change {
    epoch: u64,
    #[serde(flatten)]
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Action {
    Add {
        name: String,
        width: f64,
        height: f64,
        dark: bool,
        source: Option<Source>,
    },
    Remove {
        id: PreviewStageId,
    },
    Update {
        id: PreviewStageId,
        configuration: PreviewStageConfiguration,
    },
    Refresh {
        id: PreviewStageId,
    },
    SetPrototype {
        graph: PreviewPrototypeManifest,
    },
}

#[derive(Serialize)]
pub(crate) struct Model {
    epoch: u64,
    manifest: PreviewStageManifest,
    stages: Vec<PreviewStageInfo>,
    prototype: Option<crate::stages_prototype::Scene>,
}

#[derive(Deserialize)]
pub(crate) struct StageTarget {
    pub(crate) id: PreviewStageId,
    pub(crate) epoch: u64,
}

#[derive(Serialize)]
pub(crate) struct StageChecks {
    uri: String,
    findings: Vec<StageFinding>,
}

#[derive(Serialize)]
struct StageFinding {
    #[serde(flatten)]
    finding: PreviewFinding,
    range: Option<Range>,
}

#[derive(Deserialize)]
pub(crate) struct Pointer {
    #[serde(flatten)]
    target: StageTarget,
    x: f64,
    y: f64,
}

impl Stages {
    pub(crate) fn sync(&mut self, request: Sync, projects: &Projects) -> Result<Model, String> {
        let uri = request.source.uri.clone();
        if request.additional.len() >= ferroui_designer::MAX_PREVIEW_STAGES {
            return Err("Open at most 16 preview documents".into());
        }
        let mut sources = BTreeMap::new();
        for source in std::iter::once(request.source).chain(request.additional) {
            let key = source.uri.clone();
            if sources.contains_key(&key) {
                return Err("Duplicate preview source URI".into());
            }
            sources.insert(
                key.clone(),
                StageSource::parse(source, self.sources.get(&key), projects)?,
            );
        }
        let epoch = self.board.epoch();
        self.board.sync_sources(
            &uri,
            sources
                .values()
                .map(|source| source.preview.clone())
                .collect(),
            PreviewHostOptions::default(),
            0,
        )?;
        if let Some(manifest) = request.manifest {
            self.board.restore_manifest(manifest)?;
        } else if self.active.is_none() {
            self.board.add(PreviewStageConfiguration::new(
                "Desktop",
                &uri,
                PreviewEnvironment::default(),
            ))?;
            self.board.add(PreviewStageConfiguration::new(
                "Compact",
                &uri,
                PreviewEnvironment {
                    width: 390.0,
                    height: 680.0,
                    theme: PreviewTheme::Dark,
                    ..Default::default()
                },
            ))?;
        }
        if epoch != self.board.epoch() {
            self.playback.stop();
        }
        self.active = Some(uri);
        self.sources = sources;
        Ok(self.model())
    }

    pub(crate) fn edit(&mut self, request: Change, projects: &Projects) -> Result<Model, String> {
        if request.epoch != self.board.epoch() {
            return Err("Read the current Stages board first".into());
        }
        self.active.as_ref().ok_or("Open a Stages board")?;
        let epoch = self.board.epoch();
        match request.action {
            Action::Add {
                name,
                width,
                height,
                dark,
                source,
            } => self.add(name, width, height, dark, source, projects)?,
            Action::Remove { id } => self.board.remove(id)?,
            Action::Update { id, configuration } => {
                let previous = self.board.stage(id).ok_or("The stage was removed")?;
                if configuration.source_uri != previous.configuration.source_uri {
                    return Err("The stage source changed. Reopen its document in Stages.".into());
                }
                self.board.update(id, configuration)?;
            }
            Action::Refresh { id } => self.board.refresh(id)?,
            Action::SetPrototype { graph } => {
                let ids = self
                    .board
                    .stages()
                    .into_iter()
                    .map(|stage| stage.id)
                    .collect::<Vec<_>>();
                self.board.set_prototype(graph.restore(&ids)?)?;
            }
        }
        if epoch != self.board.epoch() {
            self.playback.stop();
        }
        Ok(self.model())
    }

    fn add(
        &mut self,
        name: String,
        width: f64,
        height: f64,
        dark: bool,
        source: Option<Source>,
        projects: &Projects,
    ) -> Result<(), String> {
        let active = self.active.as_deref().ok_or("Open a Stages board")?;
        let uri = source
            .as_ref()
            .map(|source| source.uri.as_str())
            .unwrap_or(active);
        let configuration = PreviewStageConfiguration::new(
            name,
            uri,
            PreviewEnvironment {
                width,
                height,
                theme: if dark {
                    PreviewTheme::Dark
                } else {
                    PreviewTheme::Light
                },
                ..Default::default()
            },
        );
        configuration.validate()?;
        if self.board.stages().len() >= ferroui_designer::MAX_PREVIEW_STAGES {
            return Err("A preview board supports at most 16 stages".into());
        }
        if let Some(source) = source {
            let uri = source.uri.clone();
            let entry = StageSource::parse(source, self.sources.get(&uri), projects)?;
            let documents = self
                .sources
                .iter()
                .filter(|(key, _)| *key != &uri)
                .map(|(_, entry)| entry.preview.clone())
                .chain(std::iter::once(entry.preview.clone()))
                .collect();
            self.board
                .sync_sources(active, documents, PreviewHostOptions::default(), 0)?;
            self.sources.insert(uri, entry);
        }
        self.board.add(configuration)?;
        Ok(())
    }

    pub(crate) fn prototype_elements(
        &mut self,
        target: StageTarget,
    ) -> Result<Vec<PreviewPrototypeElementChoice>, String> {
        self.mounted(&target)?;
        self.board.prototype_elements(target.id)
    }

    pub(crate) fn image(&mut self, request: StageTarget) -> Result<Image, String> {
        let control = self.mounted(&request)?;
        let result = capture_preview_stage_png(&control, Default::default())?;
        Ok(Image {
            png: STANDARD.encode(result.png),
            width: result.pixel_width,
            height: result.pixel_height,
        })
    }

    pub(crate) fn checks(&mut self, target: StageTarget, projects: &Projects) -> Result<StageChecks, String> {
        self.mounted(&target)?;
        let stage = self.board.stage(target.id).ok_or("The stage was removed")?;
        let uri = stage.configuration.source_uri;
        let source = &self.sources.get(&uri).ok_or("Open the stage's source document first")?.authored;
        let document = DesignDocument::parse(source, projects.source(source)?)?;
        let findings = self.board.preview_checks(target.id)?.into_iter().map(|finding| StageFinding {
            range: document.node_name_range(finding.node).ok(), finding,
        }).collect();
        Ok(StageChecks { uri, findings })
    }

    pub(crate) fn state_options(&self, target: StageTarget) -> Result<Vec<PreviewStageStateOptions>, String> {
        self.stage_info(&target)?;
        self.board.state_options(target.id)
    }

    pub(crate) fn pointer(&mut self, request: Pointer, projects: &Projects) -> Result<Value, String> {
        if !(0.0..=1.0).contains(&request.x) || !(0.0..=1.0).contains(&request.y) {
            return Err("Choose a point inside the stage".into());
        }
        let control = self.mounted(&request.target)?;
        let id = request.target.id;
        let stage = self.board.stage(id).ok_or("The stage was removed")?;
        if !matches!(stage.status, PreviewStageStatus::Ready { .. }) {
            return Err("Refresh this stage before using it".into());
        }
        let size = control.bounds().size();
        let point = Point::new(size.width * request.x, size.height * request.y);
        if self.playback.interact(&self.board, id, point)? {
            return Ok(json!({"prototype":self.playback.scene()}));
        }
        if stage.configuration.mode == PreviewStageMode::Live {
            let inspector = Inspector::shared();
            let mut inspector = inspector.borrow_mut();
            let node_id = inspector.node_id(&control);
            inspector.execute(
                "click",
                &json!({"node_id":node_id,"x":point.x,"y":point.y}),
                Permissions {
                    allow_input: true,
                    ..Default::default()
                },
            )?;
            return Ok(json!({"interacted":true}));
        }
        let selection = self
            .board
            .resolve_selection(&self.board.hit_test_selection(id, point)?)?;
        let source = self
            .sources
            .get(&selection.source_uri)
            .ok_or("Open the stage's source document first")?;
        let source = &source.authored;
        let parsed = DesignDocument::parse(source, projects.source(source)?)?;
        let range = parsed.node_name_range(selection.node)?;
        Ok(json!({"range":range,"uri":source.uri}))
    }

    fn stage_info(&self, target: &StageTarget) -> Result<PreviewStageInfo, String> {
        self.board
            .stage(target.id)
            .filter(|info| info.epoch == target.epoch)
            .ok_or_else(|| "The stage changed. Use its current controls.".into())
    }

    pub(crate) fn mounted(&mut self, target: &StageTarget) -> Result<Ref<Control>, String> {
        if self.stage_info(target)?.rendered_revision.is_none() {
            return Err("This stage has no accepted preview".into());
        }
        let control = self
            .board
            .control(target.id)
            .ok_or("The stage was removed")?;
        self.windows
            .entry(target.id)
            .or_insert_with(|| StageWindow::new(control.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        control.update_layout();
        self.board.refresh_runtime_mapping(target.id)?;
        Ok(control)
    }

    pub(crate) fn model(&mut self) -> Model {
        let stages = self.board.stages();
        self.windows
            .retain(|id, _| stages.iter().any(|stage| stage.id == *id));
        Model {
            epoch: self.board.epoch(),
            manifest: self.board.manifest(),
            stages,
            prototype: self.playback.scene(),
        }
    }
}

struct StageWindow(Ref<Window>);

impl StageWindow {
    fn new(control: Ref<Control>) -> Self {
        let window = Window::new();
        window.set_window_decorations(WindowDecorations::None);
        window.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
        window.set_content(Some(Rc::new(control)));
        window.show();
        Self(window)
    }
}

impl Drop for StageWindow {
    fn drop(&mut self) {
        self.0.close();
    }
}
