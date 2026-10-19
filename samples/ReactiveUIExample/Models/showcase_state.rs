//! Port of `Models/ShowcaseState.cs`: the small, serializable state restored between desktop
//! sessions.

use serde::{Deserialize, Serialize};

/// The small, serializable state restored between desktop sessions (`ShowcaseState`): the command
/// input and the CPU warning threshold.
///
/// The members are named as the properties of the reference in the saved document
/// (`WorkItemText`, `CpuWarningThreshold`), as its JSON serializer writes them. A document without
/// a member reads the default of the member: no input, and a threshold of zero.
///
/// Deviation (DEVIATIONS.md, samples): the input is an `Option`, as the record of the reference
/// reads `null` for a document without it (its `string` is not checked by the serializer).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ShowcaseState {
    /// The command input (`WorkItemText`).
    #[serde(default)]
    pub work_item_text: Option<String>,
    /// The CPU warning threshold (`CpuWarningThreshold`).
    #[serde(default)]
    pub cpu_warning_threshold: f64,
}

impl ShowcaseState {
    /// The state of `work_item_text` and `cpu_warning_threshold`
    /// (`ShowcaseState(string WorkItemText, double CpuWarningThreshold)`).
    pub fn new(work_item_text: impl Into<String>, cpu_warning_threshold: f64) -> Self {
        Self { work_item_text: Some(work_item_text.into()), cpu_warning_threshold }
    }
}
