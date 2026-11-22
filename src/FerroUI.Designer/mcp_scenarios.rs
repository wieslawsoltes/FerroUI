use super::s;
use crate::{
    workspace::{scenario_review::OpenRun, scenarios::ScenarioProgress},
    DesignerWorkspace,
};
use ferroui_designer_project::scenario_process::Options;
use serde_json::{json, Value};
use std::rc::Rc;

const LIST: &str = "ferroui_scenario_list";
const RUN: &str = "ferroui_scenario_run";
const REVIEW: &str = "ferroui_scenario_review";
const APPROVE: &str = "ferroui_scenario_approve_baseline";
const RESULT: &str = "ferroui_scenario_result";
const CANCEL: &str = "ferroui_scenario_cancel";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, LIST | RUN | REVIEW | APPROVE | RESULT | CANCEL)
}

pub(super) fn tools() -> [Value; 6] {
    [
        s::tool(LIST, "List saved scenarios in the open Studio project folder.", s::object(json!({}), &[]), false),
        s::tool(RUN, "Verify saved native render, accessibility, and interaction assertions. Read results with ferroui_scenario_result.",
            s::object(json!({"path":s::string()}), &["path"]), false),
        s::tool(REVIEW, "Review expected, actual, and difference PNGs. Optional baseline_ref selects a Git merge base. Set open to show Studio's comparison. Read the run result before approving.",
            s::object(json!({"path":s::string(), "baseline_ref":s::string(), "open":{"type":"boolean"}}), &["path"]), false),
        s::tool(APPROVE, "Publish one checked screenshot using its approval identifier and the same scenario and baseline_ref. Read the run result for the approval outcome.",
            s::object(json!({"path":s::string(), "baseline_ref":s::string(), "approval":{"type":"string", "minLength":16, "maxLength":16}}), &["path", "approval"]), true),
        s::tool(RESULT, "Read the active Studio scenario's progress and completed report.", s::object(json!({"run":s::id()}), &["run"]), false),
        s::tool(CANCEL, "Cancel the selected Studio scenario run.", s::object(json!({"run":s::id()}), &["run"]), true),
    ]
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == LIST {
        return Ok(json!({"scenarios":workspace.list_scenarios()?}));
    }
    if matches!(name, RESULT | CANCEL) {
        let id = args["run"].as_u64().ok_or("Choose a Studio scenario run")?;
        if name == CANCEL {
            workspace.cancel_scenario(id)?;
        }
        return Ok(match workspace.scenario_progress(id)? {
            ScenarioProgress::Running => json!({"run":id, "status":"running"}),
            ScenarioProgress::Complete(report) => {
                json!({"run":id, "status":"complete", "report":report.as_ref()})
            }
            ScenarioProgress::Failed(error) => json!({"run":id, "status":"failed", "error":error}),
        });
    }
    let path = args["path"]
        .as_str()
        .ok_or("Pass a scenario file relative to the Studio project folder")?;
    let approval = if name == APPROVE {
        Some(
            args["approval"]
                .as_str()
                .ok_or("Pass the approval from the review")?
                .into(),
        )
    } else {
        None
    };
    let reference = args["baseline_ref"].as_str().map(str::to_owned);
    let options = Options {
        review: name == REVIEW,
        reference: reference.clone(),
        approval,
    };
    let id = workspace.start_scenario(path, &options)?;
    if name == REVIEW && args["open"] == true {
        if let Err(error) = workspace.open_scenario_review(Some(OpenRun {
            id,
            path: path.into(),
            reference,
        })) {
            let _ = workspace.cancel_scenario(id);
            return Err(error);
        }
    }
    Ok(json!({"run":id, "status":"running"}))
}
