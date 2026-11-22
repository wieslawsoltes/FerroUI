use crate::stages_prototype::Playback;
use crate::stages_sources::{self, StageDocument, StageSource};
use crate::{
    document::DesignDocument,
    projects::Projects,
    protocol::Image,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ferroui_base::diagnostics::DesignTextEnvironment;
use ferroui_base::{Point, Ref, threading::Dispatcher};
use ferroui_controls::{Control, SizeToContent, Window, WindowDecorations};
use ferroui_designer::{
    PreviewEnvironment, PreviewPrototypeElementChoice, PreviewPrototypeManifest,
    PreviewStageBoard, PreviewStageConfiguration, PreviewStageId, PreviewStageDetails, PreviewStageInfo, PreviewStageSet, PreviewStageSpec,
    PreviewStageFocus, PreviewStageKeyboardInput, PreviewStageLayout, PreviewStageMode, PreviewStageSourceMoves, PreviewStageStateOptions,
    PreviewStageStatus, PreviewTheme, capture_preview_stage_png,
};
use ferroui_designer_language::Range;
use ferroui_designer_project::PreviewSourceBuffer;
use ferroui_inspect::{Inspector, Permissions};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::{BTreeMap, BTreeSet}, rc::Rc};

#[derive(Default)]
pub(crate) struct Stages {
    pub(crate) board: PreviewStageBoard,
    windows: BTreeMap<PreviewStageId, StageWindow>,
    sources: BTreeMap<String, StageSource>,
    active: Option<String>,
    pub(crate) playback: Playback,
}

#[derive(Deserialize)]
pub(crate) struct Sync {
    pub(crate) source: StageDocument,
    pub(crate) manifest: Option<PreviewStageSet>,
    #[serde(default)]
    pub(crate) additional: Vec<StageDocument>,
    #[serde(default, rename = "localSources")]
    pub(crate) local_sources: Vec<PreviewSourceBuffer>,
    #[serde(default, rename = "sourceMoves")]
    pub(crate) source_moves: BTreeMap<String, String>,
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
    Add(Addition),
    Remove {
        id: PreviewStageId,
    },
    Update {
        id: PreviewStageId,
        configuration: PreviewStageSpec,
    },
    Refresh {
        id: PreviewStageId,
    },
    SetPrototype {
        graph: PreviewPrototypeManifest,
    },
}

#[derive(Deserialize)]
struct Addition {
    name: String,
    width: f64,
    height: f64,
    dark: bool,
    source: Option<StageDocument>,
    #[serde(default, rename = "localSources")]
    local_sources: Vec<PreviewSourceBuffer>,
}

#[derive(Serialize)]
pub(crate) struct Model {
    epoch: u64,
    manifest: PreviewStageSet,
    stages: Vec<PreviewStageDetails>,
    prototype: Option<crate::stages_prototype::Scene>,
    dependencies: BTreeSet<String>,
    #[serde(rename = "availableThemes")]
    available_themes: Vec<PreviewTheme>,
    #[serde(rename = "textEnvironment")]
    text_environment: bool,
}

#[derive(Deserialize)]
pub(crate) struct StageTarget {
    pub(crate) id: PreviewStageId,
    pub(crate) epoch: u64,
}

#[derive(Serialize)]
pub(crate) struct StageChecks {
    uri: String,
    findings: Vec<crate::preview_checks::SourceFinding>,
    accessibility: crate::preview_checks::SourceAccessibility,
}

#[derive(Serialize)]
pub(crate) struct StagePrototypeHotspot {
    #[serde(flatten)]
    element: PreviewPrototypeElementChoice,
    uri: String,
    range: Option<Range>,
}

#[derive(Deserialize)]
pub(crate) struct Pointer {
    #[serde(flatten)]
    target: StageTarget,
    x: f64,
    y: f64,
}

#[derive(Deserialize)]
pub(crate) struct Keyboard {
    #[serde(flatten)]
    target: StageTarget,
    input: PreviewStageKeyboardInput,
}

impl Stages {
    pub(crate) fn sync(&mut self, request: Sync, projects: &Projects) -> Result<Model, String> {
        let uri = request.source.authored.uri.clone();
        if request.additional.len() >= ferroui_designer::MAX_PREVIEW_STAGES {
            return Err("Open at most 16 preview documents".into());
        }
        if let Some(manifest) = &request.manifest {
            manifest.validate()?;
        }
        let moves = PreviewStageSourceMoves::new(request.source_moves)?;
        let mut sources = BTreeMap::new();
        for source in std::iter::once(request.source).chain(request.additional) {
            let key = source.authored.uri.clone();
            if sources.contains_key(&key) {
                return Err("Duplicate preview source URI".into());
            }
            sources.insert(
                key.clone(),
                StageSource::parse(source, self.sources.get(&key), projects, &request.local_sources)?,
            );
        }
        let epoch = self.board.epoch();
        stages_sources::sync_board(&mut self.board, &uri,
            sources.iter().map(|(uri, source)| (uri.as_str(), source)), &moves)?;
        if let Some(manifest) = request.manifest {
            self.board.restore_stage_set(manifest)?;
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
            Action::Add(addition) => self.add(addition, projects)?,
            Action::Remove { id } => self.board.remove(id)?,
            Action::Update { id, configuration } => {
                let previous = self.board.stage(id).ok_or("The stage was removed")?;
                if configuration.configuration.source_uri != previous.configuration.source_uri {
                    return Err("The stage source changed. Reopen its document in Stages.".into());
                }
                self.board.update_spec(id, configuration)?;
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
        addition: Addition,
        projects: &Projects,
    ) -> Result<(), String> {
        let Addition { name, width, height, dark, source, local_sources } = addition;
        let active = self.active.as_deref().ok_or("Open a Stages board")?;
        let uri = source
            .as_ref()
            .map(|source| source.authored.uri.as_str())
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
            let uri = source.authored.uri.clone();
            let entry = StageSource::parse(source, self.sources.get(&uri), projects, &local_sources)?;
            let documents = self
                .sources
                .iter()
                .filter(|(key, _)| *key != &uri)
                .map(|(uri, entry)| (uri.as_str(), entry))
                .chain(std::iter::once((uri.as_str(), &entry)));
            stages_sources::sync_board(&mut self.board, active, documents, &PreviewStageSourceMoves::default())?;
            self.sources.insert(uri, entry);
        }
        self.board.add(configuration)?;
        Ok(())
    }

    pub(crate) fn prototype_elements(
        &mut self,
        target: StageTarget,
        projects: &Projects,
    ) -> Result<Vec<StagePrototypeHotspot>, String> {
        self.mounted(&target)?;
        let stage = self.board.stage(target.id).ok_or("The stage was removed")?;
        let uri = stage.configuration.source_uri;
        let source = &self.sources.get(&uri).ok_or("Open the stage's source document first")?.authored;
        let document = DesignDocument::parse(source, projects.source(source)?)?;
        Ok(self.board.prototype_elements(target.id)?.into_iter().map(|element| {
            let range = element.anchor.resolve(&document.xaml)
                .and_then(|node| document.node_name_range(node)).ok();
            StagePrototypeHotspot { element, uri: uri.clone(), range }
        }).collect())
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
        let findings = self.board.preview_checks(target.id)?;
        let accessibility = self.board.preview_accessibility(target.id)?;
        let map = document.rsx.as_ref().map(|rsx| rsx.mapped_markup().1);
        let findings = crate::preview_checks::source_findings(findings, &document.xaml, document.source, map.as_ref());
        let accessibility = crate::preview_checks::source_accessibility(accessibility, &document.xaml, document.source, map.as_ref());
        Ok(StageChecks { uri, findings, accessibility })
    }

    pub(crate) fn state_options(&self, target: StageTarget) -> Result<Vec<PreviewStageStateOptions>, String> {
        self.stage_info(&target)?;
        self.board.state_options(target.id)
    }

    pub(crate) fn fixture_options(&self, target: StageTarget) -> Result<Vec<String>, String> {
        self.stage_info(&target)?;
        self.board.fixture_options(target.id)
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
        let text = stage.configuration.text.environment()?;
        if DesignTextEnvironment::run(text.as_ref(), || self.playback.interact(&self.board, id, point))? {
            return Ok(json!({"prototype":self.playback.scene()}));
        }
        if stage.configuration.mode == PreviewStageMode::Live {
            let inspector = Inspector::shared();
            let mut inspector = inspector.borrow_mut();
            let node_id = inspector.node_id(&control);
            DesignTextEnvironment::run(text.as_ref(), || inspector.execute(
                "click",
                &json!({"node_id":node_id,"x":point.x,"y":point.y}),
                Permissions {
                    allow_input: true,
                    ..Default::default()
                },
            ))?;
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

    pub(crate) fn keyboard(&mut self, request: Keyboard) -> Result<Option<PreviewStageFocus>, String> {
        if self.playback.scene().is_some() {
            return Err("Close prototype playback before using stage keyboard input".into());
        }
        let target = self.board.keyboard_target(request.target.id, request.target.epoch)?;
        self.mounted(&request.target)?;
        target.input(request.input)
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
        let control = self.mount_control(target.id).ok_or("The stage was removed")?;
        Dispatcher::ui_thread().run_jobs(None);
        control.update_layout();
        self.board.refresh_runtime_mapping(target.id)?;
        Ok(control)
    }

    fn mount_control(&mut self, id: PreviewStageId) -> Option<Ref<Control>> {
        let control = self.board.control(id)?;
        self.windows.entry(id).or_insert_with(|| StageWindow::new(control.clone()));
        Some(control)
    }

    pub(crate) fn model(&mut self) -> Model {
        self.windows.retain(|id, _| self.board.control(*id).is_some());
        for stage in self.board.stages() {
            if stage.rendered_revision.is_some() && stage.configuration.layout != PreviewStageLayout::Fixed {
                let _ = self.mount_control(stage.id);
            }
        }
        if !self.windows.is_empty() {
            // A fitted window may finish its resize on the dispatcher after a
            // stage update. Report the same settled layout used for its image.
            Dispatcher::ui_thread().run_jobs(None);
            for id in self.windows.keys() {
                if let Some(control) = self.board.control(*id) {
                    control.update_layout();
                }
            }
        }
        let stages = self.board.stage_details();
        let mut dependencies = stages_sources::watch_paths(&self.sources);
        dependencies.extend(self.board.asset_dependencies());
        Model {
            epoch: self.board.epoch(),
            manifest: self.board.stage_set(),
            stages,
            prototype: self.playback.scene(),
            dependencies,
            available_themes: PreviewTheme::available(),
            text_environment: true,
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
