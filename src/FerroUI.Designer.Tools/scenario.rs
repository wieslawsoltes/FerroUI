//! Versioned scenario input shared by designer hosts and native test runners.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub version: u32,
    pub name: String,
    pub document: String,
    #[serde(default)]
    pub viewport: ScenarioViewport,
    #[serde(default)]
    pub caret: ScenarioPosition,
    pub prototype: Option<ScenarioPrototype>,
    pub steps: Vec<ScenarioStep>,
}

impl Scenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!(
                "Unsupported FerroUI scenario version {}",
                self.version
            ));
        }
        if self.name.trim().is_empty() || self.document.trim().is_empty() || self.steps.is_empty() {
            return Err("A scenario needs a name, source document, and at least one step".into());
        }
        if let Some(prototype) = &self.prototype {
            if prototype.stage_set.trim().is_empty() || prototype.flow.trim().is_empty() {
                return Err("A prototype scenario needs a saved stage set and a flow name".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScenarioViewport {
    pub width: u32,
    pub height: u32,
    pub dark: bool,
}

impl Default for ScenarioViewport {
    fn default() -> Self {
        Self {
            width: 800,
            height: 520,
            dark: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScenarioPrototype {
    pub stage_set: String,
    pub flow: String,
    /// Remap a saved source URI to a document in the selected test workspace.
    #[serde(default)]
    pub sources: BTreeMap<String, ScenarioStageSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioStageSource {
    pub path: String,
    pub caret: Option<ScenarioPosition>,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ScenarioStep {
    ActivateLink {
        name: String,
    },
    ExpectStage {
        name: String,
    },
    ExpectOverlay {
        name: Option<String>,
    },
    Click {
        selector: String,
    },
    TypeText {
        selector: String,
        text: String,
    },
    PressKey {
        selector: String,
        key: String,
    },
    ExpectProperty {
        selector: String,
        property: String,
        value: ScenarioValue,
    },
    ExpectAccessible {
        name: String,
        role: String,
        automation_id: Option<String>,
    },
    ExpectScreenshot {
        baseline: String,
        #[serde(default)]
        max_changed_pixels: u32,
        #[serde(default)]
        channel_tolerance: u8,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScenarioValue {
    Text(String),
    Boolean(bool),
    Number(f64),
    Null,
}
