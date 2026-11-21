use super::{decode, index, s, text};
use crate::DesignerWorkspace;
use ferroui_designer_language::SourceReviewRequest;
use serde::Deserialize;
use serde_json::{Value, json};
use std::rc::Rc;

const TARGETS: &str = "designer_resource_move_targets";
const PLAN: &str = "designer_plan_resource_move";
const APPLY: &str = "designer_move_resource";

#[derive(Deserialize)]
struct Destination {
    uri: String,
    request: SourceReviewRequest,
}

pub(super) fn handles(name: &str) -> bool {
    matches!(name, TARGETS | PLAN | APPLY)
}

pub(super) fn tools() -> [Value; 3] {
    let where_at = json!({"offset": s::revision(), "expected_revision": s::revision()});
    let read = s::tool(
        TARGETS,
        "Find the resource declaration at a UTF-8 byte offset in the active XAML file and list project destination dictionaries.",
        s::object(where_at.clone(), &["offset", "expected_revision"]),
        false,
    );
    let position = s::object(
        json!({"line": s::revision(), "character": s::revision()}),
        &["line", "character"],
    );
    let request = s::object(
        json!({"version": {"type":"integer"}, "position": position}),
        &["version", "position"],
    );
    let mut fields = where_at;
    fields["destination"] = s::object(
        json!({"uri":s::string(), "request":request}),
        &["uri", "request"],
    );
    let plan = s::tool(
        PLAN,
        "Review exact patches for moving an authored XAML resource. Copy uri and request from one destination returned by designer_resource_move_targets.",
        s::object(
            fields.clone(),
            &["offset", "expected_revision", "destination"],
        ),
        false,
    );
    fields["review"] = s::string();
    let apply = s::tool(
        APPLY,
        "Apply the reviewed XAML resource move as one Studio Undo step. Copy review from designer_plan_resource_move.",
        s::object(
            fields,
            &["offset", "expected_revision", "destination", "review"],
        ),
        true,
    );
    [read, plan, apply]
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let offset = index(args, "offset")?;
    if name == TARGETS {
        return Ok(json!(workspace.resource_move_targets_at(offset)?));
    }
    let destination: Destination = decode(args, "destination")?;
    let review = (name == APPLY).then(|| text(args, "review")).transpose()?;
    Ok(json!(workspace.resource_move(
        offset,
        &destination.uri,
        &destination.request,
        review.as_deref()
    )?))
}
