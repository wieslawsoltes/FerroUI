use super::{decode, integer, node_id, schema as s, state, text, Permissions};
use crate::DesignerSession;
use crate::rsx_behavior_authoring::{mapped_behaviors, rust_behavior_plan, BehaviorCommand};
use ferroui_designer_animation::{BehaviorEdit, BehaviorModel, BehaviorSource};
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxProjection;
use serde_json::{json, Value};

pub const BEHAVIORS: &str = "designer_behaviors";
pub const BEHAVIOR_EDIT: &str = "designer_behavior";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, BEHAVIORS | BEHAVIOR_EDIT)
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool(BEHAVIORS,
            "Read the selected element's attached behaviors, source ranges, typed actions and available native preview.",
            s::object(json!({"owner":s::id()}), &["owner"]), false),
        s::actions(BEHAVIOR_EDIT,
            "Edit attached behaviors using the current source revision. Action indices refer to the resulting order within a trigger. Preview runs its native actions in the designer and requires allow_input.",
            vec![
                s::action("upsert", json!({"model":s::behavior()}), &["model"], true),
                s::action("remove", json!({"behavior":s::id()}), &["behavior"], true),
                s::action("source", json!({"behavior":s::id(),"source":s::string()}), &["behavior","source"], true),
                s::action("set_trigger", json!({"behavior":s::id(),"trigger":s::behavior_trigger()}),
                    &["behavior","trigger"], true),
                s::action("append_action", json!({"behavior":s::id(),"item":s::behavior_action()}),
                    &["behavior","item"], true),
                s::action("set_action", json!({"behavior":s::id(),"action_node":s::id(),"item":s::behavior_action()}),
                    &["behavior","action_node","item"], true),
                s::action("move_action", json!({"behavior":s::id(),"action_node":s::id(),"index":{"type":"integer","minimum":0,"maximum":4095}}),
                    &["behavior","action_node","index"], true),
                s::action("remove_action", json!({"behavior":s::id(),"action_node":s::id()}), &["behavior","action_node"], true),
                s::action("preview", json!({"behavior":s::id()}), &["behavior"], true),
                s::action("clear_preview", json!({}), &[], true),
            ], true),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
    permissions: Permissions,
) -> Result<Value, String> {
    if name == BEHAVIORS {
        if let Some(rust) = &session.rust {
            return rust_behavior_report(session, rust.source(), &rust.projection()?, args);
        }
        return report(session, node_id(args, "owner")?);
    }
    let mut created = None;
    match text(args, "action")?.as_str() {
        "upsert" => {
            created = Some(session.upsert_behavior(decode::<BehaviorModel>(args, "model")?)?)
        }
        "remove" => session.remove_behavior(node_id(args, "behavior")?)?,
        "source" => {
            session.set_behavior_source(node_id(args, "behavior")?, &text(args, "source")?)?
        }
        "preview" => {
            if !permissions.allow_input {
                return Err("Executing behavior actions requires allow_input".into());
            }
            session.preview_behavior(node_id(args, "behavior")?)?;
        }
        "clear_preview" => session.clear_interaction_preview(),
        action => session.edit_behavior(node_id(args, "behavior")?, change(action, args)?)?,
    }
    Ok(
        json!({"document":state(session),"behavior":created,"activePreview":session.active_behavior_preview()}),
    )
}

fn change(action: &str, args: &Value) -> Result<BehaviorEdit, String> {
    let node = || node_id(args, "action_node");
    Ok(match action {
        "set_trigger" => BehaviorEdit::SetTrigger(decode(args, "trigger")?),
        "append_action" => BehaviorEdit::AppendAction(decode(args, "item")?),
        "set_action" => BehaviorEdit::SetAction {
            node: node()?,
            value: decode(args, "item")?,
        },
        "move_action" => BehaviorEdit::MoveAction {
            node: node()?,
            index: usize::try_from(integer(args, "index")?)
                .map_err(|_| "Action index is out of range")?,
        },
        "remove_action" => BehaviorEdit::RemoveAction(node()?),
        _ => return Err("Unknown behavior action".into()),
    })
}

pub fn rust_behavior_source_plan(
    session: &DesignerSession, projection: &RsxProjection<'_>, arguments: &Value,
) -> Result<Vec<TextPatch>, String> {
    let action = text(arguments, "action")?;
    let command = match action.as_str() {
        "upsert" => BehaviorCommand::Upsert(decode(arguments, "model")?),
        "remove" => BehaviorCommand::Remove(node_id(arguments, "behavior")?),
        _ => BehaviorCommand::Edit(node_id(arguments, "behavior")?, change(&action, arguments)?),
    };
    rust_behavior_plan(session, projection, command)
}

pub fn rust_behavior_report(
    session: &DesignerSession, source: &str, projection: &RsxProjection<'_>, arguments: &Value,
) -> Result<Value, String> {
    let owner = node_id(arguments, "owner")?;
    let document = session.document();
    let (control, entries) = mapped_behaviors(document, projection, owner)?;
    let index = LineIndex::new(source);
    let (_, map) = projection.mapped_markup();
    let range = |id| {
        let offset = document.ast().node(id)?.span.start;
        index.range(map.behavior_source(offset)?).ok()
    };
    Ok(behavior_report(session, owner, entries, projection.can_attach_behavior(control), range))
}

fn report(session: &DesignerSession, owner: u64) -> Result<Value, String> {
    let document = session.document();
    if !document
        .ast()
        .node(owner)
        .is_some_and(|node| node.is_element())
    {
        return Err("Unknown behavior owner".into());
    }
    let index = LineIndex::new(document.valid_source());
    let range = |id| {
        document
            .ast()
            .node(id)
            .and_then(|node| index.range(node.span).ok())
    };
    let sources = session.behavior_sources(owner)?;
    Ok(behavior_report(session, owner, sources, session.can_attach_behavior(owner), range))
}

fn behavior_report(
    session: &DesignerSession, owner: NodeId, sources: Vec<BehaviorSource>, can_attach: bool,
    range: impl Fn(NodeId) -> Option<ferroui_designer_language::Range>,
) -> Value {
    let document = session.document();
    let editable = session.can_manipulate();
    let behaviors: Vec<_> =
        sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                // Keep the original MCP model fields available for clients using upsert.
                let mut row = json!(source.model);
                row["range"] = json!(range(source.node));
                row["canEditTrigger"] = json!(editable && source.can_edit_trigger(document));
                row["canEditActions"] = json!(editable && source.can_edit_actions());
                row["canPreview"] = json!(session.can_preview_behavior(owner, index));
                row["actionRows"] =
                    json!(source.actions.iter().map(|action| {
            json!({"id":action.node,"range":range(action.node),"value":action.value})
        }).collect::<Vec<_>>());
                row
            })
            .collect();
    json!({"revision":document.revision(),"owner":owner,"editable":editable,
        "canAttach":editable && can_attach,"behaviors":behaviors,"activePreview":session.active_behavior_preview()})
}
