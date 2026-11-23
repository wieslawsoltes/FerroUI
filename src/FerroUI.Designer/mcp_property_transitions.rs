use super::{decode, node_id, schema as s, state, text};
use crate::property_transition_session::PropertyTransitionCommand;
use crate::DesignerSession;
use ferroui_designer_animation::PropertyTransitionTiming;
use ferroui_designer_language::LineIndex;
use serde_json::{json, Value};

const QUERY: &str = "designer_property_transitions";
const EDIT: &str = "designer_property_transition_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let timing = s::object(
        json!({"duration":s::time(),"delay":s::time(),"easing":s::easing()}),
        &[],
    );
    [
        s::tool(
            QUERY,
            "Read an authored XAML transition collection, editable timing, and compatible native properties. Supply target_type for a standalone resource.",
            s::object(json!({"owner":s::id(),"target_type":s::string()}), &["owner"]),
            false,
        ),
        s::actions(
            EDIT,
            "Edit the accepted XAML transition collection. Choose a property and optional type_name from designer_property_transitions. Move uses the final zero-based collection position. Times are in seconds.",
            vec![
                s::action("timing", json!({"owner":s::id(),"transition":s::id(),"timing":timing}),
                    &["owner","transition","timing"], true),
                s::action("add", json!({"owner":s::id(),"property":s::string(),"type_name":s::string(),
                    "target_type":s::string(),"timing":timing}),
                    &["owner","property"], true),
                s::action("retarget", json!({"owner":s::id(),"transition":s::id(),"property":s::string(),
                    "type_name":s::string(),"target_type":s::string()}),
                    &["owner","transition","property"], true),
                s::action("move", json!({"owner":s::id(),"transition":s::id(),"index":s::revision()}),
                    &["owner","transition","index"], true),
                s::action("remove", json!({"owner":s::id(),"transition":s::id()}),
                    &["owner","transition"], true),
            ],
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let owner = node_id(args, "owner")?;
    let resource_target = args["target_type"].as_str();
    if name == EDIT {
        session.edit_property_transition(owner, resource_target, command(args)?)?;
        return Ok(json!({"document":state(session)}));
    }
    let editor = session.property_transition_editor(owner, resource_target)?;
    let model = editor.list;
    let source = LineIndex::new(session.document().valid_source());
    let entries = model
        .entries
        .iter()
        .map(|entry| {
            json!({
                "id":entry.id, "typeName":entry.type_name, "property":entry.property,
                "duration":entry.duration, "delay":entry.delay, "easing":entry.easing,
                "durationSeconds":entry.duration_seconds, "delaySeconds":entry.delay_seconds,
                "canRetarget":entry.can_retarget,
                "editableTiming":entry.editable_timing, "range":source.range(entry.source).ok()
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "owner":owner, "collection":model.collection, "canEdit":editor.can_edit,
        "canEditCollection":editor.can_edit && model.collection.is_some(),
        "canAdd":editor.can_edit && model.can_add() && !editor.choices.is_empty(),
        "requiresTarget":editor.requires_target,
        "properties":editor.choices,
        "range":model.source.and_then(|span| source.range(span).ok()), "entries":entries
    }))
}

fn command(args: &Value) -> Result<PropertyTransitionCommand, String> {
    let transition = || node_id(args, "transition");
    let type_name = || args["type_name"].as_str().map(str::to_owned);
    Ok(match args["action"].as_str() {
        Some("timing") => PropertyTransitionCommand::Timing {
            transition: transition()?,
            timing: decode(args, "timing")?,
        },
        Some("add") => PropertyTransitionCommand::Add {
            property: text(args, "property")?,
            type_name: type_name(),
            timing: if args.get("timing").is_some() {
                decode(args, "timing")?
            } else {
                PropertyTransitionTiming::default()
            },
        },
        Some("retarget") => PropertyTransitionCommand::Retarget {
            transition: transition()?,
            property: text(args, "property")?,
            type_name: type_name(),
        },
        Some("move") => PropertyTransitionCommand::Move {
            transition: transition()?,
            index: decode(args, "index")?,
        },
        Some("remove") => PropertyTransitionCommand::Remove {
            transition: transition()?,
        },
        _ => return Err("Unknown property transition action".into()),
    })
}
