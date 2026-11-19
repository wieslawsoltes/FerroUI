//! Retained native comparison stages. The board owns presentation state and native
//! graphs; documents remain snapshots of the host's sole authored transaction model.
use crate::runtime::RuntimePreview;
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewFlowDirection, PreviewHostOptions,
    PreviewPrototype, PreviewPrototypeManifest, PreviewStatus, PreviewTheme, RuntimeNode,
};
use ferroui_base::media::{FlowDirection, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::UriRef;
use ferroui_base::{Point, Ref, Visual, WeakRef};
use ferroui_controls::{Border, ContentControl, Control, ThemeVariantScope};
use ferroui_designer_core::{Ast, Document, NodeId};
use crate::PreviewSourceAnchor;
use ferroui_designer_project::LocalPreviewSources;
use ferroui_inspect::{Inspector, TreeKind};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[path = "preview_stage_includes.rs"]
mod includes;
#[path = "preview_stage_layout.rs"]
mod layout;
#[path = "preview_stage_manifest.rs"]
mod manifest;
pub use manifest::{PreviewStageDetails, PreviewStageManifest, PreviewStageSet, PreviewStageSnapshot, PreviewStageSpec};
#[path = "preview_stage_prototype_elements.rs"]
mod prototype_elements;
use layout::StageLayoutHost;
use includes::{IncludeCatalog, IncludeCatalogs, IncludeSource};

pub type PreviewStageId = u64;
pub const MAX_PREVIEW_STAGES: usize = 16;
const MAX_MANIFEST_BYTES: usize = 1_048_576;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStageMode {
    #[default]
    Select,
    Live,
}

/// Native measurement policy for a stage. Environment dimensions are the finite
/// layout proposal and maximum size; fitting axes use the content's desired size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStageLayout {
    #[default]
    Fixed,
    SizeThatFits,
    /// Fixed width with a content-fitting height.
    FixedWidth,
    /// Content-fitting width with a fixed height.
    FixedHeight,
}

impl PreviewStageLayout {
    pub(crate) fn fits_width(self) -> bool {
        matches!(self, Self::SizeThatFits | Self::FixedHeight)
    }

    pub(crate) fn fits_height(self) -> bool {
        matches!(self, Self::SizeThatFits | Self::FixedWidth)
    }
}

/// Resolved native logical dimensions, independent from a card's visual scaling.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewStageTarget {
    #[default]
    Document,
    Definition {
        anchor: PreviewSourceAnchor,
        options: DefinitionPreviewOptions,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageState {
    pub owner: PreviewSourceAnchor,
    pub group: String,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageConfiguration {
    pub name: String,
    pub source_uri: String,
    pub target: PreviewStageTarget,
    pub environment: PreviewEnvironment,
    #[serde(default)]
    pub layout: PreviewStageLayout,
    pub design_time_data: bool,
    pub states: Vec<PreviewStageState>,
    pub pinned: bool,
    pub auto_update: bool,
    pub mode: PreviewStageMode,
}

impl PreviewStageConfiguration {
    pub fn new(
        name: impl Into<String>,
        source_uri: impl Into<String>,
        environment: PreviewEnvironment,
    ) -> Self {
        Self {
            name: name.into(),
            source_uri: source_uri.into(),
            target: PreviewStageTarget::Document,
            environment,
            layout: PreviewStageLayout::Fixed,
            design_time_data: true,
            states: Vec::new(),
            pinned: false,
            auto_update: true,
            mode: PreviewStageMode::Select,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        bounded(&self.name, 128, "Stage name", false)?;
        validate_uri(&self.source_uri)?;
        self.environment.validate()?;
        if let PreviewStageTarget::Definition { anchor, options } = &self.target {
            anchor.validate()?;
            options.validate()?;
            if let Some(target_type) = &options.target_type {
                bounded(target_type, 512, "Definition target type", false)?;
            }
        }
        if self.states.len() > 32 {
            return Err("A preview stage supports at most 32 visual-state groups".into());
        }
        let mut groups = BTreeSet::new();
        for state in &self.states {
            state.owner.validate()?;
            bounded(&state.group, 256, "State group", false)?;
            bounded(&state.state, 256, "Visual state", false)?;
            let identity = (
                serde_json::to_string(&state.owner).map_err(|error| error.to_string())?,
                state.group.clone(),
            );
            if !groups.insert(identity) {
                return Err("A stage can select only one state in each owner's state group".into());
            }
        }
        if self.text_size() > 262_144 {
            return Err("Preview-stage configuration text exceeds 256 KiB".into());
        }
        Ok(())
    }

    fn text_size(&self) -> usize {
        self.name.len()
            + self.source_uri.len()
            + 256
            + match &self.target {
                PreviewStageTarget::Document => 0,
                PreviewStageTarget::Definition { anchor, options } => {
                    anchor.text_size()
                        + options.sample_content.len()
                        + options.target_type.as_ref().map_or(0, String::len)
                }
            }
            + self
                .states
                .iter()
                .map(|state| state.owner.text_size() + state.group.len() + state.state.len() + 64)
                .sum::<usize>()
    }

    fn same_rendering(&self, other: &Self) -> bool {
        self.source_uri == other.source_uri
            && self.target == other.target
            && self.environment.theme == other.environment.theme
            && self.environment.flow_direction == other.environment.flow_direction
            && self.design_time_data == other.design_time_data
            && self.states == other.states
    }

    fn same_layout(&self, other: &Self) -> bool {
        self.layout == other.layout
            && self.environment.width == other.environment.width
            && self.environment.height == other.environment.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewStageStatus {
    Ready {
        revision: u64,
    },
    Paused {
        revision: Option<u64>,
    },
    PendingSource {
        revision: u64,
    },
    InvalidSource {
        revision: u64,
    },
    MissingDocument,
    Failed {
        revision: Option<u64>,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PreviewStageInfo<Configuration = PreviewStageConfiguration> {
    pub id: PreviewStageId,
    pub configuration: Configuration,
    pub epoch: u64,
    pub status: PreviewStageStatus,
    pub layout_size: PreviewStageSize,
    pub rendered_revision: Option<u64>,
    pub source_revision: Option<u64>,
    pub notices: Vec<String>,
}

/// A host-supplied authored buffer. `pending` means text has been staged but its
/// accepted runtime update has not yet been requested; the board never applies it.
#[derive(Clone)]
pub struct PreviewStageSource {
    pub uri: String,
    pub document: Document,
    pub pending: bool,
}

impl PreviewStageSource {
    pub(crate) fn fixture_options(&self) -> Result<Vec<String>, String> {
        if self.pending || !self.document.is_valid() {
            return Err("Repair the stage source before choosing design fixtures".into());
        }
        Ok(ferroui_designer_core::design_fixtures::DesignFixtures::from_document(&self.document)?
            .into_iter().flat_map(|set| set.fixtures).map(|fixture| fixture.name).collect())
    }
}

/// An accepted native/source identity. Its private weak reference and retained AST
/// prevent callers from manufacturing a source selection using a stale visual ID.
#[derive(Clone)]
pub struct PreviewStageSelectionToken {
    stage: PreviewStageId,
    epoch: u64,
    inspector_id: u64,
    visual: WeakRef<Visual>,
    node: NodeId,
    ast: Arc<Ast>,
    source: String,
}

#[derive(Clone, Debug)]
pub struct PreviewStageSelection {
    pub stage: PreviewStageId,
    pub source_uri: String,
    pub node: NodeId,
    pub anchor: Option<PreviewSourceAnchor>,
    pub configuration: PreviewStageConfiguration,
    pub revision: u64,
    ast: Arc<Ast>,
    source: String,
}

impl PreviewStageSelection {
    /// Tab activation may rebase revisions, but must retain the exact authored AST
    /// and source accepted by the stage before a selection reaches authoring tools.
    pub fn matches_source(&self, uri: &str, document: &Document) -> bool {
        self.source_uri == uri
            && document.is_valid()
            && self.source == document.source()
            && Arc::ptr_eq(&self.ast, &document.ast_snapshot())
            && document.ast().node(self.node).is_some()
            && self
                .anchor
                .as_ref()
                .is_none_or(|anchor| anchor.resolve(document).ok() == Some(self.node))
    }
}

struct RenderedStage {
    // Drop the scenario's clock/classes while the native graph is still available.
    _scenario: Option<crate::preview_stage_scenarios::StageScenarioGuard>,
    background_subscription: Rc<dyn ferroui_base::reactive::IDisposable>,
    backing: Ref<Border>,
    preview: RuntimePreview,
    scope: Ref<ThemeVariantScope>,
    document: Document,
    spec: PreviewStageSpec,
    options_epoch: u64,
    source_snapshot: SourceSnapshot,
    source_catalog: Rc<IncludeCatalog>,
}

impl Drop for RenderedStage {
    fn drop(&mut self) {
        self.background_subscription.dispose();
    }
}

struct Stage {
    id: PreviewStageId,
    spec: PreviewStageSpec,
    control: Ref<StageLayoutHost>,
    // Isolated player presentation only: never serialized or part of the accepted
    // source/configuration epoch. Shared with retained and candidate theme handlers.
    prototype_overlay: Rc<Cell<bool>>,
    epoch: u64,
    status: PreviewStageStatus,
    rendered: Option<RenderedStage>,
    observed: Option<SourceStamp>,
    observed_dependencies: BTreeMap<String, Option<IncludeSource>>,
}

#[derive(Clone)]
struct SourceStamp {
    revision: u64,
    ast: Arc<Ast>,
    source: String,
    pending: bool,
    valid: bool,
    options_epoch: u64,
}

impl SourceStamp {
    fn new(source: &PreviewStageSource, options_epoch: u64) -> Self {
        Self {
            revision: source.document.revision(),
            ast: source.document.ast_snapshot(),
            source: source.document.source().into(),
            pending: source.pending,
            valid: source.document.is_valid(),
            options_epoch,
        }
    }
    fn matches_source(&self, source: &PreviewStageSource, options_epoch: u64) -> bool {
        self.revision == source.document.revision()
            && self.pending == source.pending
            && self.valid == source.document.is_valid()
            && self.options_epoch == options_epoch
            && self.source == source.document.source()
            && Arc::ptr_eq(&self.ast, &source.document.ast_snapshot())
    }

    fn same_buffer(&self, other: &Self) -> bool {
        self.pending == other.pending
            && self.valid == other.valid
            && self.source == other.source
            && Arc::ptr_eq(&self.ast, &other.ast)
    }

    fn matches_buffer(&self, source: &PreviewStageSource) -> bool {
        self.pending == source.pending
            && self.valid == source.document.is_valid()
            && self.source == source.document.source()
            && Arc::ptr_eq(&self.ast, &source.document.ast_snapshot())
    }
}

/// One UI-thread collection of native comparison stages across the host's open
/// document buffers. It never owns document history or performs authored mutations.
pub struct PreviewStageBoard {
    stages: Vec<Stage>,
    prototype: PreviewPrototype,
    active: Option<PreviewStageId>,
    active_uri: String,
    sources: BTreeMap<String, PreviewStageSource>,
    includes: IncludeCatalogs,
    options: PreviewHostOptions,
    options_epoch: u64,
    epoch: u64,
}

impl Default for PreviewStageBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewStageBoard {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            prototype: PreviewPrototype::default(),
            active: None,
            active_uri: String::new(),
            sources: BTreeMap::new(),
            includes: IncludeCatalogs::default(),
            options: PreviewHostOptions::default(),
            options_epoch: 0,
            epoch: 1,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn active(&self) -> Option<PreviewStageId> {
        self.active
    }

    pub fn prototype(&self) -> PreviewPrototype {
        self.prototype.clone()
    }

    /// Replace local interaction descriptors without rebuilding native stages or
    /// mutating authored source. Rejected graphs leave the accepted board intact.
    pub fn set_prototype(&mut self, prototype: PreviewPrototype) -> Result<(), String> {
        prototype.validate(&self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        if self.prototype != prototype {
            self.prototype = prototype;
            self.bump();
        }
        Ok(())
    }

    pub fn stages(&self) -> Vec<PreviewStageInfo> {
        self.stages.iter().map(|stage| self.info(stage, stage.spec.configuration.clone())).collect()
    }

    pub fn stage(&self, id: PreviewStageId) -> Option<PreviewStageInfo> {
        self.stages
            .iter()
            .find(|stage| stage.id == id)
            .map(|stage| self.info(stage, stage.spec.configuration.clone()))
    }

    pub fn control(&self, id: PreviewStageId) -> Option<Ref<Control>> {
        self.stages
            .iter()
            .find(|stage| stage.id == id)
            .map(|stage| stage.control.clone().upcast())
    }

    /// Let an overlay's native content supply its own shape and translucency.
    /// Switching between an overlay and a base screen keeps the native graph,
    /// interaction epochs and live input intact, including a retained failed frame.
    pub(crate) fn set_prototype_overlay_backing(
        &mut self,
        id: PreviewStageId,
        overlay: bool,
    ) -> Result<(), String> {
        let stage = &self.stages[self.index(id)?];
        if stage.prototype_overlay.replace(overlay) != overlay {
            if let Some(rendered) = &stage.rendered {
                apply_stage_backing(&rendered.scope, &rendered.backing, overlay);
            }
        }
        Ok(())
    }

    pub fn stage_spec(&self, id: PreviewStageId) -> Option<PreviewStageSpec> {
        self.stages.iter().find(|stage| stage.id == id).map(|stage| stage.spec.clone())
    }

    pub fn stage_details(&self) -> Vec<PreviewStageDetails> {
        self.stages.iter().map(|stage| self.info(stage, stage.spec.clone())).collect()
    }

    fn info<C>(&self, stage: &Stage, configuration: C) -> PreviewStageInfo<C> {
        PreviewStageInfo {
            id: stage.id,
            configuration,
            epoch: stage.epoch,
            status: stage.status.clone(),
            layout_size: stage.control.layout_size(),
            rendered_revision: stage
                .rendered
                .as_ref()
                .map(|rendered| rendered.document.revision()),
            source_revision: self
                .sources
                .get(&stage.spec.configuration.source_uri)
                .map(|source| source.document.revision()),
            notices: stage
                .rendered
                .as_ref()
                .map_or_else(Vec::new, |rendered| rendered.preview.notices.clone()),
        }
    }

    pub fn sync_sources(
        &mut self,
        active_uri: &str,
        sources: Vec<PreviewStageSource>,
        options: PreviewHostOptions,
        options_epoch: u64,
    ) -> Result<(), String> {
        self.sync_sources_with_local_includes(active_uri, sources, options, options_epoch, &BTreeMap::new())
    }

    /// Synchronize authored buffers and the host's scoped literal includes for each view.
    /// Native include requests determine which cards need a replacement frame.
    pub fn sync_sources_with_local_includes(
        &mut self,
        active_uri: &str,
        sources: Vec<PreviewStageSource>,
        options: PreviewHostOptions,
        options_epoch: u64,
        local_includes: &BTreeMap<&str, &LocalPreviewSources>,
    ) -> Result<(), String> {
        validate_uri(active_uri)?;
        let mut next = BTreeMap::new();
        for mut source in sources {
            validate_uri(&source.uri)?;
            source.document = source.document.snapshot_without_history();
            if next.insert(source.uri.clone(), source).is_some() {
                return Err("Duplicate preview source URI".into());
            }
        }
        let includes = IncludeCatalogs::new(&next, local_includes, options_epoch)?;
        let active_changed = self.active_uri != active_uri;
        let sources_changed = self.sources.len() != next.len()
            || next.iter().any(|(uri, source)| {
                self.sources.get(uri).is_none_or(|old| {
                    old.pending != source.pending
                        || old.document.revision() != source.document.revision()
                        || old.document.is_valid() != source.document.is_valid()
                        || old.document.source() != source.document.source()
                        || !Arc::ptr_eq(
                            &old.document.ast_snapshot(),
                            &source.document.ast_snapshot(),
                        )
                })
            });
        let options_changed = self.options_epoch != options_epoch;
        self.sources = next;
        self.includes = includes;
        self.options = options;
        self.options_epoch = options_epoch;
        self.active_uri = active_uri.into();
        if active_changed || sources_changed || options_changed {
            self.bump();
        }
        let ids = self
            .stages
            .iter()
            .filter_map(|stage| {
                let next = self.sources.get(&stage.spec.configuration.source_uri);
                let changed = match (&stage.observed, &next) {
                    (Some(old), Some(next)) => !old.matches_source(next, options_epoch),
                    (None, None) => !matches!(stage.status, PreviewStageStatus::MissingDocument),
                    _ => true,
                };
                let catalog = self.includes.for_document(&stage.spec.configuration.source_uri);
                (changed || observed_dependencies_changed(stage, catalog)).then_some(stage.id)
            })
            .collect::<Vec<_>>();
        for id in ids {
            let _ = self.reload(id, false);
        }
        Ok(())
    }

    pub fn add(
        &mut self,
        configuration: PreviewStageConfiguration,
    ) -> Result<PreviewStageId, String> {
        self.add_spec(configuration.into())
    }

    pub fn add_spec(&mut self, spec: PreviewStageSpec) -> Result<PreviewStageId, String> {
        let configuration = &spec.configuration;
        spec.validate()?;
        if self.stages.len() >= MAX_PREVIEW_STAGES {
            return Err(format!(
                "A preview board supports at most {MAX_PREVIEW_STAGES} stages"
            ));
        }
        let id = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let control = StageLayoutHost::new(configuration.layout, configuration.environment);
        self.stages.push(Stage {
            id,
            spec,
            control,
            prototype_overlay: Rc::new(Cell::new(false)),
            epoch: 1,
            status: PreviewStageStatus::MissingDocument,
            rendered: None,
            observed: None,
            observed_dependencies: BTreeMap::new(),
        });
        self.active = Some(id);
        self.bump();
        // Construction failures are represented by a retained error card. Callers can
        // still edit, refresh or remove that stage without changing authored source.
        let _ = self.reload(id, true);
        Ok(id)
    }

    pub fn update(
        &mut self,
        id: PreviewStageId,
        configuration: PreviewStageConfiguration,
    ) -> Result<(), String> {
        let index = self.index(id)?;
        let prior = &self.stages[index].spec;
        let design_fixture = if prior.configuration.source_uri == configuration.source_uri {
            prior.design_fixture.clone()
        } else {
            None
        };
        self.update_spec(id, PreviewStageSpec { configuration, design_fixture })
    }

    pub fn update_spec(&mut self, id: PreviewStageId, spec: PreviewStageSpec) -> Result<(), String> {
        spec.validate()?;
        let index = self.index(id)?;
        let previous = &self.stages[index].spec;
        if *previous == spec {
            return Ok(());
        }
        let rendering_changed = !previous.same_rendering(&spec);
        let layout_changed = !previous.configuration.same_layout(&spec.configuration);
        let was_updating = previous.configuration.auto_update;
        self.stages[index].spec = spec;
        self.touch(index);
        if rendering_changed {
            let _ = self.reload(id, true);
        } else {
            if layout_changed && self.apply_layout(index).is_err() {
                self.apply_mode(index);
                return Ok(());
            }
            if !self.stages[index].spec.configuration.auto_update {
                self.stages[index].status = PreviewStageStatus::Paused {
                    revision: self.stages[index]
                        .rendered
                        .as_ref()
                        .map(|rendered| rendered.document.revision()),
                };
                self.apply_mode(index);
            } else if !was_updating {
                let _ = self.reload(id, false);
            } else {
                self.apply_mode(index);
            }
        }
        Ok(())
    }

    pub fn set_active(&mut self, id: PreviewStageId) -> Result<(), String> {
        self.index(id)?;
        if self.active != Some(id) {
            self.active = Some(id);
            self.bump();
        }
        Ok(())
    }

    pub fn duplicate(&mut self, id: PreviewStageId) -> Result<PreviewStageId, String> {
        let mut spec = self.stages[self.index(id)?].spec.clone();
        spec.configuration.name = format!(
            "{} copy",
            spec.configuration.name.chars().take(120).collect::<String>()
        );
        self.add_spec(spec)
    }

    pub fn remove(&mut self, id: PreviewStageId) -> Result<(), String> {
        let index = self.index(id)?;
        let stage = self.stages.remove(index);
        self.prototype.remove_stage(id);
        stage.control.children().clear();
        if self.active == Some(id) {
            self.active = self
                .stages
                .get(index.min(self.stages.len().saturating_sub(1)))
                .map(|stage| stage.id);
        }
        drop(stage);
        self.bump();
        Ok(())
    }

    pub fn move_stage(&mut self, id: PreviewStageId, index: usize) -> Result<(), String> {
        let previous = self.index(id)?;
        if index >= self.stages.len() {
            return Err("Stage order index is outside the board".into());
        }
        if previous != index {
            let stage = self.stages.remove(previous);
            self.stages.insert(index, stage);
            self.bump();
        }
        Ok(())
    }

    pub fn set_paused(&mut self, id: PreviewStageId, paused: bool) -> Result<(), String> {
        let mut config = self.stages[self.index(id)?].spec.configuration.clone();
        config.auto_update = !paused;
        self.update(id, config)
    }

    /// Explicit refresh also works while automatic updates are paused.
    pub fn refresh(&mut self, id: PreviewStageId) -> Result<(), String> {
        self.reload(id, true)
    }

    pub fn refresh_all(&mut self) -> Vec<(PreviewStageId, String)> {
        let ids = self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>();
        ids.into_iter()
            .filter_map(|id| self.refresh(id).err().map(|error| (id, error)))
            .collect()
    }

    /// Create variants atomically with respect to descriptor validation and capacity.
    /// Each native failure remains isolated to its resulting card.
    pub fn compare_variants(
        &mut self,
        id: PreviewStageId,
        variants: Vec<(String, PreviewEnvironment)>,
    ) -> Result<Vec<PreviewStageId>, String> {
        let source = self.stages[self.index(id)?].spec.clone();
        if variants.is_empty() || self.stages.len() + variants.len() > MAX_PREVIEW_STAGES {
            return Err("Choose variants within the board's 16-stage capacity".into());
        }
        let mut configs = Vec::new();
        for (name, environment) in variants {
            let mut configuration = source.clone();
            configuration.configuration.name = name;
            configuration.configuration.environment = environment;
            configuration.validate()?;
            configs.push(configuration);
        }
        self.add_specs(configs)
    }

    /// Add a bounded comparison batch. Descriptor validation and capacity are
    /// checked for the entire batch before any native factory can be called.
    /// Individual native loading failures remain visible on their own cards.
    pub fn add_configurations(
        &mut self,
        configurations: Vec<PreviewStageConfiguration>,
    ) -> Result<Vec<PreviewStageId>, String> {
        self.add_specs(configurations.into_iter().map(PreviewStageSpec::from).collect())
    }

    pub fn add_specs(&mut self, configurations: Vec<PreviewStageSpec>) -> Result<Vec<PreviewStageId>, String> {
        if configurations.is_empty()
            || configurations.len() > MAX_PREVIEW_STAGES.saturating_sub(self.stages.len())
        {
            return Err("Choose configurations within the board's 16-stage capacity".into());
        }
        for configuration in &configurations {
            configuration.validate()?;
        }
        configurations
            .into_iter()
            .map(|spec| self.add_spec(spec))
            .collect()
    }

    pub fn manifest(&self) -> PreviewStageManifest {
        self.stage_set().into_manifest()
    }

    pub fn stage_set(&self) -> PreviewStageSet {
        PreviewStageSet {
            version: if self.prototype.is_empty() {
                1
            } else if self.prototype.has_overlay_actions() {
                3
            } else {
                2
            },
            stages: self
                .stages
                .iter()
                .map(|stage| stage.spec.clone())
                .collect(),
            active: self
                .active
                .and_then(|id| self.stages.iter().position(|stage| stage.id == id)),
            prototype: PreviewPrototypeManifest::from_prototype(
                &self.prototype,
                &self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>(),
            )
            .expect("Accepted prototype graph references existing preview stages"),
        }
    }

    pub fn snapshot(&self) -> PreviewStageSnapshot {
        self.manifest()
    }

    /// Imported IDs are freshly allocated so callbacks from a retired board cannot
    /// address a new card. Validation finishes before any old stage is removed.
    pub fn restore_manifest(&mut self, manifest: PreviewStageManifest) -> Result<(), String> {
        self.restore_stage_set(manifest.into())
    }

    pub fn restore_stage_set(&mut self, manifest: PreviewStageSet) -> Result<(), String> {
        manifest.validate()?;
        let mut candidate = Self::new();
        candidate.sources = self.sources.clone();
        candidate.includes = self.includes.clone();
        candidate.options = self.options.clone();
        candidate.options_epoch = self.options_epoch;
        candidate.active_uri = self.active_uri.clone();
        for spec in manifest.stages {
            candidate.add_spec(spec)?;
        }
        candidate.prototype = manifest.prototype.restore(
            &candidate
                .stages
                .iter()
                .map(|stage| stage.id)
                .collect::<Vec<_>>(),
        )?;
        candidate.active = manifest
            .active
            .and_then(|index| candidate.stages.get(index).map(|stage| stage.id));
        candidate.epoch = self
            .epoch
            .checked_add(1)
            .expect("Preview board epoch exhausted");
        *self = candidate;
        Ok(())
    }

    pub fn restore(&mut self, snapshot: PreviewStageSnapshot) -> Result<(), String> {
        self.restore_manifest(snapshot)
    }

    /// Call after a successful Save As, before synchronizing the new URI snapshots.
    /// The old graph stays visible but immediately loses source-selection authority.
    pub fn retarget_source(&mut self, old_uri: &str, new_uri: &str) -> Result<(), String> {
        validate_uri(new_uri)?;
        if old_uri == new_uri {
            return Ok(());
        }
        let mut changed = Vec::new();
        for (index, stage) in self.stages.iter_mut().enumerate() {
            if stage.spec.configuration.source_uri == old_uri {
                stage.spec.configuration.source_uri = new_uri.into();
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
        Ok(())
    }

    pub fn runtime_tree(
        &self,
        id: PreviewStageId,
        kind: TreeKind,
    ) -> Result<Vec<RuntimeNode>, String> {
        let stage = &self.stages[self.index(id)?];
        Ok(stage.rendered.as_ref().map_or_else(Vec::new, |rendered| {
            let mut nodes = rendered.preview.tree(kind);
            if self.current(stage).is_err() {
                for node in &mut nodes {
                    node.source_id = None;
                }
            } else if matches!(stage.spec.configuration.target, PreviewStageTarget::Document) {
                for node in &mut nodes {
                    if Inspector::shared()
                        .borrow()
                        .resolve(node.inspector_id)
                        .ok()
                        .and_then(|object| object.cast::<Visual>())
                        .is_some_and(|visual| visual.templated_parent().is_some())
                    {
                        node.source_id = None;
                    }
                }
            }
            nodes
        }))
    }

    /// Inspect only the current native frame for this stage's source and environment.
    pub fn preview_checks(&self, id: PreviewStageId) -> Result<Vec<crate::PreviewFinding>, String> {
        let stage = &self.stages[self.index(id)?];
        let rendered = self.current(stage)?;
        crate::preview_checks::check_preview(&rendered.document, &rendered.preview, &stage.control.clone().upcast())
    }

    /// Read current authored state groups, including when a stage needs repair.
    pub fn state_options(
        &self,
        id: PreviewStageId,
    ) -> Result<Vec<crate::PreviewStageStateOptions>, String> {
        let stage = &self.stages[self.index(id)?];
        let source = self
            .sources
            .get(&stage.spec.configuration.source_uri)
            .ok_or("Open the stage source before choosing states")?;
        crate::preview_stage_state_options::source_options(source, &stage.spec.configuration.target)
    }

    pub fn fixture_options(&self, id: PreviewStageId) -> Result<Vec<String>, String> {
        let stage = &self.stages[self.index(id)?];
        let source = self.sources.get(&stage.spec.configuration.source_uri)
            .ok_or("Open the stage source before choosing design fixtures")?;
        source.fixture_options()
    }

    /// Resolve one authored hotspot against the exact accepted native graph. The
    /// returned axis-aligned rectangle uses stage logical coordinates and is clipped
    /// to the stage viewport and native ancestor clips. Call again immediately before
    /// activation: native scrolling and layout can move a target without source edits.
    pub fn prototype_hotspot_bounds(
        &self,
        id: PreviewStageId,
        anchor: &PreviewSourceAnchor,
    ) -> Result<ferroui_base::Rect, String> {
        // Boundary (#567): a guard while the native graph of the previewed document is read;
        // every expected failure is an `Err` of the resolver. Isolating a panic needs a build
        // that unwinds (`dev`, `release` or `dist-unwind` profile).
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.resolve_prototype_hotspot_bounds(id, anchor)
        }))
        .unwrap_or_else(|_| {
            Err("The prototype hotspot's native geometry could not be inspected".into())
        })
    }

    /// Exact source identity for native keyboard focus routing. This does not
    /// infer a target from overlapping rectangles or create native controls.
    pub(crate) fn prototype_hotspot_visual(
        &self,
        id: PreviewStageId,
        anchor: &PreviewSourceAnchor,
    ) -> Result<Ref<Visual>, String> {
        let stage = &self.stages[self.index(id)?];
        let rendered = self.current(stage)?;
        let node = anchor.resolve(&rendered.document)?;
        let visual = rendered
            .preview
            .unique_scenario_visual(node)
            .map_err(|_| "The prototype hotspot has multiple native instances; open a definition stage to target one instance".to_owned())?
            .ok_or("The prototype hotspot has no realized native visual")?;
        if matches!(stage.spec.configuration.target, PreviewStageTarget::Document)
            && visual.templated_parent().is_some()
        {
            return Err("Open a definition stage to target an instantiated template part".into());
        }
        Ok(visual)
    }

    fn resolve_prototype_hotspot_bounds(
        &self,
        id: PreviewStageId,
        anchor: &PreviewSourceAnchor,
    ) -> Result<ferroui_base::Rect, String> {
        use ferroui_base::input::InputElement;
        use ferroui_base::layout::Layoutable;
        use ferroui_base::Rect;

        let visual = self.prototype_hotspot_visual(id, anchor)?;
        let stage = &self.stages[self.index(id)?];
        let host = stage.control.clone().upcast::<Visual>();
        if !host.is_attached_to_visual_tree() {
            return Err(
                "Attach the prototype stage to a native view before resolving its hotspots".into(),
            );
        }
        let to_host = visual
            .transform_to_visual(&host)
            .ok_or("The prototype hotspot is detached from its stage")?;
        let mut bounds = Rect::from_size(visual.bounds().size()).transform_to_aabb(to_host);
        let valid_rect = |rect: Rect| {
            [
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                rect.right(),
                rect.bottom(),
            ]
            .into_iter()
            .all(f64::is_finite)
                && rect.width > 0.0
                && rect.height > 0.0
        };
        if !valid_rect(bounds) {
            return Err("The prototype hotspot has no finite visible area".into());
        }
        let mut current = Some(visual);
        while let Some(ancestor) = current {
            if !ancestor.is_effectively_visible()
                || ancestor.is_render_suppressed()
                || !ancestor.opacity().is_finite()
                || ancestor.opacity() <= 0.0
                || ancestor
                    .cast::<InputElement>()
                    .is_some_and(|input| !input.is_effectively_enabled())
            {
                return Err("The prototype hotspot is hidden or disabled".into());
            }
            if ancestor
                .cast::<Layoutable>()
                .is_some_and(|layout| !layout.is_measure_valid() || !layout.is_arrange_valid())
            {
                return Err("Wait for the prototype hotspot's native layout to settle".into());
            }
            let transform = ancestor
                .transform_to_visual(&host)
                .ok_or("The prototype hotspot is detached from its stage")?;
            if ancestor == host || ancestor.clip_to_bounds() {
                let clip = Rect::from_size(ancestor.bounds().size()).transform_to_aabb(transform);
                if !valid_rect(clip) {
                    return Err("The prototype hotspot's clip has no finite visible area".into());
                }
                bounds = bounds.intersect(clip);
            }
            if let Some(clip) = ancestor.clip() {
                let clip = clip.bounds().transform_to_aabb(transform);
                if !valid_rect(clip) {
                    return Err("The prototype hotspot's clip has no finite visible area".into());
                }
                bounds = bounds.intersect(clip);
            }
            if !valid_rect(bounds) {
                return Err("The prototype hotspot is outside the visible stage viewport".into());
            }
            if ancestor == host {
                return Ok(bounds);
            }
            current = ancestor.visual_parent();
        }
        Err("The prototype hotspot no longer belongs to this stage".into())
    }

    /// Validate a supplied current buffer without synchronizing, constructing or
    /// refreshing anything. Read-only automation uses this before exposing source
    /// identities when an editor change precedes the board's next UI refresh.
    pub fn source_is_current(
        &self,
        id: PreviewStageId,
        uri: &str,
        document: &Document,
        pending: bool,
        options_epoch: u64,
    ) -> bool {
        self.stages
            .iter()
            .any(|stage| stage.id == id && matches!(stage.status, PreviewStageStatus::Ready { .. }))
            && self.frame_is_current(id, uri, document, pending, options_epoch)
    }

    /// Check whether an accepted native frame represents these exact current
    /// buffers, including a paused frame explicitly refreshed at this revision.
    /// This does not grant source-selection or application-input authority.
    /// Hosts must also validate all their current dependency buffers through
    /// `dependencies_are_current` before publishing an image of the frame.
    pub fn frame_is_current(
        &self,
        id: PreviewStageId,
        uri: &str,
        document: &Document,
        pending: bool,
        options_epoch: u64,
    ) -> bool {
        let Some(stage) = self.stages.iter().find(|stage| stage.id == id) else {
            return false;
        };
        let revision = match stage.status {
            PreviewStageStatus::Ready { revision }
            | PreviewStageStatus::Paused {
                revision: Some(revision),
            } => revision,
            _ => return false,
        };
        let Some(rendered) = &stage.rendered else {
            return false;
        };
        !pending
            && document.is_valid()
            && stage.spec.configuration.source_uri == uri
            && revision == document.revision()
            && rendered.document.revision() == revision
            && rendered.document.source() == document.source()
            && Arc::ptr_eq(&rendered.document.ast_snapshot(), &document.ast_snapshot())
            && rendered.options_epoch == options_epoch
            && rendered.spec.same_rendering(&stage.spec)
            && rendered.spec.configuration.same_layout(&stage.spec.configuration)
            && rendered_dependencies_current(rendered, self.includes.for_document(uri))
    }

    /// Read-only validation against all current host buffers, including native
    /// include dependencies changed before the board's next synchronization.
    pub fn dependencies_are_current(
        &self,
        id: PreviewStageId,
        sources: &[PreviewStageSource],
    ) -> bool {
        let Some(stage) = self.stages.iter().find(|stage| stage.id == id) else {
            return false;
        };
        let Some(rendered) = &stage.rendered else {
            return false;
        };
        let mut current = BTreeMap::new();
        for source in sources {
            let Ok(uri) = native_source_uri(&source.uri) else {
                return false;
            };
            if current.insert(uri, source).is_some() {
                return false;
            }
        }
        rendered.source_snapshot.requested_uris().iter().all(|uri| {
            match (rendered.source_catalog.get(uri), current.get(uri).copied()) {
                (Some(accepted), current) => accepted.matches_open_buffer(current),
                (None, None) => true,
                _ => false,
            }
        })
    }

    /// Refresh identities after native attachment or template/item realization.
    /// This neither reloads source nor resets a stage's temporary interaction state.
    pub fn refresh_runtime_mappings(&mut self) {
        for index in 0..self.stages.len() {
            self.refresh_mapping(index);
        }
    }

    /// Refresh only the presented screen. Prototype playback can retain many
    /// screens without traversing every hidden graph on each native layout tick.
    /// The same accepted-source/dependency checks apply as the whole-board update.
    pub fn refresh_runtime_mapping(&mut self, id: PreviewStageId) -> Result<(), String> {
        let index = self.index(id)?;
        self.refresh_mapping(index);
        Ok(())
    }

    fn refresh_mapping(&mut self, index: usize) {
        let stage = &mut self.stages[index];
        if !matches!(stage.status, PreviewStageStatus::Ready { .. }) {
            return;
        }
        if let Some(rendered) = &mut stage.rendered {
            let Some(source) = self.sources.get(&stage.spec.configuration.source_uri) else {
                return;
            };
            if !source.pending
                && source.document.is_valid()
                && source.document.revision() == rendered.document.revision()
                && Arc::ptr_eq(
                    &source.document.ast_snapshot(),
                    &rendered.document.ast_snapshot(),
                )
                && rendered_dependencies_current(rendered, self.includes.for_document(&stage.spec.configuration.source_uri))
            {
                rendered.preview.rebuild_map(&rendered.document);
            }
        }
    }

    pub fn selection_token(
        &self,
        id: PreviewStageId,
        inspector_id: u64,
    ) -> Result<PreviewStageSelectionToken, String> {
        let stage = &self.stages[self.index(id)?];
        let rendered = self.current(stage)?;
        let visual = Inspector::shared()
            .borrow()
            .resolve(inspector_id)?
            .cast::<Visual>()
            .ok_or("The runtime item is not a visual")?;
        let node = rendered
            .preview
            .source_id(&visual)
            .ok_or("The visual is generated or belongs to another stage")?;
        if matches!(stage.spec.configuration.target, PreviewStageTarget::Document)
            && visual.templated_parent().is_some()
        {
            return Err("Open a definition stage to select an instantiated template part".into());
        }
        if !is_in_preview(&visual, &rendered.preview) {
            return Err("The visual no longer belongs to this stage".into());
        }
        Ok(PreviewStageSelectionToken {
            stage: id,
            epoch: stage.epoch,
            inspector_id,
            visual: visual.downgrade(),
            node,
            ast: rendered.document.ast_snapshot(),
            source: rendered.document.source().into(),
        })
    }

    pub fn hit_test_selection(
        &self,
        id: PreviewStageId,
        point: Point,
    ) -> Result<PreviewStageSelectionToken, String> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err("Stage hit-test position must be finite".into());
        }
        let stage = &self.stages[self.index(id)?];
        let rendered = self.current(stage)?;
        let root = rendered
            .preview
            .root()
            .ok_or("The stage has no native content")?;
        let local = stage
            .control
            .translate_point(point, &root)
            .ok_or("The stage is not attached to a native visual tree")?;
        for hit in root.get_visuals_at(local) {
            let mut current = Some(hit);
            while let Some(visual) = current {
                if rendered.preview.source_id(&visual).is_some()
                    && (!matches!(stage.spec.configuration.target, PreviewStageTarget::Document)
                        || visual.templated_parent().is_none())
                {
                    let inspector_id = Inspector::shared().borrow_mut().node_id(&visual);
                    return self.selection_token(id, inspector_id);
                }
                current = visual.visual_parent();
            }
        }
        Err("No authored control at this stage position".into())
    }

    pub fn resolve_selection(
        &self,
        token: &PreviewStageSelectionToken,
    ) -> Result<PreviewStageSelection, String> {
        let stage = &self.stages[self.index(token.stage)?];
        let rendered = self.current(stage)?;
        if stage.epoch != token.epoch
            || !Arc::ptr_eq(&token.ast, &rendered.document.ast_snapshot())
            || token.source != rendered.document.source()
        {
            return Err("The preview stage changed; select the control again".into());
        }
        let visual = Inspector::shared()
            .borrow()
            .resolve(token.inspector_id)?
            .cast::<Visual>()
            .ok_or("The preview visual was retired")?;
        if !token.visual.points_to(&visual)
            || rendered.preview.source_id(&visual) != Some(token.node)
            || !is_in_preview(&visual, &rendered.preview)
        {
            return Err("The preview visual changed; select the control again".into());
        }
        Ok(PreviewStageSelection {
            stage: stage.id,
            source_uri: stage.spec.configuration.source_uri.clone(),
            node: token.node,
            anchor: PreviewSourceAnchor::capture(&rendered.document, token.node).ok(),
            configuration: stage.spec.configuration.clone(),
            revision: rendered.document.revision(),
            ast: token.ast.clone(),
            source: token.source.clone(),
        })
    }

    fn current<'a>(&self, stage: &'a Stage) -> Result<&'a RenderedStage, String> {
        let PreviewStageStatus::Ready { revision } = stage.status else {
            return Err("Refresh the stage before selecting authored source".into());
        };
        let rendered = stage
            .rendered
            .as_ref()
            .ok_or("The stage has no accepted native graph")?;
        let source = self
            .sources
            .get(&stage.spec.configuration.source_uri)
            .ok_or("The stage document is not open")?;
        if source.pending
            || !source.document.is_valid()
            || source.document.revision() != revision
            || rendered.document.source() != source.document.source()
            || !Arc::ptr_eq(
                &rendered.document.ast_snapshot(),
                &source.document.ast_snapshot(),
            )
            || rendered.options_epoch != self.options_epoch
            || !rendered.spec.same_rendering(&stage.spec)
            || !rendered.spec.configuration.same_layout(&stage.spec.configuration)
            || !rendered_dependencies_current(rendered, self.includes.for_document(&stage.spec.configuration.source_uri))
        {
            return Err("The stage source or construction services changed".into());
        }
        Ok(rendered)
    }

    fn reload(&mut self, id: PreviewStageId, force: bool) -> Result<(), String> {
        let index = self.index(id)?;
        let spec = self.stages[index].spec.clone();
        let config = &spec.configuration;
        let catalog = self.includes.for_document(&config.source_uri).clone();
        let Some(source) = self.sources.get(&config.source_uri).cloned() else {
            self.stages[index].observed = None;
            self.stages[index].status = PreviewStageStatus::MissingDocument;
            self.touch(index);
            self.apply_mode(index);
            return Err("The stage's source document is not open".into());
        };
        self.stages[index].observed = Some(SourceStamp::new(&source, self.options_epoch));
        let dependency_keys = observed_dependency_keys(&self.stages[index]);
        self.stages[index].observed_dependencies =
            dependency_stamps(&dependency_keys, &catalog);
        if !source.document.is_valid() {
            self.stages[index].status = PreviewStageStatus::InvalidSource {
                revision: source.document.revision(),
            };
            self.touch(index);
            self.apply_mode(index);
            return Err("Repair the stage's XAML source before refreshing".into());
        }
        if source.pending {
            self.stages[index].status = PreviewStageStatus::PendingSource {
                revision: source.document.revision(),
            };
            self.touch(index);
            self.apply_mode(index);
            return Err("Apply the pending source before refreshing this stage".into());
        }
        if !force && !config.auto_update {
            self.stages[index].status = PreviewStageStatus::Paused {
                revision: self.stages[index]
                    .rendered
                    .as_ref()
                    .map(|rendered| rendered.document.revision()),
            };
            self.touch(index);
            self.apply_mode(index);
            return Ok(());
        }
        // Reopening an unchanged retained tab only rebases its source revision. Keep
        // independent text, focus and scroll state while retiring selection tokens.
        if !force {
            let stage = &mut self.stages[index];
            if let Some(rendered) = stage.rendered.as_mut() {
                if rendered.options_epoch == self.options_epoch
                    && rendered.spec.same_rendering(&spec)
                    && rendered.spec.configuration.same_layout(config)
                    && rendered.document.source() == source.document.source()
                    && Arc::ptr_eq(
                        &rendered.document.ast_snapshot(),
                        &source.document.ast_snapshot(),
                    )
                    && rendered_dependencies_current(rendered, &catalog)
                {
                    rendered.document = source.document;
                    rendered.preview.status = PreviewStatus::Ready {
                        revision: rendered.document.revision(),
                    };
                    stage.status = PreviewStageStatus::Ready {
                        revision: rendered.document.revision(),
                    };
                    self.touch(index);
                    self.apply_mode(index);
                    return Ok(());
                }
            }
        }
        let source_snapshot = includes::snapshot(&catalog);
        let candidate = self.build_candidate(index, &source.document, &source_snapshot, &catalog);
        let attempted = source_snapshot.requested_uris();
        let mut requested = attempted.clone();
        if candidate.is_err() {
            requested.extend(dependency_keys);
        }
        self.stages[index].observed_dependencies =
            dependency_stamps(&requested, &catalog);
        match candidate {
            Ok(candidate) => {
                let stage = &mut self.stages[index];
                if let Some(previous) = stage.rendered.take() {
                    stage
                        .control
                        .children()
                        .remove(previous.scope.clone().upcast::<Control>());
                    drop(previous);
                }
                candidate.scope.set_opacity(1.0);
                stage.control.configure(config.layout, config.environment);
                stage.rendered = Some(candidate);
                stage.status = if config.auto_update {
                    PreviewStageStatus::Ready {
                        revision: source.document.revision(),
                    }
                } else {
                    PreviewStageStatus::Paused {
                        revision: Some(source.document.revision()),
                    }
                };
                self.touch(index);
                self.apply_mode(index);
                self.stages[index].control.update_layout();
                if let Some(rendered) = &mut self.stages[index].rendered {
                    rendered.preview.rebuild_map(&rendered.document);
                }
                Ok(())
            }
            Err(message) => {
                self.stages[index].status = blocked_dependencies(&catalog, &attempted, source.document.revision())
                    .map(|(status, _)| status)
                    .unwrap_or_else(|| PreviewStageStatus::Failed {
                        revision: Some(source.document.revision()),
                        message: message.clone(),
                    });
                self.touch(index);
                self.apply_mode(index);
                Err(message)
            }
        }
    }

    fn build_candidate(
        &self,
        index: usize,
        document: &Document,
        source_snapshot: &SourceSnapshot,
        catalog: &Rc<IncludeCatalog>,
    ) -> Result<RenderedStage, String> {
        let stage = &self.stages[index];
        let spec = &stage.spec;
        let config = &spec.configuration;
        let definition_scope = match &config.target {
            PreviewStageTarget::Document => None,
            PreviewStageTarget::Definition { anchor, options } => {
                Some(crate::definition_projection::scope_for(
                    document,
                    anchor.resolve(document)?,
                    options.clone(),
                    Vec::new(),
                )?)
            }
        };
        let resolved_states = config
            .states
            .iter()
            .map(|state| {
                Ok(crate::preview_stage_scenarios::ResolvedStageState {
                    owner: state.owner.resolve(document)?,
                    group: state.group.as_str(),
                    state: state.state.as_str(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let scope = ThemeVariantScope::new();
        apply_environment(&scope, config.environment);
        apply_scope_layout(&scope, config.layout, config.environment);
        scope.set_opacity(0.0);
        scope.set_is_hit_test_visible(false);
        let host = ContentControl::new();
        host.set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Stretch);
        host.set_vertical_content_alignment(ferroui_base::layout::VerticalAlignment::Stretch);
        let backing = Border::new();
        backing.set_child(Some(host.clone().upcast()));
        scope.set_child(Some(backing.clone().upcast()));
        apply_stage_backing(&scope, &backing, stage.prototype_overlay.get());
        let weak_scope = scope.downgrade();
        let weak_backing = backing.downgrade();
        let prototype_overlay = stage.prototype_overlay.clone();
        let background_subscription = scope.property_changed(move |change| {
            if change.is_effective_value_change()
                && change.property()
                    == ThemeVariantScope::actual_theme_variant_property().as_property()
            {
                if let (Some(scope), Some(backing)) = (weak_scope.upgrade(), weak_backing.upgrade())
                {
                    apply_stage_backing(&scope, &backing, prototype_overlay.get());
                }
            }
        });
        // Candidate inheritance and resource lookup see the same rooted logical tree
        // as the old graph, without detaching it before all native validation succeeds.
        stage
            .control
            .children()
            .add(scope.clone().upcast::<Control>());
        // Application factories and registered constructors are extension points.
        // Match the main development reload driver's panic containment so a bad
        // candidate cannot strand its hidden host or tear down the entire board.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            source_snapshot.run(|| {
                let mut options = self.options.clone();
                options.design_time_data = config.design_time_data;
                let mut preview = RuntimePreview::new_isolated_with_fixture(
                    host,
                    config.source_uri.clone(),
                    document,
                    options,
                    spec.design_fixture.clone(),
                    definition_scope,
                )?;
                match &preview.status {
                    PreviewStatus::Ready { revision } if *revision == document.revision() => {}
                    PreviewStatus::Failed { message, .. } => return Err(message.clone()),
                    _ => {
                        return Err("The candidate preview did not accept the current source".into())
                    }
                }
                preview
                    .set_interactive(config.mode == PreviewStageMode::Live && config.auto_update);
                stage.control.update_layout();
                preview.rebuild_map(document);
                let scenario = if resolved_states.is_empty() {
                    None
                } else {
                    Some(crate::preview_stage_scenarios::apply_stage_states(
                        document,
                        &mut preview,
                        &resolved_states,
                    )?)
                };
                if let Some((_, message)) = blocked_dependencies(catalog, &source_snapshot.requested_uris(), document.revision())
                {
                    return Err(message);
                }
                Ok(RenderedStage {
                    _scenario: scenario,
                    background_subscription: background_subscription.clone(),
                    backing: backing.clone(),
                    preview,
                    scope: scope.clone(),
                    document: document.clone(),
                    spec: spec.clone(),
                    options_epoch: self.options_epoch,
                    source_snapshot: source_snapshot.clone(),
                    source_catalog: catalog.clone(),
                })
            })
        }))
        .unwrap_or_else(|payload| {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_owned())
                })
                .unwrap_or_else(|| "A registered native constructor panicked".into());
            Err(format!("Native preview construction failed: {message}"))
        });
        if result.is_err() {
            background_subscription.dispose();
            stage.control.children().remove(scope.upcast::<Control>());
        }
        result
    }

    fn apply_mode(&mut self, index: usize) {
        let stage = &mut self.stages[index];
        if let Some(rendered) = &mut stage.rendered {
            let live = stage.spec.configuration.mode == PreviewStageMode::Live
                && stage.spec.configuration.auto_update
                && matches!(stage.status, PreviewStageStatus::Ready { .. });
            rendered.scope.set_is_hit_test_visible(live);
            if rendered._scenario.is_none() {
                rendered.preview.set_interactive(live);
            }
        }
    }

    fn apply_layout(&mut self, index: usize) -> Result<(), String> {
        let stage = &mut self.stages[index];
        let config = &stage.spec.configuration;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stage.control.configure(config.layout, config.environment);
            if let Some(rendered) = &mut stage.rendered {
                apply_scope_layout(&rendered.scope, config.layout, config.environment);
            }
            stage.control.update_layout();
            if let Some(rendered) = &mut stage.rendered {
                rendered.preview.rebuild_map(&rendered.document);
                rendered.spec.configuration.layout = config.layout;
                rendered.spec.configuration.environment.width = config.environment.width;
                rendered.spec.configuration.environment.height = config.environment.height;
            }
        }));
        if let Err(payload) = result {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|value| (*value).to_owned())
                })
                .unwrap_or_else(|| "A native layout callback panicked".into());
            let message = format!("Native preview layout failed: {message}");
            stage.status = PreviewStageStatus::Failed {
                revision: stage
                    .rendered
                    .as_ref()
                    .map(|rendered| rendered.document.revision()),
                message: message.clone(),
            };
            return Err(message);
        }
        Ok(())
    }

    fn index(&self, id: PreviewStageId) -> Result<usize, String> {
        self.stages
            .iter()
            .position(|stage| stage.id == id)
            .ok_or_else(|| "Unknown or retired preview stage".into())
    }
    fn bump(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("Preview board epoch exhausted");
    }
    fn touch(&mut self, index: usize) {
        self.stages[index].epoch = self.stages[index]
            .epoch
            .checked_add(1)
            .expect("Preview stage epoch exhausted");
        self.bump();
    }
}

impl Drop for PreviewStageBoard {
    fn drop(&mut self) {
        for stage in &self.stages {
            stage.control.children().clear();
        }
    }
}

fn native_source_uri(uri: &str) -> Result<String, String> {
    // The text of the URL of the document, the key the reload session and the loaders use
    // (C5 of #423).
    UriRef::parse_absolute(uri)
        .map(String::from)
        .map_err(|_| "Preview source URI must be absolute".into())
}

fn blocked_dependencies(catalog: &IncludeCatalog, dependencies: &BTreeSet<String>, revision: u64) -> Option<(PreviewStageStatus, String)> {
    dependencies.iter().find_map(|uri| catalog.get(uri)?.blocked(uri, revision))
}

fn same_dependency(accepted: Option<&IncludeSource>, current: Option<&IncludeSource>) -> bool {
    match (accepted, current) {
        (Some(accepted), Some(current)) => accepted.same_buffer(current),
        (None, None) => true,
        _ => false,
    }
}

fn rendered_dependencies_current(
    rendered: &RenderedStage,
    current: &IncludeCatalog,
) -> bool {
    rendered
        .source_snapshot
        .requested_uris()
        .iter()
        .all(|uri| same_dependency(rendered.source_catalog.get(uri), current.get(uri)))
}

fn observed_dependency_keys(stage: &Stage) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = stage.observed_dependencies.keys().cloned().collect();
    if let Some(rendered) = &stage.rendered {
        keys.extend(rendered.source_snapshot.requested_uris());
    }
    keys
}

fn observed_dependencies_changed(stage: &Stage, current: &IncludeCatalog) -> bool {
    observed_dependency_keys(stage).iter().any(|uri| {
        // A deferred template can discover an include after the graph is accepted.
        // Compare it with the complete source view captured at construction, never
        // adopt the host's newer buffer as the graph's assumed baseline.
        let accepted = stage
            .observed_dependencies
            .get(uri)
            .map(Option::as_ref)
            .unwrap_or_else(|| {
                stage
                    .rendered
                    .as_ref()
                    .and_then(|rendered| rendered.source_catalog.get(uri))
            });
        !same_dependency(accepted, current.get(uri))
    })
}

fn dependency_stamps(
    dependencies: &BTreeSet<String>,
    catalog: &IncludeCatalog,
) -> BTreeMap<String, Option<IncludeSource>> {
    dependencies
        .iter()
        .map(|uri| (uri.clone(), catalog.get(uri).cloned()))
        .collect()
}

fn apply_stage_backing(scope: &ThemeVariantScope, backing: &Border, overlay: bool) {
    backing.set_background((!overlay).then(|| {
        SolidColorBrush::parse(
            if scope.actual_theme_variant() == Some(ThemeVariant::dark()) {
                "#FF202020"
            } else {
                "#FFFFFFFF"
            },
        )
        .unwrap()
        .into()
    }));
}

fn apply_environment(scope: &Ref<ThemeVariantScope>, environment: PreviewEnvironment) {
    scope.set_clip_to_bounds(true);
    scope.set_requested_theme_variant(match environment.theme {
        PreviewTheme::Light => Some(ThemeVariant::light()),
        PreviewTheme::Dark => Some(ThemeVariant::dark()),
        PreviewTheme::Inherit => None,
    });
    match environment.flow_direction {
        PreviewFlowDirection::Inherit => scope.clear_value(Visual::flow_direction_property()),
        PreviewFlowDirection::LeftToRight => scope.set_flow_direction(FlowDirection::LeftToRight),
        PreviewFlowDirection::RightToLeft => scope.set_flow_direction(FlowDirection::RightToLeft),
    }
}

fn apply_scope_layout(
    scope: &Ref<ThemeVariantScope>,
    layout: PreviewStageLayout,
    environment: PreviewEnvironment,
) {
    scope.set_width(if layout.fits_width() {
        f64::NAN
    } else {
        environment.width
    });
    scope.set_height(if layout.fits_height() {
        f64::NAN
    } else {
        environment.height
    });
    scope.set_max_width(environment.width);
    scope.set_max_height(environment.height);
}

fn is_in_preview(visual: &Ref<Visual>, preview: &RuntimePreview) -> bool {
    let Some(root) = preview.root() else {
        return false;
    };
    let root = root.upcast::<Visual>();
    let mut current = Some(visual.clone());
    while let Some(visual) = current {
        if visual == root {
            return true;
        }
        current = visual.visual_parent();
    }
    false
}

fn validate_uri(uri: &str) -> Result<(), String> {
    bounded(uri, 8192, "Source URI", false)?;
    UriRef::parse_absolute(uri)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn bounded(value: &str, limit: usize, label: &str, empty: bool) -> Result<(), String> {
    if (!empty && value.trim().is_empty()) || value.len() > limit || value.contains('\0') {
        return Err(format!(
            "{label} must be {}and at most {limit} bytes",
            if empty { "" } else { "nonempty " }
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "preview_stages_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preview_stage_backing_tests.rs"]
mod backing_tests;
