use super::selection_workflows::{accepted_target_report, require_model};
use super::{decode, node_id, schema as s, state};
use crate::DesignerSession;
use ferroui_designer_language::LineIndex;
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember};
use serde_json::{Value, json};
use std::hash::{DefaultHasher, Hash, Hasher};

#[path = "mcp_rust_templates.rs"]
mod rust;
pub use rust::{rust_template_declaration_plan, rust_template_navigation, rust_template_report, rust_template_source_plan};
pub(super) use rust::call_workspace as call_rust_workspace;

const QUERY: &str = "designer_item_templates";
pub(super) const ACTION: &str = "designer_item_template";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | ACTION)
}

pub(super) fn tools() -> Vec<Value> {
    let properties = json!({"enum":ItemTemplateMember::AUTHORABLE});
    let arguments = json!({"node":s::id(),"property":properties,"expected_model":s::string()});
    let mut assign = arguments.clone();
    assign["resource"] = s::id();
    let mut included = arguments.clone();
    included["key"] = s::string();
    let mut create = arguments.clone();
    create["rust_type"] = json!({"type":"string","minLength":1,"maxLength":512});
    let required = ["node", "property", "expected_model"];
    vec![
        s::tool(
            QUERY,
            "Read an authored control or column's data template slots. Pass property Template for its control template slot. Use the returned model to open or change the authored template.",
            s::object(json!({"node":s::id(),"property":{"const":"Template"}}), &["node"]),
            false,
        ),
        s::actions(
            ACTION,
            "Open a native template preview or edit a source-owned XAML or mapped RSX template. Supply rust_type when creating an empty Rust slot. Resource assignment and copies apply to XAML. Each source edit is one undo step and checks the accepted model and revision.",
            vec![
                s::action("preview", arguments.clone(), &required, true),
                s::action("create_inline", create, &required, true),
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
    if session.uses_rust_source() {
        return rust::call(session, name, args);
    }
    let node = node_id(args, "node")?;
    let control = args["property"] == "Template";
    let report = report(session, node, control)?;
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

fn report(session: &DesignerSession, node: u64, control: bool) -> Result<Value, String> {
    let document = session.document();
    let member = if control { ItemTemplateMember::Template } else { ItemTemplateMember::ItemTemplate };
    let slots = if control { vec![session.item_template_slot(node, member)?] } else { session.item_template_slots(node)? };
    let (resources, included_resources) = if slots.iter().any(|slot| slot.reason.is_none()) {
        (
            session.local_template_resources(node, member)?,
            session.included_template_resources(node, member)?,
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
                "localDefinition":session.local_template_definition(slot),
                "includedDefinition":session.included_template_definition(slot)?,
                "canCopyInline":copy_stamp.is_some(),
                "copyStamp":copy_stamp,
                "copyReason":copy.and_then(Result::err),
                "canReset":slot.reason.is_none() && slot.can_reset
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(accepted_target_report(
        session,
        QUERY,
        json!({"node":node,"slots":slots,"resources":resources,"includedResources":included_resources}),
    ))
}
