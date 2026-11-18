//! Versioned scenario input shared by designer hosts and native test runners.
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ScenarioStep {
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
