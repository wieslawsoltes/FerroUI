use super::{QUERY, accepted_target_report, decode, node_id, require_model, state};
use crate::{DesignerSession, mcp::ensure_revision};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::RsxProjection;
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember};
use serde_json::{Value, json};

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let rust = session.rust.as_ref().ok_or("Open a Rust view")?;
    let projection = rust.projection()?;
    if name == QUERY {
        return rust_template_report(session, rust.source(), &projection, args);
    }
    let (owner, property) = checked_member(session, rust.source(), &projection, args)?;
    if args["action"] == "preview" {
        session.preview_rust_template(owner, property)?;
    } else {
        let change = decode_change(args)?;
        session.edit_rust_template(owner, property, change, args["rust_type"].as_str())?;
    }
    Ok(state(session))
}

pub fn rust_template_report(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Value, String> {
    let owner = node_id(args, "node")?;
    let index = LineIndex::new(source);
    let slots = session
        .rust_template_slots(projection, owner)
        .into_iter()
        .filter(|slot| args.get("property").is_none() || args["property"] == slot.property.name())
        .map(|slot| {
            json!({
                "property":slot.property,
                "value":{"kind": if slot.expression.is_some() { "expression" } else { "unset" }},
                "range":slot.expression.and_then(|span| index.range(span).ok()),
                "rustBody":slot.body.and_then(|span| index.range(span).ok()),
                "reason":slot.reason, "canCreate":slot.can_create, "canReset":slot.can_reset
            })
        })
        .collect::<Vec<_>>();
    Ok(accepted_target_report(
        session,
        QUERY,
        json!({
            "node":owner,"slots":slots,"resources":[],"includedResources":[]
        }),
    ))
}

pub fn rust_template_navigation(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Value, String> {
    let (owner, property) = checked_member(session, source, projection, args)?;
    let body = session.rust_template_body(projection, owner, property)?;
    let range = LineIndex::new(source)
        .range(body)
        .map_err(|error| error.to_string())?;
    Ok(json!({"property":property,"range":range}))
}

pub fn rust_template_source_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    let (owner, property) = checked_member(session, source, projection, args)?;
    session.rust_template_plan(
        projection,
        owner,
        property,
        &decode_change(args)?,
        args["rust_type"].as_str(),
    )
}

fn checked_member(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<(u64, ItemTemplateMember), String> {
    ensure_revision(session, args)?;
    // The query filter is reserved for XAML control templates. Resolve the
    // selected Rust property against the same complete slot model the UI read.
    let owner = node_id(args, "node")?;
    require_model(
        args,
        &rust_template_report(session, source, projection, &json!({"node":owner}))?,
    )?;
    Ok((owner, decode(args, "property")?))
}

fn decode_change(args: &Value) -> Result<ItemTemplateChange, String> {
    match args["action"].as_str() {
        Some("create_inline") => Ok(ItemTemplateChange::CreateInline),
        Some("reset") => Ok(ItemTemplateChange::Reset),
        _ => Err("Create, open, or reset a mapped inline Rust template".into()),
    }
}
