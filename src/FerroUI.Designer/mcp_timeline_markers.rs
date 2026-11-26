use super::{number, schema as s, text};
use crate::DesignerSession;
use ferroui_designer_animation::PreviewMarkerEdit;
use serde_json::{Value, json};

const LIST: &str = "designer_timeline_markers";
const EDIT: &str = "designer_timeline_marker_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, LIST | EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let scope = || json!({"program":s::id(),"all":{"type":"boolean"}});
    let action = |name, timed| {
        let mut fields = scope();
        fields["name"] = json!({"type":"string","minLength":1,"maxLength":48});
        if timed {
            fields["time"] = s::time();
        }
        let required: &[&str] = if timed { &["name", "time"] } else { &["name"] };
        s::action(name, fields, required, true)
    };
    let mut query = scope();
    query["expected_revision"] = s::revision();
    [
        s::tool(
            LIST,
            "List named preview positions for a clip. Set all to true for the shared elapsed ruler.",
            s::object(query, &["expected_revision"]),
            false,
        ),
        s::actions(
            EDIT,
            "Set, remove, or seek to a session-local marker. Time uses the selected clip's clock, or elapsed seconds with all true.",
            vec![
                action("set", true),
                action("remove", false),
                action("seek", false),
            ],
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    tool: &str,
    args: &Value,
) -> Result<Value, String> {
    let program = args["program"].as_u64();
    let all = args["all"] == true;
    if all && program.is_some() {
        return Err("Choose a clip or the shared elapsed ruler".into());
    }
    let mut position = None;
    if tool == EDIT {
        let name = text(args, "name")?;
        match args["action"].as_str() {
            Some("set") => session.edit_preview_marker(
                program,
                all,
                PreviewMarkerEdit::Set {
                    name,
                    time: number(args, "time")?,
                },
            )?,
            Some("remove") => {
                session.edit_preview_marker(program, all, PreviewMarkerEdit::Remove { name })?
            }
            Some("seek") => {
                session.seek_preview_marker(program, all, &name)?;
                position = Some(session.time());
            }
            _ => return Err("Choose a marker action".into()),
        }
    }
    let mut result = json!({"revision":session.document().revision(),"markers":session.preview_markers(program, all)?});
    if let Some(time) = position {
        result["time"] = json!(time);
    }
    Ok(result)
}
