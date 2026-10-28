//! Retained native comparison stages. The board owns presentation state and native
//! graphs; documents remain snapshots of the host's sole authored transaction model.
use crate::runtime::RuntimePreview;
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewFlowDirection, PreviewHostOptions,
    PreviewStatus, PreviewTheme, RuntimeNode,
};
use ferroui_base::media::{FlowDirection, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Point, Ref, Visual, WeakRef};
use ferroui_controls::{Border, ContentControl, Control, Grid, ThemeVariantScope};
use ferroui_designer_core::{Ast, Document, Node, NodeId};
use ferroui_inspect::{Inspector, TreeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub type PreviewStageId = u64;
pub const MAX_PREVIEW_STAGES: usize = 16;
const MAX_MANIFEST_BYTES: usize = 1_048_576;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(1);

/// A namespace-qualified source path. No runtime or session-local numeric node ID
/// is persisted. Named/keyed paths survive sibling insertion and reordering, while
/// ambiguous unnamed siblings are rejected instead of guessing which view was meant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSourceAnchor {
    pub path: Vec<PreviewSourceSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewSourceSegment {
    pub namespace: String,
    pub type_name: String,
    pub name: Option<String>,
    pub key: Option<String>,
}

impl PreviewSourceSegment {
    fn from_node(node: &Node) -> Self {
        Self {
            namespace: node.namespace_uri.clone().unwrap_or_default(),
            type_name: node.local_name().into(),
            name: node.xaml_name().map(str::to_owned),
            key: node
                .attributes
                .iter()
                .find(|attribute| {
                    attribute.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && attribute.name.rsplit(':').next() == Some("Key")
                })
                .map(|attribute| attribute.value.clone()),
        }
    }

    fn matches(&self, node: &Node) -> bool {
        node.is_element() && *self == Self::from_node(node)
    }
}

impl PreviewSourceAnchor {
    pub fn capture(document: &Document, node: NodeId) -> Result<Self, String> {
        if !document.is_valid() {
            return Err("Repair the XAML before capturing a preview target".into());
        }
        let mut path = Vec::new();
        let mut current = Some(node);
        while let Some(id) = current {
            let node = document
                .ast()
                .node(id)
                .filter(|node| node.is_element())
                .ok_or("Preview target must be an authored element")?;
            path.push(PreviewSourceSegment::from_node(node));
            current = node.parent;
        }
        path.reverse();
        let anchor = Self { path };
        anchor.validate()?;
        if anchor.resolve(document)? != node {
            return Err("Preview target does not have an unambiguous source path".into());
        }
        Ok(anchor)
    }

    pub fn resolve(&self, document: &Document) -> Result<NodeId, String> {
        self.validate()?;
        if !document.is_valid() {
            return Err("The preview target source is invalid".into());
        }
        let root = document
            .ast()
            .node(document.ast().root())
            .ok_or("Missing source root")?;
        if !self.path[0].matches(root) {
            return Err("The preview target root was replaced or renamed".into());
        }
        let mut current = root.id;
        for segment in self.path.iter().skip(1) {
            let owner = document
                .ast()
                .node(current)
                .ok_or("Missing preview target ancestor")?;
            let candidates = owner
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .filter(|node| segment.matches(node))
                .map(|node| node.id)
                .collect::<Vec<_>>();
            current = match candidates.as_slice() {
                [only] => *only,
                [] => return Err("The preview target was removed, renamed or moved".into()),
                _ => return Err("The preview target is ambiguous; name the control or use a unique resource key".into()),
            };
        }
        Ok(current)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.path.is_empty() || self.path.len() > 128 {
            return Err("A preview target path must contain between 1 and 128 elements".into());
        }
        for segment in &self.path {
            bounded(&segment.namespace, 4096, "Target namespace", true)?;
            bounded(&segment.type_name, 512, "Target type", false)?;
            for value in [&segment.name, &segment.key].into_iter().flatten() {
                bounded(value, 4096, "Target name or key", false)?;
            }
        }
        if self.text_size() > 65_536 {
            return Err("Preview target path text exceeds 64 KiB".into());
        }
        Ok(())
    }

    fn text_size(&self) -> usize {
        self.path
            .iter()
            .map(|segment| {
                segment.namespace.len()
                    + segment.type_name.len()
                    + segment.name.as_ref().map_or(0, String::len)
                    + segment.key.as_ref().map_or(0, String::len)
                    + 64
            })
            .sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStageMode {
    #[default]
    Select,
    Live,
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
            && self.environment == other.environment
            && self.design_time_data == other.design_time_data
            && self.states == other.states
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
pub struct PreviewStageInfo {
    pub id: PreviewStageId,
    pub configuration: PreviewStageConfiguration,
    pub epoch: u64,
    pub status: PreviewStageStatus,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageManifest {
    pub version: u32,
    pub stages: Vec<PreviewStageConfiguration>,
    pub active: Option<usize>,
}

pub type PreviewStageSnapshot = PreviewStageManifest;

impl PreviewStageManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported preview-stage manifest version".into());
        }
        if self.stages.len() > MAX_PREVIEW_STAGES {
            return Err(format!(
                "A preview board supports at most {MAX_PREVIEW_STAGES} stages"
            ));
        }
        if self.active.is_some_and(|index| index >= self.stages.len()) {
            return Err("The active preview-stage index is outside the manifest".into());
        }
        for config in &self.stages {
            config.validate()?;
        }
        if self
            .stages
            .iter()
            .map(PreviewStageConfiguration::text_size)
            .sum::<usize>()
            > MAX_MANIFEST_BYTES
        {
            return Err("Preview-stage manifest text exceeds 1 MiB".into());
        }
        Ok(())
    }

    pub fn from_json(source: &str) -> Result<Self, String> {
        if source.len() > MAX_MANIFEST_BYTES {
            return Err("Preview-stage manifest exceeds 1 MiB".into());
        }
        let manifest: Self = serde_json::from_str(source).map_err(|error| error.to_string())?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        let source = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        if source.len() > MAX_MANIFEST_BYTES {
            return Err("Preview-stage manifest exceeds 1 MiB".into());
        }
        Ok(source)
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
    preview: RuntimePreview,
    scope: Ref<ThemeVariantScope>,
    document: Document,
    configuration: PreviewStageConfiguration,
    options_epoch: u64,
}

impl Drop for RenderedStage {
    fn drop(&mut self) {
        self.background_subscription.dispose();
    }
}

struct Stage {
    id: PreviewStageId,
    configuration: PreviewStageConfiguration,
    control: Ref<Grid>,
    epoch: u64,
    status: PreviewStageStatus,
    rendered: Option<RenderedStage>,
    observed: Option<SourceStamp>,
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
}

/// One UI-thread collection of native comparison stages across the host's open
/// document buffers. It never owns document history or performs authored mutations.
pub struct PreviewStageBoard {
    stages: Vec<Stage>,
    active: Option<PreviewStageId>,
    active_uri: String,
    sources: BTreeMap<String, PreviewStageSource>,
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
            active: None,
            active_uri: String::new(),
            sources: BTreeMap::new(),
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

    pub fn stages(&self) -> Vec<PreviewStageInfo> {
        self.stages.iter().map(|stage| self.info(stage)).collect()
    }

    pub fn stage(&self, id: PreviewStageId) -> Option<PreviewStageInfo> {
        self.stages
            .iter()
            .find(|stage| stage.id == id)
            .map(|stage| self.info(stage))
    }

    pub fn control(&self, id: PreviewStageId) -> Option<Ref<Control>> {
        self.stages
            .iter()
            .find(|stage| stage.id == id)
            .map(|stage| stage.control.clone().upcast())
    }

    fn info(&self, stage: &Stage) -> PreviewStageInfo {
        PreviewStageInfo {
            id: stage.id,
            configuration: stage.configuration.clone(),
            epoch: stage.epoch,
            status: stage.status.clone(),
            rendered_revision: stage
                .rendered
                .as_ref()
                .map(|rendered| rendered.document.revision()),
            source_revision: self
                .sources
                .get(&stage.configuration.source_uri)
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
        validate_uri(active_uri)?;
        let mut next = BTreeMap::new();
        for mut source in sources {
            validate_uri(&source.uri)?;
            source.document = source.document.snapshot_without_history();
            if next.insert(source.uri.clone(), source).is_some() {
                return Err("Duplicate preview source URI".into());
            }
        }
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
                let next = self.sources.get(&stage.configuration.source_uri);
                let changed = match (&stage.observed, &next) {
                    (Some(old), Some(next)) => !old.matches_source(next, options_epoch),
                    (None, None) => !matches!(stage.status, PreviewStageStatus::MissingDocument),
                    _ => true,
                };
                changed.then_some(stage.id)
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
        configuration.validate()?;
        if self.stages.len() >= MAX_PREVIEW_STAGES {
            return Err(format!(
                "A preview board supports at most {MAX_PREVIEW_STAGES} stages"
            ));
        }
        let id = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let control = Grid::new();
        control.set_clip_to_bounds(true);
        control.set_width(configuration.environment.width);
        control.set_height(configuration.environment.height);
        self.stages.push(Stage {
            id,
            configuration,
            control,
            epoch: 1,
            status: PreviewStageStatus::MissingDocument,
            rendered: None,
            observed: None,
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
        configuration.validate()?;
        let index = self.index(id)?;
        let previous = self.stages[index].configuration.clone();
        if previous == configuration {
            return Ok(());
        }
        let rendering_changed = !previous.same_rendering(&configuration);
        self.stages[index].configuration = configuration;
        self.touch(index);
        if rendering_changed {
            let _ = self.reload(id, true);
        } else if !self.stages[index].configuration.auto_update {
            self.stages[index].status = PreviewStageStatus::Paused {
                revision: self.stages[index]
                    .rendered
                    .as_ref()
                    .map(|rendered| rendered.document.revision()),
            };
            self.apply_mode(index);
        } else if !previous.auto_update {
            let _ = self.reload(id, false);
        } else {
            self.apply_mode(index);
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
        let mut configuration = self.stages[self.index(id)?].configuration.clone();
        configuration.name = format!(
            "{} copy",
            configuration.name.chars().take(120).collect::<String>()
        );
        self.add(configuration)
    }

    pub fn remove(&mut self, id: PreviewStageId) -> Result<(), String> {
        let index = self.index(id)?;
        let stage = self.stages.remove(index);
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
        let mut config = self.stages[self.index(id)?].configuration.clone();
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
        let source = self.stages[self.index(id)?].configuration.clone();
        if variants.is_empty() || self.stages.len() + variants.len() > MAX_PREVIEW_STAGES {
            return Err("Choose variants within the board's 16-stage capacity".into());
        }
        let mut configs = Vec::new();
        for (name, environment) in variants {
            let mut configuration = source.clone();
            configuration.name = name;
            configuration.environment = environment;
            configuration.validate()?;
            configs.push(configuration);
        }
        configs.into_iter().map(|config| self.add(config)).collect()
    }

    pub fn manifest(&self) -> PreviewStageManifest {
        PreviewStageManifest {
            version: 1,
            stages: self
                .stages
                .iter()
                .map(|stage| stage.configuration.clone())
                .collect(),
            active: self
                .active
                .and_then(|id| self.stages.iter().position(|stage| stage.id == id)),
        }
    }

    pub fn snapshot(&self) -> PreviewStageSnapshot {
        self.manifest()
    }

    /// Imported IDs are freshly allocated so callbacks from a retired board cannot
    /// address a new card. Validation finishes before any old stage is removed.
    pub fn restore_manifest(&mut self, manifest: PreviewStageManifest) -> Result<(), String> {
        manifest.validate()?;
        let mut candidate = Self::new();
        candidate.sources = self.sources.clone();
        candidate.options = self.options.clone();
        candidate.options_epoch = self.options_epoch;
        candidate.active_uri = self.active_uri.clone();
        for configuration in manifest.stages {
            candidate.add(configuration)?;
        }
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
            if stage.configuration.source_uri == old_uri {
                stage.configuration.source_uri = new_uri.into();
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
            } else if matches!(stage.configuration.target, PreviewStageTarget::Document) {
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
        let Some(stage) = self.stages.iter().find(|stage| stage.id == id) else {
            return false;
        };
        let PreviewStageStatus::Ready { revision } = stage.status else {
            return false;
        };
        let Some(rendered) = &stage.rendered else {
            return false;
        };
        !pending
            && document.is_valid()
            && stage.configuration.source_uri == uri
            && revision == document.revision()
            && rendered.document.revision() == revision
            && rendered.document.source() == document.source()
            && Arc::ptr_eq(&rendered.document.ast_snapshot(), &document.ast_snapshot())
            && rendered.options_epoch == options_epoch
            && rendered.configuration.same_rendering(&stage.configuration)
    }

    /// Refresh identities after native attachment or template/item realization.
    /// This neither reloads source nor resets a stage's temporary interaction state.
    pub fn refresh_runtime_mappings(&mut self) {
        for stage in &mut self.stages {
            if !matches!(stage.status, PreviewStageStatus::Ready { .. }) {
                continue;
            }
            if let Some(rendered) = &mut stage.rendered {
                let Some(source) = self.sources.get(&stage.configuration.source_uri) else {
                    continue;
                };
                if !source.pending
                    && source.document.is_valid()
                    && source.document.revision() == rendered.document.revision()
                    && Arc::ptr_eq(
                        &source.document.ast_snapshot(),
                        &rendered.document.ast_snapshot(),
                    )
                {
                    rendered.preview.rebuild_map(&rendered.document);
                }
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
        if matches!(stage.configuration.target, PreviewStageTarget::Document)
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
                    && (!matches!(stage.configuration.target, PreviewStageTarget::Document)
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
            source_uri: stage.configuration.source_uri.clone(),
            node: token.node,
            anchor: PreviewSourceAnchor::capture(&rendered.document, token.node).ok(),
            configuration: stage.configuration.clone(),
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
            .get(&stage.configuration.source_uri)
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
            || !rendered.configuration.same_rendering(&stage.configuration)
        {
            return Err("The stage source or construction services changed".into());
        }
        Ok(rendered)
    }

    fn reload(&mut self, id: PreviewStageId, force: bool) -> Result<(), String> {
        let index = self.index(id)?;
        let config = self.stages[index].configuration.clone();
        let Some(source) = self.sources.get(&config.source_uri).cloned() else {
            self.stages[index].observed = None;
            self.stages[index].status = PreviewStageStatus::MissingDocument;
            self.touch(index);
            self.apply_mode(index);
            return Err("The stage's source document is not open".into());
        };
        self.stages[index].observed = Some(SourceStamp::new(&source, self.options_epoch));
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
                    && rendered.configuration.same_rendering(&config)
                    && rendered.document.source() == source.document.source()
                    && Arc::ptr_eq(
                        &rendered.document.ast_snapshot(),
                        &source.document.ast_snapshot(),
                    )
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
        let candidate = self.build_candidate(index, &source.document);
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
                stage.control.set_width(config.environment.width);
                stage.control.set_height(config.environment.height);
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
                self.stages[index].status = PreviewStageStatus::Failed {
                    revision: Some(source.document.revision()),
                    message: message.clone(),
                };
                self.touch(index);
                self.apply_mode(index);
                Err(message)
            }
        }
    }

    fn build_candidate(&self, index: usize, document: &Document) -> Result<RenderedStage, String> {
        let stage = &self.stages[index];
        let config = &stage.configuration;
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
        scope.set_opacity(0.0);
        scope.set_is_hit_test_visible(false);
        let host = ContentControl::new();
        host.set_horizontal_content_alignment(ferroui_base::layout::HorizontalAlignment::Stretch);
        host.set_vertical_content_alignment(ferroui_base::layout::VerticalAlignment::Stretch);
        let backing = Border::new();
        backing.set_child(Some(host.clone().upcast()));
        scope.set_child(Some(backing.clone().upcast()));
        let set_backing = |scope: &ThemeVariantScope, backing: &Border| {
            backing.set_background(Some(
                SolidColorBrush::parse(
                    if scope.actual_theme_variant() == Some(ThemeVariant::dark()) {
                        "#FF202020"
                    } else {
                        "#FFFFFFFF"
                    },
                )
                .unwrap()
                .into(),
            ));
        };
        set_backing(&scope, &backing);
        let weak_scope = scope.downgrade();
        let weak_backing = backing.downgrade();
        let background_subscription = scope.property_changed(move |change| {
            if change.is_effective_value_change()
                && change.property()
                    == ThemeVariantScope::actual_theme_variant_property().as_property()
            {
                if let (Some(scope), Some(backing)) = (weak_scope.upgrade(), weak_backing.upgrade())
                {
                    set_backing(&scope, &backing);
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
            let sources = self
                .sources
                .values()
                .filter(|source| !source.pending && source.document.is_valid())
                .map(|source| (source.uri.clone(), source.document.source().to_owned()))
                .collect();
            let mut options = self.options.clone();
            options.design_time_data = config.design_time_data;
            let mut preview =
                ferroui_markup_xaml::source_overlay::with_sources(Rc::new(sources), || {
                    RuntimePreview::new_isolated(
                        host,
                        config.source_uri.clone(),
                        document,
                        options,
                        definition_scope,
                    )
                })?;
            match &preview.status {
                PreviewStatus::Ready { revision } if *revision == document.revision() => {}
                PreviewStatus::Failed { message, .. } => return Err(message.clone()),
                _ => return Err("The candidate preview did not accept the current source".into()),
            }
            preview.set_interactive(config.mode == PreviewStageMode::Live && config.auto_update);
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
            Ok(RenderedStage {
                _scenario: scenario,
                background_subscription: background_subscription.clone(),
                preview,
                scope: scope.clone(),
                document: document.clone(),
                configuration: config.clone(),
                options_epoch: self.options_epoch,
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
            let live = stage.configuration.mode == PreviewStageMode::Live
                && stage.configuration.auto_update
                && matches!(stage.status, PreviewStageStatus::Ready { .. });
            rendered.scope.set_is_hit_test_visible(live);
            if rendered._scenario.is_none() {
                rendered.preview.set_interactive(live);
            }
        }
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

fn apply_environment(scope: &Ref<ThemeVariantScope>, environment: PreviewEnvironment) {
    scope.set_width(environment.width);
    scope.set_height(environment.height);
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
    ferroui_base::utilities::Uri::absolute(uri)
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
