//! Readable plans and guarded commits for selection-wide authoring. The native session
//! owns compatibility, layout semantics and history; automation supplies no raw edits.
use super::schema as s;
use super::{node_id, state, text};
use crate::{
    CopiedPropertyKind, DesignerSession, Edit, LayoutMatch, LayoutReset, PropertyPastePreview,
    PropertyTransferIssue,
};
use serde_json::{json, Value};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_selection_layout"
            | "designer_selection_layout_edit"
            | "designer_property_transfer"
            | "designer_property_transfer_edit"
    )
}

pub(super) fn tools() -> Vec<Value> {
    let operation = json!({"enum":[
        "reset_margin", "reset_alignment", "reset_width", "reset_height",
        "reset_size", "reset_all", "match_width", "match_height", "match_size"
    ]});
    let properties = s::array(s::string(), 64);
    vec![
        s::tool(
            "designer_selection_layout",
            "Plan one layout command for the current authored selection without changing source. Reset removes local values. Match uses the first selected control's measured logical dimensions and each destination's native layout adapter. Returns source edits and an ephemeral model token.",
            s::object(json!({"operation":operation}), &["operation"]),
            false,
        ),
        s::tool(
            "designer_selection_layout_edit",
            "Apply the exact operation reviewed with designer_selection_layout as one undo transaction. expected_model retires after selection, source, scope, presentation, permissions or relevant native layout changes. No animation keys are recorded.",
            s::object(
                json!({"operation":operation,"expected_revision":s::revision(),"expected_model":s::string()}),
                &["operation", "expected_revision", "expected_model"],
            ),
            true,
        ),
        s::tool(
            "designer_property_transfer",
            "Inspect authored appearance properties from source and plan their transfer to the current selection. Optional properties selects captured names; omitted means all captured values. Returns explicit compatible writes, skipped values and a model token. Identity, content, data and geometry are excluded. No source or selection changes occur.",
            s::object(json!({"source":s::id(),"properties":properties}), &["source"]),
            false,
        ),
        s::tool(
            "designer_property_transfer_edit",
            "Apply the appearance transfer reviewed with designer_property_transfer. Pass its source, properties, model and revision. Uses the same native metadata, resource identity and atomic non-animation transaction as the property-paste editor. Unsupported values remain listed in the read plan.",
            s::object(
                json!({"source":s::id(),"properties":properties,"expected_revision":s::revision(),"expected_model":s::string()}),
                &["source", "properties", "expected_revision", "expected_model"],
            ),
            true,
        ),
    ]
}

fn layout_plan(
    session: &DesignerSession,
    operation: &str,
) -> Result<(&'static str, Vec<Edit>), String> {
    let reset = match operation {
        "reset_margin" => Some(LayoutReset::Margin),
        "reset_alignment" => Some(LayoutReset::Alignment),
        "reset_width" => Some(LayoutReset::Width),
        "reset_height" => Some(LayoutReset::Height),
        "reset_size" => Some(LayoutReset::Size),
        "reset_all" => Some(LayoutReset::All),
        _ => None,
    };
    if let Some(reset) = reset {
        return Ok((
            reset.label(),
            session.plan_layout_reset(session.selection(), reset)?,
        ));
    }
    let mode = match operation {
        "match_width" => LayoutMatch::Width,
        "match_height" => LayoutMatch::Height,
        "match_size" => LayoutMatch::Size,
        _ => return Err("Unknown selection layout operation".into()),
    };
    Ok((mode.label(), session.plan_match_selection_size(mode)?))
}

fn issues(values: &[PropertyTransferIssue]) -> Vec<Value> {
    values
        .iter()
        .map(|issue| {
            json!({
            "target":issue.target, "target_label":issue.target_label,
            "property":issue.property, "reason":issue.reason
            })
        })
        .collect()
}

fn property_plan(
    session: &DesignerSession,
    args: &Value,
) -> Result<(Value, PropertyPastePreview), String> {
    let source = node_id(args, "source")?;
    let clipboard = session.copy_properties_from(source)?;
    let names = args["properties"].as_array().map_or_else(
        || clipboard.property_names(),
        |values| {
            values
                .iter()
                .map(|value| value.as_str().expect("validated property name").to_owned())
                .collect()
        },
    );
    let preview = session.prepare_property_paste(&clipboard, &names)?;
    let report = json!({
        "source":source,
        "source_label":clipboard.source_label(),
        "properties":names,
        "entries":clipboard.entries().iter().map(|entry| json!({
            "name":entry.name, "category":entry.category, "summary":entry.summary,
            "kind":match entry.kind {
                CopiedPropertyKind::Literal => "literal",
                CopiedPropertyKind::Object => "object",
                CopiedPropertyKind::Resource => "resource",
            }
        })).collect::<Vec<_>>(),
        "capture_skipped":issues(clipboard.skipped()),
        "changes":preview.changes().iter().map(|change| json!({
            "target":change.target, "target_label":change.target_label,
            "property":change.property, "changed":change.changed
        })).collect::<Vec<_>>(),
        "paste_skipped":issues(preview.skipped()),
        "edits":preview.edits(),
        "native_model":format!("{:016x}", preview.native_identity_token()),
        "change_count":preview.change_count()
    });
    Ok((
        accepted_report(session, "property_transfer", report),
        preview,
    ))
}

fn accepted_report(session: &DesignerSession, operation: &str, mut report: Value) -> Value {
    report["revision"] = json!(session.document().revision());
    report["selection"] = json!(session.selection());
    report["scope_epoch"] = json!(session.definition_scope_epoch());
    report["layer_epoch"] = json!(session.layer_view_epoch());
    // The token is an optimistic-concurrency check, not a permission or credential.
    // Hash identity as well as revision so retained models cannot cross equal-source
    // allocations. Layout reports include the recomputed measured adapter edits.
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    operation.hash(&mut hash);
    session.document_uri().hash(&mut hash);
    session.generation().hash(&mut hash);
    Arc::as_ptr(&session.document().ast_snapshot()).hash(&mut hash);
    report.to_string().hash(&mut hash);
    report["model"] = json!(format!("{:016x}", hash.finish()));
    report
}

fn require_model(args: &Value, report: &Value) -> Result<(), String> {
    if args["expected_model"] == report["model"] {
        Ok(())
    } else {
        Err("The accepted authoring model changed; read the current plan before applying it".into())
    }
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "designer_selection_layout" | "designer_selection_layout_edit" => {
            if name == "designer_selection_layout_edit" {
                // A reviewed no-op is still a write command and must not bypass roles.
                session
                    .ensure_writable()
                    .map_err(|error| error.to_string())?;
            }
            let operation = text(args, "operation")?;
            let (label, edits) = layout_plan(session, &operation)?;
            let report = accepted_report(
                session,
                "selection_layout",
                json!({
                    "operation":operation, "label":label, "edits":edits,
                    "edit_count":edits.len()
                }),
            );
            if name == "designer_selection_layout" {
                return Ok(report);
            }
            require_model(args, &report)?;
            if !edits.is_empty() {
                session
                    .apply(label, edits)
                    .map_err(|error| error.to_string())?;
            }
            Ok(state(session))
        }
        "designer_property_transfer" | "designer_property_transfer_edit" => {
            let (report, preview) = property_plan(session, args)?;
            if name == "designer_property_transfer" {
                return Ok(report);
            }
            require_model(args, &report)?;
            session.commit_property_paste(&preview)?;
            Ok(state(session))
        }
        _ => Err("Unknown selection authoring tool".into()),
    }
}

#[cfg(test)]
#[path = "mcp_selection_workflows_tests.rs"]
mod tests;
