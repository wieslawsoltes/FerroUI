//! View-to-view interactions are local designer presentation data. Persistent
//! references use stage positions; live references use freshly allocated stage IDs.
use crate::{PreviewSourceAnchor, PreviewStageId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PREVIEW_PROTOTYPE_FLOWS: usize = 32;
pub const MAX_PREVIEW_PROTOTYPE_LINKS: usize = 128;
pub const MAX_PREVIEW_PROTOTYPE_HISTORY: usize = 64;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewPrototypeAction {
    Navigate { stage: PreviewStageId },
    Back,
}

impl PreviewPrototype {
    pub fn is_empty(&self) -> bool {
        self.flows.is_empty() && self.links.is_empty()
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
            if let PreviewPrototypeAction::Navigate { stage } = link.action {
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
                .map(|link| link.name.len() + link.trigger.text_size() + 64)
                .sum::<usize>()
    }

    /// All screens reachable through Navigate edges, including the starting screen.
    /// Back resolves dynamically from session history and adds no static edge.
    pub fn reachable(&self, start: PreviewStageId) -> BTreeSet<PreviewStageId> {
        let mut outgoing = BTreeMap::<PreviewStageId, Vec<PreviewStageId>>::new();
        for link in &self.links {
            if let PreviewPrototypeAction::Navigate { stage } = link.action {
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
        self.links.retain(|link| {
            link.source != stage
                && !matches!(link.action, PreviewPrototypeAction::Navigate { stage: target } if target == stage)
        });
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreviewPrototypeActionManifest {
    Navigate { stage: usize },
    Back,
}

impl PreviewPrototypeManifest {
    pub fn is_empty(&self) -> bool {
        self.flows.is_empty() && self.links.is_empty()
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
                .map(|link| link.name.len() + link.trigger.text_size() + 64)
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
    current: PreviewStageId,
    history: Vec<PreviewStageId>,
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
            current: flow.start,
            history: Vec::new(),
        })
    }

    pub fn flow(&self) -> &str {
        &self.flow
    }
    pub fn current(&self) -> PreviewStageId {
        self.current
    }
    pub fn history(&self) -> &[PreviewStageId] {
        &self.history
    }
    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// Restart navigation; the host decides separately whether native views reset.
    pub fn restart(&mut self) -> PreviewStageId {
        self.current = self.start;
        self.history.clear();
        self.current
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
        if link.source != self.current || !self.graph.links.contains(link) {
            return Err(
                "The prototype interaction does not belong to the current accepted screen".into(),
            );
        }
        match link.action {
            PreviewPrototypeAction::Navigate { stage } => {
                if stage == self.current {
                    return Ok(self.current);
                }
                if self.history.len() >= MAX_PREVIEW_PROTOTYPE_HISTORY {
                    return Err(
                        "Prototype navigation history is full; go back or restart the flow".into(),
                    );
                }
                self.history.push(self.current);
                self.current = stage;
                Ok(self.current)
            }
            PreviewPrototypeAction::Back => self.back(),
        }
    }

    pub fn back(&mut self) -> Result<PreviewStageId, String> {
        let stage = self
            .history
            .pop()
            .ok_or("This prototype flow has no previous screen")?;
        self.current = stage;
        Ok(stage)
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
