use super::{decode, ensure_revision, node_id, schema as s, state, text};
use crate::property_transition_session::PropertyTransitionEditor;
use crate::property_transition_rsx::{plan_rust_property_transition, rust_property_transition_editor};
use crate::DesignerSession;
use ferroui_designer_animation::native::TransitionPropertyChoice;
use ferroui_designer_animation::{PropertyTransitionCommand, PropertyTransitionTiming};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxProjection;
use serde_json::{json, Value};

pub const PROPERTY_TRANSITIONS: &str = "designer_property_transitions";
pub const PROPERTY_TRANSITION_EDIT: &str = "designer_property_transition_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, PROPERTY_TRANSITIONS | PROPERTY_TRANSITION_EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let timing = s::object(
        json!({"duration":s::time(),"delay":s::time(),"easing":s::easing()}),
        &[],
    );
    [
        s::tool(
            PROPERTY_TRANSITIONS,
            "Read an authored XAML or mapped Rust transition collection, editable timing, and compatible native properties. Supply target_type for a standalone XAML resource.",
            s::object(json!({"owner":s::id(),"target_type":s::string()}), &["owner"]),
            false,
        ),
        s::actions(
            PROPERTY_TRANSITION_EDIT,
            "Edit the accepted XAML or mapped Rust transition collection. Choose a property and optional type_name from designer_property_transitions. Move uses the final zero-based collection position. Times are in seconds.",
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
    if name == PROPERTY_TRANSITION_EDIT {
        ensure_revision(session, args)?;
        session.edit_property_transition(owner, resource_target, command(args)?)?;
        return Ok(json!({"document":state(session)}));
    }
    let editor = session.property_transition_editor(owner, resource_target)?;
    let source = if session.uses_rust_source() {
        session.authored_source()
    } else {
        session.document().valid_source()
    };
    Ok(report(editor, source))
}

pub fn rust_property_transition_report(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Value, String> {
    require_rust_owner(args)?;
    let editor = session.rust_property_transition_editor(node_id(args, "owner")?, projection)?;
    Ok(report(editor, source))
}

pub fn rust_property_transition_source_plan(
    session: &DesignerSession,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    require_rust_owner(args)?;
    ensure_revision(session, args)?;
    session
        .rust_property_transition_plan(node_id(args, "owner")?, projection, &command(args)?)
}

/// Report source ranges and choices after the native preview has resolved the owner.
pub fn authored_property_transition_report(
    document: &Document,
    source: &str,
    owner: NodeId,
    projection: Option<&RsxProjection<'_>>,
    native_options: Vec<TransitionPropertyChoice>,
) -> Result<Value, String> {
    let editor = if let Some(projection) = projection {
        rust_property_transition_editor(document, owner, projection, true, Some(&native_options))?
    } else {
        let list = ferroui_designer_animation::PropertyTransitionList::read(document, owner)
            .map_err(|error| error.to_string())?;
        PropertyTransitionEditor {
            requires_target: list.target_owner.is_none(),
            list,
            choices: native_options,
            can_edit: true,
        }
    };
    Ok(report(editor, source))
}

/// Plan an original Rust edit using property options from the accepted live registry.
pub fn mapped_property_transition_plan(
    document: &Document,
    owner: NodeId,
    projection: &RsxProjection<'_>,
    native_options: &[TransitionPropertyChoice],
    command: &PropertyTransitionCommand,
) -> Result<Vec<TextPatch>, String> {
    plan_rust_property_transition(document, owner, projection, command, Some(native_options))
}

fn require_rust_owner(args: &Value) -> Result<(), String> {
    if args["target_type"].is_string() {
        return Err("The mapped Rust control supplies this transition target".into());
    }
    Ok(())
}

fn report(editor: PropertyTransitionEditor, source: &str) -> Value {
    let model = editor.list;
    let source = LineIndex::new(source);
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
    json!({
        "owner":model.owner, "collection":model.collection, "canEdit":editor.can_edit,
        "canEditCollection":editor.can_edit && model.collection.is_some(),
        "canAdd":editor.can_edit && model.can_add() && !editor.choices.is_empty(),
        "requiresTarget":editor.requires_target,
        "properties":editor.choices,
        "range":model.source.and_then(|span| source.range(span).ok()), "entries":entries
    })
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
