use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioReport {
    pub name: String,
    pub document: PathBuf,
    pub passed: bool,
    pub notices: Vec<String>,
    pub steps: Vec<ScenarioStepReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<BaselineReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visuals: Vec<VisualReview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScenarioStepReport {
    pub step: usize,
    pub passed: bool,
    pub message: String,
    pub artifacts: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BaselineReference {
    pub name: String,
    pub commit: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualReview {
    pub step: usize,
    pub baseline: PathBuf,
    pub workspace: BaselineComparison,
    pub comparison: BaselineComparison,
    pub actual: PathBuf,
    pub expected: Option<PathBuf>,
    pub difference: Option<PathBuf>,
    pub approval: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineComparison {
    pub status: ComparisonStatus,
    pub actual: ImageSize,
    pub expected: Option<ImageSize>,
    pub changed_pixels: Option<u32>,
    pub max_changed_pixels: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComparisonStatus {
    Match,
    Changed,
    Missing,
    Resized,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioSuiteReport {
    pub passed: bool,
    pub scenarios: Vec<ScenarioSuiteEntry>,
    pub review_report: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScenarioSuiteEntry {
    pub path: PathBuf,
    pub report: Option<ScenarioReport>,
    pub error: Option<String>,
}
