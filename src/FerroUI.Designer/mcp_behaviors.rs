use super::{decode, integer, node_id, schema as s, state, text, Permissions};
use crate::DesignerSession;
use ferroui_designer_animation::{BehaviorEdit, BehaviorModel, BehaviorSource};
use ferroui_designer_language::LineIndex;
use serde_json::{json, Value};

const QUERY: &str = "designer_behaviors";
const EDIT: &str = "designer_behavior";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | EDIT)
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool(QUERY,
            "Read the selected element's attached behaviors, source ranges, typed actions and available native preview.",
            s::object(json!({"owner":s::id()}), &["owner"]), false),
        s::actions(EDIT,
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
    if name == QUERY {
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
    let editable = !session.uses_rust_source() && session.can_manipulate();
    let sources = session.behavior_sources(owner)?;
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
    Ok(
        json!({"revision":document.revision(),"owner":owner,"editable":editable,
        "canAttach":editable && BehaviorSource::can_attach(document, owner),"behaviors":behaviors,
        "activePreview":session.active_behavior_preview()}),
    )
}
