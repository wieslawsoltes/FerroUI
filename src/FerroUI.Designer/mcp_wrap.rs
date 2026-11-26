//! The same guarded wrapping plan powers Studio, MCP and source-owning editor clients.
use super::selection_workflows::{accepted_report, require_model};
use super::{ensure_revision, schema as s, state, DesignerMcp};
use crate::selection_wrap::{PreparedWrap, WrapContext};
use crate::{DesignerSession, WrapKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_rsx::RsxProjection;
use ferroui_mcp::UiToolProvider;
use serde_json::{json, Value};

const QUERY: &str = "designer_wrap_selection";
const EDIT: &str = "designer_wrap_selection_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let container = json!({"enum": WrapKind::ALL});
    [
        s::tool(QUERY, "List supported wrapper containers for the authored selection. Supply container to read the exact source plan and model. Wrapping uses the source order of consecutive siblings.",
            s::object(json!({"container":container}), &[]), false),
        s::tool(EDIT, "Apply the reviewed wrap plan in one undo step and select the new container. Pass the same container, expected_model and current source revision.",
            s::object(json!({"container":container,"expected_model":s::string(),"expected_revision":s::revision()}),
                &["container","expected_model","expected_revision"]), true),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == QUERY {
        return query(session, args, None);
    }
    let prepared = checked(session, args, None)?;
    let wrapper = session.commit_wrap(prepared)?;
    let mut result = state(session);
    result["wrapper"] = json!(wrapper);
    Ok(result)
}

fn query(
    session: &DesignerSession,
    args: &Value,
    projection: Option<RsxProjection<'_>>,
) -> Result<Value, String> {
    let context = WrapContext::new(session, projection)?;
    if let Some(container) = args.get("container") {
        let kind = serde_json::from_value(container.clone()).map_err(|error| error.to_string())?;
        return Ok(report(session, &context.prepare(kind)?));
    }
    let choices = WrapKind::ALL.map(|kind| {
        json!({
            "container":kind, "label":kind.label(), "detail":kind.detail(),
            "reason":context.prepare(kind).err(),
        })
    });
    Ok(
        json!({"choices":choices,"selection":session.selection(),"revision":session.document().revision()}),
    )
}

fn checked(
    session: &DesignerSession,
    args: &Value,
    projection: Option<RsxProjection<'_>>,
) -> Result<PreparedWrap, String> {
    ensure_revision(session, args)?;
    session
        .ensure_writable()
        .map_err(|error| error.to_string())?;
    let kind =
        serde_json::from_value(args["container"].clone()).map_err(|error| error.to_string())?;
    let prepared = WrapContext::new(session, projection)?.prepare(kind)?;
    require_model(args, &report(session, &prepared))?;
    Ok(prepared)
}

fn report(session: &DesignerSession, plan: &PreparedWrap) -> Value {
    accepted_report(
        session,
        "wrap_selection",
        json!({
            "layout":plan.layout,
            "wrapper":plan.wrapper,
            "patches":plan.change.patches(),
        }),
    )
}

impl DesignerMcp {
    /// Query wrapping against the accepted native layout and its optional editor Rust source.
    pub fn wrap_query(
        &self,
        args: &Value,
        projection: Option<RsxProjection<'_>>,
    ) -> Result<Value, String> {
        self.validate(QUERY, args)?;
        let owner = self.session()?;
        let session = owner
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        query(&session, args, projection)
    }

    /// Validate a reviewed wrap for a host that owns its own source buffer and undo history.
    pub fn wrap_source_plan(
        &self,
        args: &Value,
        projection: Option<RsxProjection<'_>>,
    ) -> Result<(Vec<TextPatch>, SourceSpan), String> {
        let prepared = self.prepare_wrap_edit(args, projection)?;
        Ok((prepared.change.patches().to_vec(), prepared.wrapper))
    }

    /// Return the checked syntax and authored source edits for an editor transaction.
    pub fn prepare_wrap_edit(
        &self,
        args: &Value,
        projection: Option<RsxProjection<'_>>,
    ) -> Result<PreparedWrap, String> {
        self.validate(EDIT, args)?;
        let owner = self.session()?;
        let session = owner
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        checked(&session, args, projection)
    }
}
