use super::{QUERY, accepted_target_report, decode, node_id, require_model, state};
use crate::{DesignerSession, DesignerWorkspace, mcp::ensure_revision};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_rsx::{RsxProjection, RsxSourceEdit, RsxSourceLocation};
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember};
use serde_json::{Value, json};
use std::rc::Rc;

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

pub(crate) fn call_workspace(
    workspace: &Rc<DesignerWorkspace>,
    args: &Value,
) -> Result<Value, String> {
    let session = workspace.session();
    let (owner, property) = {
        let session = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        let rust = session.rust.as_ref().ok_or("Open a Rust view")?;
        checked_member(&session, rust.source(), &rust.projection()?, args)?
    };
    if args["action"] == "preview" {
        workspace.open_rust_template(owner, property)?;
    } else {
        workspace.edit_workspace_rust_template(
            owner,
            property,
            decode_change(args)?,
            args["rust_type"].as_str(),
        )?;
        workspace.refresh(true);
    }
    let session = session.borrow();
    Ok(state(&session))
}

pub fn rust_template_report(
    session: &DesignerSession,
    _source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Value, String> {
    let owner = node_id(args, "node")?;
    let slots = session
        .rust_template_slots(projection, owner)
        .into_iter()
        .filter(|slot| args.get("property").is_none() || args["property"] == slot.property.name())
        .map(|slot| {
            json!({
                "property":slot.property,
                "value":{"kind": if slot.expression.is_some() { "expression" } else { "unset" }},
                "range":slot.expression.as_ref().and_then(|source| location_range(projection, source)),
                "sourceUri":slot.expression.as_ref().and_then(|source| source.uri.as_deref()),
                "rustBody":slot.body.as_ref().and_then(|body| location_range(projection, &body.location)),
                "rustBodyUri":slot.body.as_ref().and_then(|body| body.location.uri.as_deref()),
                "rustFactory":slot.body.as_ref().and_then(|body| body.factory.as_deref()),
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
    let range =
        location_range(projection, &body.location).ok_or("The Rust template range changed")?;
    let mut target = json!({"property":property,"range":range});
    if let Some(uri) = body.location.uri {
        target["uri"] = json!(uri);
    }
    Ok(target)
}

fn location_range(projection: &RsxProjection<'_>, location: &RsxSourceLocation) -> Option<Range> {
    LineIndex::new(projection.declaration_source(location)?)
        .range(location.span)
        .ok()
}

pub fn rust_template_source_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    let edit = rust_template_declaration_plan(session, source, projection, args)?;
    if edit.source.uri.is_some() {
        return Err("Apply the template edit to its owning Rust module".into());
    }
    Ok(edit.patches)
}

pub fn rust_template_declaration_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<RsxSourceEdit, String> {
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
        _ => Err("Create, open, or reset a mapped Rust template".into()),
    }
}
