use super::{integer, s, state, text};
use crate::{DesignerWorkspace, OutlinePlacement};
use ferroui_designer_project::design_handoff::{Access, ManifestFile};
use serde_json::{json, Value};
use std::rc::Rc;

const REVIEW: &str = "designer_design_handoff";
const APPLY: &str = "designer_apply_handoff";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, REVIEW | APPLY)
}

pub(super) fn tools() -> [Value; 2] {
    let mut apply = s::tool(APPLY, "Apply a checked handoff mapping in Studio. Copy an asset, reveal or preview a Library definition, or place a mapped control or image. Place specifies the current authored revision, anchor and position. OpenReview shows the same native review window.",
        s::object(json!({"manifest":s::string(),"id":s::string(),"review":s::string(),
            "action":{"enum":["copy","place","source","preview","openAsset","openReview"]},
            "anchor":s::id(),"position":{"enum":["before","inside","after"]},"expected_revision":s::revision()}),
            &["manifest","action"]), true);
    apply["annotations"]["destructiveHint"] = false.into();
    [
        s::tool(REVIEW, "Review a solution-relative design handoff manifest against the active XAML or RSX design. Returns exact targets, source references, transfer paths and review IDs.",
            s::object(json!({"manifest":s::string()}), &["manifest"]), false),
        apply,
    ]
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let solution = workspace
        .solution()
        .ok_or("Open a local solution for design handoff")?;
    let path = ManifestFile::in_workspace(&solution.root, &text(args, "manifest")?)
        .map_err(|error| error.to_string())?;
    let access = Access::Workspace;
    if name == REVIEW {
        return serde_json::to_value(workspace.review_handoff(&path, access)?)
            .map_err(|error| error.to_string());
    }
    let action = text(args, "action")?;
    if action == "openReview" {
        workspace.review_handoff(&path, access)?;
        workspace.open_handoff_review(Some((&path, access)))?;
        return Ok(json!({"opened":true}));
    }
    let id = text(args, "id")?;
    let review = text(args, "review")?;
    if action == "copy" {
        return Ok(json!({"uri":workspace.copy_handoff(&path, access, &id, &review)?}));
    }
    let resolved = workspace.check_handoff(&path, access, &id, &review)?;
    match action.as_str() {
        "place" => {
            let position = match args["position"].as_str() {
                Some("before") => OutlinePlacement::Before,
                Some("inside") => OutlinePlacement::Inside,
                Some("after") => OutlinePlacement::After,
                _ => return Err("Choose a placement position in the outline".into()),
            };
            workspace.place_handoff(
                resolved,
                integer(args, "expected_revision")?,
                integer(args, "anchor")?,
                position,
            )?;
            Ok(json!({"placed":true,"document":state(&workspace.session().borrow())}))
        }
        "source" | "preview" => {
            let entry = resolved
                .row
                .definition
                .ok_or("Choose an available Library definition")?;
            let uri = workspace.open_project_library_definition(entry, action == "preview")?;
            Ok(json!({"uri":uri,"document":state(&workspace.session().borrow())}))
        }
        "openAsset" => {
            Ok(json!({"uri":resolved.row.asset_uri.ok_or("Choose an available asset")?}))
        }
        _ => Err("Choose a supported handoff action".into()),
    }
}
