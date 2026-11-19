use crate::mcp::schema as s;
use crate::mcp::{decode, integer};
use crate::DesignerWorkspace;
use ferroui_designer_tools::{CanvasGuideSettings, CANVAS_RULER};
use serde_json::{json, Value};

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_canvas_guides" | "designer_canvas_guides_edit"
    )
}

pub(super) fn tools() -> [Value; 2] {
    let axis = json!({"enum":["Horizontal","Vertical"]});
    let limit = CanvasGuideSettings::MAX_POSITION;
    let position = json!({"type":"number","minimum":-limit,"maximum":limit});
    let lines = s::array(
        s::object(
            json!({"id":s::id(),"axis":axis,"position":position}),
            &["axis", "position"],
        ),
        CanvasGuideSettings::MAX_GUIDES,
    );
    let boolean = json!({"type":"boolean"});
    let mut branches = Vec::new();
    for (action, fields, required) in [
        (
            "visibility",
            json!({"rulers":boolean,"guides":boolean}),
            vec![],
        ),
        (
            "add",
            json!({"axis":axis,"position":position}),
            vec!["axis", "position"],
        ),
        (
            "move",
            json!({"id":s::id(),"position":position}),
            vec!["id", "position"],
        ),
        (
            "duplicate",
            json!({"id":s::id(),"position":position}),
            vec!["id", "position"],
        ),
        ("remove", json!({"id":s::id()}), vec!["id"]),
        ("clear", json!({}), vec![]),
        (
            "apply",
            json!({"rulers":boolean,"guides":boolean,"lines":lines}),
            vec!["rulers", "guides", "lines"],
        ),
    ] {
        branches.push(s::action(action, fields, &required, false));
    }
    [
        s::tool("designer_canvas_guides", "Read the active canvas scope, ruler visibility, and guides in logical coordinates.", s::object(json!({}), &[]), false),
        s::tool("designer_canvas_guides_edit", "Change the local canvas guides or visibility using the reported revision and epoch. Apply edits a complete set atomically.",
            s::object(json!({"expected_revision":s::revision(),"expected_epoch":s::revision(),"edit":{"oneOf":branches}}), &["expected_revision", "expected_epoch", "edit"]), true),
    ]
}

pub(super) fn call(
    workspace: &DesignerWorkspace,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == "designer_canvas_guides_edit" {
        let accepted = workspace.canvas_guide_snapshot();
        if integer(args, "expected_epoch")? != accepted.epoch() {
            return Err("Read the current canvas guides before editing".into());
        }
        workspace.apply_canvas_guide_edit(&accepted, decode(args, "edit")?)?;
    }
    let owner = workspace.session();
    let session = owner.borrow();
    Ok(json!({
        "revision":session.document().revision(), "epoch":workspace.canvas_guide_snapshot().epoch(),
        "uri":session.document_uri(), "scope":session.definition_scope().map(|scope|scope.node),
        "settings":workspace.canvas_guides(), "selected":workspace.selected_canvas_guide(), "ruler":CANVAS_RULER,
        "maxGuides":CanvasGuideSettings::MAX_GUIDES, "maxPosition":CanvasGuideSettings::MAX_POSITION,
    }))
}
