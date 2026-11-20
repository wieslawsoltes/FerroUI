use super::selection_workflows::{accepted_report, require_model};
use super::{decode, node_id, schema as s, state};
use crate::DesignerSession;
use ferroui_designer_language::LineIndex;
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember, item_template_resources};
use serde_json::{Value, json};

const QUERY: &str = "designer_item_templates";
const ACTION: &str = "designer_item_template";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | ACTION)
}

pub(super) fn tools() -> Vec<Value> {
    let properties = json!({"enum":ItemTemplateMember::ALL});
    let arguments = json!({"node":s::id(),"property":properties,"expected_model":s::string()});
    let mut assign = arguments.clone();
    assign["resource"] = s::id();
    let required = ["node", "property", "expected_model"];
    vec![
        s::tool(
            QUERY,
            "Read an authored control or column's item template slots, source ranges, local definitions and compatible resources. Use the returned model for item template actions.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::actions(
            ACTION,
            "Open a native item template preview or create, assign, copy or reset an authored XAML template. Preview changes no source. Each source edit is one undo step and checks the accepted model and revision.",
            vec![
                s::action("preview", arguments.clone(), &required, true),
                s::action("create_inline", arguments.clone(), &required, true),
                s::action("copy_inline", arguments.clone(), &required, true),
                s::action("reset", arguments, &required, true),
                s::action(
                    "assign_resource",
                    assign,
                    &["node", "property", "expected_model", "resource"],
                    true,
                ),
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
    let node = node_id(args, "node")?;
    let report = report(session, node)?;
    if name == QUERY {
        return Ok(report);
    }
    require_model(args, &report)?;
    let property = decode::<ItemTemplateMember>(args, "property")?;
    if args["action"] == "preview" {
        session.preview_item_template(node, property)?;
    } else {
        let change = serde_json::from_value::<ItemTemplateChange>(args.clone())
            .map_err(|error| error.to_string())?;
        session.edit_item_template(node, property, change)?;
    }
    Ok(state(session))
}

fn report(session: &DesignerSession, node: u64) -> Result<Value, String> {
    let document = session.document();
    let slots = session.item_template_slots(node)?;
    let resources = if slots.iter().any(|slot| slot.reason.is_none()) {
        item_template_resources(document, node).map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };
    let index = LineIndex::new(document.source());
    let slots = slots
        .iter()
        .map(|slot| {
            json!({
                "property":slot.property, "value":slot.value, "reason":slot.reason,
                "range":slot.span.and_then(|span| index.range(span).ok()),
                "canCopyInline":slot.reason.is_none() && slot.can_copy_inline(document),
                "canReset":slot.reason.is_none() && slot.can_reset
            })
        })
        .collect::<Vec<_>>();
    Ok(accepted_report(
        session,
        QUERY,
        json!({"node":node,"slots":slots,"resources":resources}),
    ))
}
