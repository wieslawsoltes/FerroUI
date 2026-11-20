use super::selection_workflows::{accepted_report, require_model};
use super::{decode, node_id, schema as s, state};
use crate::DesignerSession;
use ferroui_designer_language::LineIndex;
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember, item_template_resources};
use serde_json::{Value, json};
use std::hash::{DefaultHasher, Hash, Hasher};

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
    let mut included = arguments.clone();
    included["key"] = s::string();
    let required = ["node", "property", "expected_model"];
    vec![
        s::tool(
            QUERY,
            "Read an authored control or column's template slots, source ranges, local and included definitions, and assignable resources. Use the returned model for item template actions.",
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
                s::action("assign_included_resource", included, &["node", "property", "expected_model", "key"], true),
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
        let slot = session.item_template_slot(node, property)?;
        if let Some(target) = session.included_item_template_preview(&slot)? {
            return Ok(json!({"document":state(session), "openDefinition":target}));
        }
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
    let (resources, included_resources) = if slots.iter().any(|slot| slot.reason.is_none()) {
        (
            item_template_resources(document, node).map_err(|error| error.to_string())?,
            session.included_template_resources(node)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    let index = LineIndex::new(document.source());
    let slots = slots
        .iter()
        .map(|slot| {
            let copy = session.inline_template_copy_offer(slot);
            let copy_stamp = copy.as_ref().and_then(|copy| copy.as_ref().ok()).map(|edits| {
                let mut hash = DefaultHasher::new();
                json!(edits).to_string().hash(&mut hash);
                format!("{:016x}", hash.finish())
            });
            Ok(json!({
                "property":slot.property, "value":slot.value, "reason":slot.reason,
                "range":slot.span.and_then(|span| index.range(span).ok()),
                "includedDefinition":session.assigned_included_template(slot)?,
                "canCopyInline":copy_stamp.is_some(),
                "copyStamp":copy_stamp,
                "copyReason":copy.and_then(Result::err),
                "canReset":slot.reason.is_none() && slot.can_reset
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(accepted_report(
        session,
        QUERY,
        json!({"node":node,"slots":slots,"resources":resources,"includedResources":included_resources}),
    ))
}
