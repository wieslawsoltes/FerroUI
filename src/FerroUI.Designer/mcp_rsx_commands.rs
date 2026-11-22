use super::{
    decode, ensure_revision, node_id, schema as s,
    selection_workflows::{accepted_target_report, require_model},
    state,
};
use crate::{DesignerSession, DesignerWorkspace};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxCommandValue, RsxProjection};
use serde_json::{json, Value};
use std::rc::Rc;

pub const RSX_COMMANDS: &str = "designer_rsx_commands";
pub const RSX_COMMAND_EDIT: &str = "designer_rsx_command_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, RSX_COMMANDS | RSX_COMMAND_EDIT)
}

pub(super) fn tools() -> [Value; 2] {
    let selection = json!({"oneOf":[
        s::object(json!({"kind":{"enum":["keep","clear"]}}), &["kind"]),
        s::object(json!({"kind":{"const":"set"},"expression":s::string()}), &["kind","expression"])
    ]});
    let change = s::object(
        json!({"property":s::string(),"command":selection,"parameter":selection}),
        &["property", "command", "parameter"],
    );
    [
        s::tool(
            RSX_COMMANDS,
            "Read source-backed command slots for an authored Rust RSX control. Returns exact UTF-16 ranges, registered types, editable values and the accepted model token. Rust expressions execute in the application.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::tool(
            RSX_COMMAND_EDIT,
            "Keep, clear or set the Rust command and its parameter in one undo transaction. Use the model from designer_rsx_commands. Only mapped builder fields in the accepted RSX invocation can be changed.",
            s::object(
                json!({"node":s::id(),"change":change,"expected_revision":s::revision(),"expected_model":s::string()}),
                &["node", "change", "expected_revision", "expected_model"],
            ),
            true,
        ),
    ]
}

pub fn rust_command_report(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Value, String> {
    let owner = node_id(args, "node")?;
    let index = LineIndex::new(source);
    let field = |value: RsxCommandValue| {
        let range = value.expression.and_then(|span| index.range(span).ok());
        let authored = value.expression.and_then(|span| source.get(span.range()));
        json!({"property":value.property, "typeName":value.type_name, "editable":value.editable,
            "range":range,"authored":authored})
    };
    let slots: Vec<_> = session
        .rust_command_slots(projection, owner)
        .into_iter()
        .map(|slot| {
            json!({"property":slot.binding.property.clone(),"binding":field(slot.binding),
                "parameter":slot.parameter.map(field)})
        })
        .collect();
    Ok(accepted_target_report(
        session,
        RSX_COMMANDS,
        json!({"node":owner,"slots":slots}),
    ))
}

pub fn rust_command_source_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    ensure_revision(session, args)?;
    require_model(
        args,
        &rust_command_report(session, source, projection, args)?,
    )?;
    session.rust_command_plan(projection, node_id(args, "node")?, &decode(args, "change")?)
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let rust = session.rust.as_ref().ok_or("Open an authored RSX view")?;
    let projection = rust.projection()?;
    if name == RSX_COMMANDS {
        return rust_command_report(session, rust.source(), &projection, args);
    }
    let plan = rust_command_source_plan(session, rust.source(), &projection, args)?;
    session.apply_rust_command(plan)?;
    Ok(state(session))
}

pub(super) fn call_workspace(
    workspace: &Rc<DesignerWorkspace>,
    args: &Value,
) -> Result<Value, String> {
    let (plan, _) = workspace.with_current_rust_sources(|session, projection| {
        rust_command_source_plan(session, session.authored_source(), projection, args)
    })?;
    workspace.session().borrow_mut().apply_rust_command(plan)?;
    workspace.refresh(true);
    Ok(state(&workspace.session().borrow()))
}
