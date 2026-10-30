//! View-to-view interactions are local designer presentation data. Persistent
//! references use stage positions; live references use freshly allocated stage IDs.
use crate::{PreviewSourceAnchor, PreviewStageId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PREVIEW_PROTOTYPE_FLOWS: usize = 32;
pub const MAX_PREVIEW_PROTOTYPE_LINKS: usize = 128;
pub const MAX_PREVIEW_PROTOTYPE_HISTORY: usize = 64;
pub const MAX_PREVIEW_PROTOTYPE_OVERLAYS: usize = 8;
pub const MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET: i32 = 16_384;
const MAX_PROTOTYPE_TEXT_BYTES: usize = 524_288;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewPrototype {
    pub flows: Vec<PreviewPrototypeFlow>,
    pub links: Vec<PreviewPrototypeLink>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeFlow {
    pub name: String,
    pub start: PreviewStageId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeLink {
    pub name: String,
    pub source: PreviewStageId,
    pub trigger: PreviewSourceAnchor,
    pub action: PreviewPrototypeAction,
}

/// Placement in the base screen's logical coordinate space. The native presenter
/// constrains the resulting visible rectangle to the prototype viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewPrototypeOverlayPlacement {
    #[default]
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Manual {
        x: i32,
        y: i32,
    },
}

// Serde's internally tagged unit variants ignore trailing fields even when the
// enum denies unknown fields. Empty struct variants enforce the complete wire
// shape while keeping the public API's convenient fieldless variants unchanged.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PrototypeOverlayPlacementWire {
    Center {},
    TopLeft {},
    TopCenter {},
    TopRight {},
    CenterLeft {},
    CenterRight {},
    BottomLeft {},
    BottomCenter {},
    BottomRight {},
    Manual { x: i32, y: i32 },
}

impl<'de> Deserialize<'de> for PreviewPrototypeOverlayPlacement {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match PrototypeOverlayPlacementWire::deserialize(deserializer)? {
                PrototypeOverlayPlacementWire::Center {} => Self::Center,
                PrototypeOverlayPlacementWire::TopLeft {} => Self::TopLeft,
                PrototypeOverlayPlacementWire::TopCenter {} => Self::TopCenter,
                PrototypeOverlayPlacementWire::TopRight {} => Self::TopRight,
                PrototypeOverlayPlacementWire::CenterLeft {} => Self::CenterLeft,
                PrototypeOverlayPlacementWire::CenterRight {} => Self::CenterRight,
                PrototypeOverlayPlacementWire::BottomLeft {} => Self::BottomLeft,
                PrototypeOverlayPlacementWire::BottomCenter {} => Self::BottomCenter,
                PrototypeOverlayPlacementWire::BottomRight {} => Self::BottomRight,
                PrototypeOverlayPlacementWire::Manual { x, y } => Self::Manual { x, y },
            },
        )
    }
}

/// Presentation settings for one overlay entry. Outside dismissal is opt-in;
/// explicit CloseOverlay and Back remain available regardless of that setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewPrototypeOverlayOptions {
    pub placement: PreviewPrototypeOverlayPlacement,
    pub dismiss_on_outside: bool,
    pub dim_background: bool,
}

impl Default for PreviewPrototypeOverlayOptions {
    fn default() -> Self {
        Self {
            placement: PreviewPrototypeOverlayPlacement::Center,
            dismiss_on_outside: false,
            dim_background: true,
        }
    }
}

impl PreviewPrototypeOverlayOptions {
    pub fn validate(&self) -> Result<(), String> {
        if let PreviewPrototypeOverlayPlacement::Manual { x, y } = self.placement {
            let range =
                -MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET..=MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET;
            if !range.contains(&x) || !range.contains(&y) {
                return Err(format!(
                    "Manual prototype overlay coordinates must be within ±{MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET} logical units"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeOverlay {
    pub stage: PreviewStageId,
    pub options: PreviewPrototypeOverlayOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewPrototypeAction {
    Navigate {
        stage: PreviewStageId,
    },
    Back,
    OpenOverlay {
        stage: PreviewStageId,
        options: PreviewPrototypeOverlayOptions,
    },
    SwapOverlay {
        stage: PreviewStageId,
        options: PreviewPrototypeOverlayOptions,
    },
    CloseOverlay,
}

// Live stage IDs and persistent stage indices share the same strict action
// grammar. Fieldless actions use empty structs for the same reason as placement.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PrototypeActionWire<T> {
    Navigate {
        stage: T,
    },
    Back {},
    OpenOverlay {
        stage: T,
        #[serde(default)]
        options: PreviewPrototypeOverlayOptions,
    },
    SwapOverlay {
        stage: T,
        #[serde(default)]
        options: PreviewPrototypeOverlayOptions,
    },
    CloseOverlay {},
}

impl<'de> Deserialize<'de> for PreviewPrototypeAction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match PrototypeActionWire::<PreviewStageId>::deserialize(deserializer)? {
                PrototypeActionWire::Navigate { stage } => Self::Navigate { stage },
                PrototypeActionWire::Back {} => Self::Back,
                PrototypeActionWire::OpenOverlay { stage, options } => {
                    Self::OpenOverlay { stage, options }
                }
                PrototypeActionWire::SwapOverlay { stage, options } => {
                    Self::SwapOverlay { stage, options }
                }
                PrototypeActionWire::CloseOverlay {} => Self::CloseOverlay,
            },
        )
    }
}

impl PreviewPrototypeAction {
    pub fn target_stage(&self) -> Option<PreviewStageId> {
        match *self {
            Self::Navigate { stage }
            | Self::OpenOverlay { stage, .. }
            | Self::SwapOverlay { stage, .. } => Some(stage),
            Self::Back | Self::CloseOverlay => None,
        }
    }

    fn is_overlay(&self) -> bool {
        matches!(
            self,
            Self::OpenOverlay { .. } | Self::SwapOverlay { .. } | Self::CloseOverlay
        )
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::OpenOverlay { options, .. } | Self::SwapOverlay { options, .. } => {
                options.validate()
            }
            Self::Navigate { .. } | Self::Back | Self::CloseOverlay => Ok(()),
        }
    }

    fn text_size(&self) -> usize {
        if matches!(self, Self::OpenOverlay { .. } | Self::SwapOverlay { .. }) {
            192
        } else {
            64
        }
    }
}

impl PreviewPrototype {
    pub fn is_empty(&self) -> bool {
        self.flows.is_empty() && self.links.is_empty()
    }

    pub fn has_overlay_actions(&self) -> bool {
        self.links.iter().any(|link| link.action.is_overlay())
    }

    /// Validate the complete graph before installing any of its descriptors.
    /// Authored/native target availability is checked by the host at interaction.
    pub fn validate(&self, stages: &[PreviewStageId]) -> Result<(), String> {
        self.validate_structure()?;
        let stages = stages.iter().copied().collect::<BTreeSet<_>>();
        for flow in &self.flows {
            if !stages.contains(&flow.start) {
                return Err(format!(
                    "The start stage for prototype flow '{}' no longer exists",
                    flow.name
                ));
            }
        }
        for link in &self.links {
            if !stages.contains(&link.source) {
                return Err(format!(
                    "The source stage for prototype interaction '{}' no longer exists",
                    link.name
                ));
            }
            if let Some(stage) = link.action.target_stage() {
                if !stages.contains(&stage) {
                    return Err(format!(
                        "The destination stage for prototype interaction '{}' no longer exists",
                        link.name
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), String> {
        if self.flows.len() > MAX_PREVIEW_PROTOTYPE_FLOWS {
            return Err(format!(
                "A prototype supports at most {MAX_PREVIEW_PROTOTYPE_FLOWS} named flows"
            ));
        }
        if self.links.len() > MAX_PREVIEW_PROTOTYPE_LINKS {
            return Err(format!(
                "A prototype supports at most {MAX_PREVIEW_PROTOTYPE_LINKS} interactions"
            ));
        }
        let mut names = BTreeSet::new();
        for flow in &self.flows {
            validate_label(&flow.name, "Prototype flow name")?;
            if !names.insert(&flow.name) {
                return Err("Prototype flow names must be unique".into());
            }
        }
        let mut triggers = BTreeSet::new();
        for link in &self.links {
            validate_label(&link.name, "Prototype interaction name")?;
            link.trigger.validate()?;
            link.action.validate()?;
            let key = serde_json::to_string(&link.trigger).map_err(|error| error.to_string())?;
            if !triggers.insert((link.source, key)) {
                return Err(
                    "A source control can have only one prototype interaction in each stage".into(),
                );
            }
        }
        if self.text_size() > MAX_PROTOTYPE_TEXT_BYTES {
            return Err("Prototype descriptor text exceeds 512 KiB".into());
        }
        Ok(())
    }

    pub(crate) fn text_size(&self) -> usize {
        self.flows
            .iter()
            .map(|flow| flow.name.len() + 32)
            .sum::<usize>()
            + self
                .links
                .iter()
                .map(|link| link.name.len() + link.trigger.text_size() + link.action.text_size())
                .sum::<usize>()
    }

    /// All views reachable through Navigate/Open/Swap edges, including the start.
    /// Back and CloseOverlay resolve dynamically and add no static edge.
    pub fn reachable(&self, start: PreviewStageId) -> BTreeSet<PreviewStageId> {
        let mut outgoing = BTreeMap::<PreviewStageId, Vec<PreviewStageId>>::new();
        for link in &self.links {
            if let Some(stage) = link.action.target_stage() {
                outgoing.entry(link.source).or_default().push(stage);
            }
        }
        let mut reachable = BTreeSet::new();
        let mut pending = vec![start];
        while let Some(stage) = pending.pop() {
            if reachable.insert(stage) {
                if let Some(targets) = outgoing.get(&stage) {
                    pending.extend(targets.iter().copied());
                }
            }
        }
        reachable
    }

    pub(crate) fn remove_stage(&mut self, stage: PreviewStageId) {
        self.flows.retain(|flow| flow.start != stage);
        self.links
            .retain(|link| link.source != stage && link.action.target_stage() != Some(stage));
    }
}

/// Portable graph descriptors. Every stage reference is an index into the owning
/// preview-stage manifest, never a runtime ID or a source node's transient identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewPrototypeManifest {
    pub flows: Vec<PreviewPrototypeFlowManifest>,
    pub links: Vec<PreviewPrototypeLinkManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeFlowManifest {
    pub name: String,
    pub start: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeLinkManifest {
    pub name: String,
    pub source: usize,
    pub trigger: PreviewSourceAnchor,
    pub action: PreviewPrototypeActionManifest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewPrototypeActionManifest {
    Navigate {
        stage: usize,
    },
    Back,
    OpenOverlay {
        stage: usize,
        options: PreviewPrototypeOverlayOptions,
    },
    SwapOverlay {
        stage: usize,
        options: PreviewPrototypeOverlayOptions,
    },
    CloseOverlay,
}

impl<'de> Deserialize<'de> for PreviewPrototypeActionManifest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match PrototypeActionWire::<usize>::deserialize(deserializer)? {
                PrototypeActionWire::Navigate { stage } => Self::Navigate { stage },
                PrototypeActionWire::Back {} => Self::Back,
                PrototypeActionWire::OpenOverlay { stage, options } => {
                    Self::OpenOverlay { stage, options }
                }
                PrototypeActionWire::SwapOverlay { stage, options } => {
                    Self::SwapOverlay { stage, options }
                }
                PrototypeActionWire::CloseOverlay {} => Self::CloseOverlay,
            },
        )
    }
}

impl PreviewPrototypeManifest {
    pub fn is_empty(&self) -> bool {
        self.flows.is_empty() && self.links.is_empty()
    }

    pub fn has_overlay_actions(&self) -> bool {
        self.links.iter().any(|link| {
            matches!(
                link.action,
                PreviewPrototypeActionManifest::OpenOverlay { .. }
                    | PreviewPrototypeActionManifest::SwapOverlay { .. }
                    | PreviewPrototypeActionManifest::CloseOverlay
            )
        })
    }

    pub fn from_prototype(
        graph: &PreviewPrototype,
        stages: &[PreviewStageId],
    ) -> Result<Self, String> {
        graph.validate(stages)?;
        let indices = stages
            .iter()
            .enumerate()
            .map(|(index, stage)| (*stage, index))
            .collect::<BTreeMap<_, _>>();
        if indices.len() != stages.len() {
            return Err("A prototype stage mapping cannot contain duplicate stage IDs".into());
        }
        Ok(Self {
            flows: graph
                .flows
                .iter()
                .map(|flow| PreviewPrototypeFlowManifest {
                    name: flow.name.clone(),
                    start: indices[&flow.start],
                })
                .collect(),
            links: graph
                .links
                .iter()
                .map(|link| PreviewPrototypeLinkManifest {
                    name: link.name.clone(),
                    source: indices[&link.source],
                    trigger: link.trigger.clone(),
                    action: match link.action {
                        PreviewPrototypeAction::Navigate { stage } => {
                            PreviewPrototypeActionManifest::Navigate {
                                stage: indices[&stage],
                            }
                        }
                        PreviewPrototypeAction::Back => PreviewPrototypeActionManifest::Back,
                        PreviewPrototypeAction::OpenOverlay { stage, options } => {
                            PreviewPrototypeActionManifest::OpenOverlay {
                                stage: indices[&stage],
                                options,
                            }
                        }
                        PreviewPrototypeAction::SwapOverlay { stage, options } => {
                            PreviewPrototypeActionManifest::SwapOverlay {
                                stage: indices[&stage],
                                options,
                            }
                        }
                        PreviewPrototypeAction::CloseOverlay => {
                            PreviewPrototypeActionManifest::CloseOverlay
                        }
                    },
                })
                .collect(),
        })
    }

    /// Validation uses only positional placeholders and never constructs previews.
    pub fn validate(&self, stage_count: usize) -> Result<(), String> {
        if stage_count > crate::MAX_PREVIEW_STAGES {
            return Err("The prototype manifest has too many stage slots".into());
        }
        let stages = (1..=stage_count as u64).collect::<Vec<_>>();
        self.restore(&stages).map(|_| ())
    }

    pub fn restore(&self, stages: &[PreviewStageId]) -> Result<PreviewPrototype, String> {
        if self.flows.len() > MAX_PREVIEW_PROTOTYPE_FLOWS
            || self.links.len() > MAX_PREVIEW_PROTOTYPE_LINKS
        {
            return Err("The prototype manifest exceeds its flow or interaction capacity".into());
        }
        if stages.iter().copied().collect::<BTreeSet<_>>().len() != stages.len() {
            return Err("A prototype stage mapping cannot contain duplicate stage IDs".into());
        }
        let reference = |index: usize| {
            stages
                .get(index)
                .copied()
                .ok_or_else(|| "A prototype stage index is outside the manifest".to_owned())
        };
        let graph = PreviewPrototype {
            flows: self
                .flows
                .iter()
                .map(|flow| {
                    Ok(PreviewPrototypeFlow {
                        name: flow.name.clone(),
                        start: reference(flow.start)?,
                    })
                })
                .collect::<Result<_, String>>()?,
            links: self
                .links
                .iter()
                .map(|link| {
                    Ok(PreviewPrototypeLink {
                        name: link.name.clone(),
                        source: reference(link.source)?,
                        trigger: link.trigger.clone(),
                        action: match link.action {
                            PreviewPrototypeActionManifest::Navigate { stage } => {
                                PreviewPrototypeAction::Navigate {
                                    stage: reference(stage)?,
                                }
                            }
                            PreviewPrototypeActionManifest::Back => PreviewPrototypeAction::Back,
                            PreviewPrototypeActionManifest::OpenOverlay { stage, options } => {
                                PreviewPrototypeAction::OpenOverlay {
                                    stage: reference(stage)?,
                                    options,
                                }
                            }
                            PreviewPrototypeActionManifest::SwapOverlay { stage, options } => {
                                PreviewPrototypeAction::SwapOverlay {
                                    stage: reference(stage)?,
                                    options,
                                }
                            }
                            PreviewPrototypeActionManifest::CloseOverlay => {
                                PreviewPrototypeAction::CloseOverlay
                            }
                        },
                    })
                })
                .collect::<Result<_, String>>()?,
        };
        graph.validate(stages)?;
        Ok(graph)
    }

    pub(crate) fn text_size(&self) -> usize {
        self.flows
            .iter()
            .map(|flow| flow.name.len() + 32)
            .sum::<usize>()
            + self
                .links
                .iter()
                .map(|link| {
                    let action_size = if matches!(
                        link.action,
                        PreviewPrototypeActionManifest::OpenOverlay { .. }
                            | PreviewPrototypeActionManifest::SwapOverlay { .. }
                    ) {
                        192
                    } else {
                        64
                    };
                    link.name.len() + link.trigger.text_size() + action_size
                })
                .sum::<usize>()
    }
}

/// A bounded navigation cursor over one accepted graph. It owns no source session,
/// controls, timers or application state. Hosts validate native/source authority
/// before activating an interaction and can retain native views across Back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPrototypeSession {
    graph: PreviewPrototype,
    flow: String,
    start: PreviewStageId,
    screen: PreviewStageId,
    history: Vec<PreviewStageId>,
    overlays: Vec<PreviewPrototypeOverlay>,
}

impl PreviewPrototypeSession {
    /// The host must first validate graph stage references against its live board.
    /// This operation also validates structure so standalone callers cannot install
    /// ambiguous interactions, invalid anchors or unbounded descriptors.
    pub fn start(graph: &PreviewPrototype, flow_name: &str) -> Result<Self, String> {
        graph.validate_structure()?;
        let flow = graph
            .flows
            .iter()
            .find(|flow| flow.name == flow_name)
            .ok_or("The prototype flow no longer exists")?;
        Ok(Self {
            graph: graph.clone(),
            flow: flow.name.clone(),
            start: flow.start,
            screen: flow.start,
            history: Vec::new(),
            overlays: Vec::new(),
        })
    }

    pub fn flow(&self) -> &str {
        &self.flow
    }
    /// The only stage that accepts an interaction: the top overlay, or the screen.
    pub fn current(&self) -> PreviewStageId {
        self.overlays
            .last()
            .map_or(self.screen, |overlay| overlay.stage)
    }
    /// The base screen beneath any open overlays. History stores these IDs only.
    pub fn screen(&self) -> PreviewStageId {
        self.screen
    }
    /// Overlay entries in back-to-front order, excluding the base screen.
    pub fn overlays(&self) -> &[PreviewPrototypeOverlay] {
        &self.overlays
    }
    /// Visible native hosts in back-to-front order. Only current() accepts actions.
    pub fn active_stages(&self) -> Vec<PreviewStageId> {
        std::iter::once(self.screen)
            .chain(self.overlays.iter().map(|overlay| overlay.stage))
            .collect()
    }
    pub fn history(&self) -> &[PreviewStageId] {
        &self.history
    }
    pub fn can_go_back(&self) -> bool {
        !self.overlays.is_empty() || !self.history.is_empty()
    }

    /// Restart navigation; the host decides separately whether native views reset.
    pub fn restart(&mut self) -> PreviewStageId {
        self.screen = self.start;
        self.history.clear();
        self.overlays.clear();
        self.screen
    }

    pub fn activate_link(&mut self, index: usize) -> Result<PreviewStageId, String> {
        let link = self
            .graph
            .links
            .get(index)
            .cloned()
            .ok_or("The prototype interaction no longer exists")?;
        self.activate(&link)
    }

    pub fn activate(&mut self, link: &PreviewPrototypeLink) -> Result<PreviewStageId, String> {
        if link.source != self.current() || !self.graph.links.contains(link) {
            return Err(
                "The prototype interaction does not belong to the current accepted screen".into(),
            );
        }
        match link.action {
            PreviewPrototypeAction::Navigate { stage } => {
                if stage == self.screen {
                    self.overlays.clear();
                    return Ok(self.screen);
                }
                if self.history.len() >= MAX_PREVIEW_PROTOTYPE_HISTORY {
                    return Err(
                        "Prototype navigation history is full; go back or restart the flow".into(),
                    );
                }
                self.history.push(self.screen);
                self.screen = stage;
                self.overlays.clear();
                Ok(self.screen)
            }
            PreviewPrototypeAction::Back => self.back(),
            PreviewPrototypeAction::OpenOverlay { stage, options } => {
                if self.overlays.len() >= MAX_PREVIEW_PROTOTYPE_OVERLAYS {
                    return Err(format!("A prototype supports at most {MAX_PREVIEW_PROTOTYPE_OVERLAYS} open overlays"));
                }
                if stage == self.screen
                    || self.overlays.iter().any(|overlay| overlay.stage == stage)
                {
                    return Err("The prototype overlay stage is already presented".into());
                }
                self.overlays
                    .push(PreviewPrototypeOverlay { stage, options });
                Ok(stage)
            }
            PreviewPrototypeAction::SwapOverlay { stage, options } => {
                let lower = self
                    .overlays
                    .len()
                    .checked_sub(1)
                    .ok_or("Open a prototype overlay before swapping it")?;
                if stage == self.screen
                    || self.overlays[..lower]
                        .iter()
                        .any(|overlay| overlay.stage == stage)
                {
                    return Err(
                        "The replacement prototype overlay stage is already presented".into(),
                    );
                }
                self.overlays[lower] = PreviewPrototypeOverlay { stage, options };
                Ok(stage)
            }
            PreviewPrototypeAction::CloseOverlay => self.close_overlay(),
        }
    }

    /// Close the top overlay first; otherwise return through screen history.
    pub fn back(&mut self) -> Result<PreviewStageId, String> {
        if !self.overlays.is_empty() {
            return self.close_overlay();
        }
        let stage = self
            .history
            .pop()
            .ok_or("This prototype flow has no previous screen")?;
        self.screen = stage;
        Ok(stage)
    }

    pub fn close_overlay(&mut self) -> Result<PreviewStageId, String> {
        self.overlays
            .pop()
            .ok_or("This prototype flow has no open overlay")?;
        Ok(self.current())
    }
}

fn validate_label(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 128 || value.contains('\0') {
        Err(format!("{label} must be nonempty and at most 128 bytes"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "preview_prototype_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preview_prototype_overlay_tests.rs"]
mod overlay_tests;
