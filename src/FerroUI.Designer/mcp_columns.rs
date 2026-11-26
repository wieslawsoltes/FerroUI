use super::selection_workflows::{accepted_report, require_model};
use super::{ensure_revision, node_id, schema as s, state};
use crate::{DesignerSession, DesignerWorkspace};
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxProjection, RsxSourceEdit};
use ferroui_designer_tools::{AuthoredColumn, ColumnChange, ColumnKind};
use serde_json::{Value, json};
use std::rc::Rc;

const QUERY: &str = "designer_columns";
pub(super) const EDIT: &str = "designer_columns_edit";

pub(super) fn handles(name: &str) -> bool {
    matches!(name, QUERY | EDIT)
}

pub(super) fn tools() -> Vec<Value> {
    let index = json!({"type":"integer","minimum":0});
    let kinds = json!({"enum":[ColumnKind::Table,ColumnKind::Text,ColumnKind::CheckBox,ColumnKind::Template,ColumnKind::RowHeader,ColumnKind::Expander]});
    vec![
        s::tool(
            QUERY,
            "Read authored XAML or mapped RSX columns for TableView and TreeDataGrid, with typed fields, source ranges and the current edit model. Pass the table or an authored descendant.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::actions(
            EDIT,
            "Add, move, duplicate, remove, or edit one source-owned column using the current model. Move indices refer to the resulting order. Typed values, object values and runtime-owned collections are checked.",
            vec![
                s::action(
                    "insert",
                    json!({"table":s::id(),"expected_model":s::string(),"index":index,"kind":kinds,"header":s::string()}),
                    &["table", "expected_model", "index", "kind"],
                    true,
                ),
                s::action(
                    "move",
                    json!({"table":s::id(),"expected_model":s::string(),"column":s::id(),"index":index}),
                    &["table", "expected_model", "column", "index"],
                    true,
                ),
                s::action(
                    "duplicate",
                    json!({"table":s::id(),"expected_model":s::string(),"column":s::id()}),
                    &["table", "expected_model", "column"],
                    true,
                ),
                s::action(
                    "remove",
                    json!({"table":s::id(),"expected_model":s::string(),"column":s::id()}),
                    &["table", "expected_model", "column"],
                    true,
                ),
                s::action(
                    "set",
                    json!({"table":s::id(),"expected_model":s::string(),"column":s::id(),"field":s::string(),"value":s::string()}),
                    &["table", "expected_model", "column", "field", "value"],
                    true,
                ),
                s::action(
                    "clear",
                    json!({"table":s::id(),"expected_model":s::string(),"column":s::id(),"field":s::string()}),
                    &["table", "expected_model", "column", "field"],
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
    if name == QUERY {
        return columns_report(session, node_id(args, "node")?, None);
    }
    ensure_revision(session, args)?;
    let table = node_id(args, "table")?;
    require_model(args, &columns_report(session, table, None)?)?;
    let change: ColumnChange =
        serde_json::from_value(args.clone()).map_err(|error| error.to_string())?;
    session.edit_authored_columns(table, change)?;
    Ok(state(session))
}

pub(super) fn call_workspace(
    workspace: &Rc<DesignerWorkspace>,
    args: &Value,
) -> Result<Value, String> {
    let table = node_id(args, "table")?;
    let session = workspace.session();
    {
        let session = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        ensure_revision(&session, args)?;
        require_model(args, &columns_report(&session, table, None)?)?;
    }
    let change = serde_json::from_value(args.clone()).map_err(|error| error.to_string())?;
    workspace.edit_workspace_columns(table, change)?;
    workspace.refresh(true);
    let session = session.borrow();
    Ok(state(&session))
}

pub fn columns_report(
    session: &DesignerSession,
    node: u64,
    rust: Option<(&str, &RsxProjection<'_>)>,
) -> Result<Value, String> {
    let Some(columns) = session.column_authoring(node, rust.map(|(_, projection)| projection))?
    else {
        return Ok(json!({"table":null}));
    };
    let model = columns.model;
    // Keep byte ranges for native navigation and add UTF-16 ranges for editor clients.
    let modules = rust
        .and_then(|(_, projection)| projection.module_sources())
        .or_else(|| {
            session
                .rust
                .as_ref()?
                .module_sources()
                .map(|modules| modules.as_ref())
        });
    let uri = columns
        .source
        .as_ref()
        .and_then(|source| source.location.uri.as_deref());
    let source = match uri {
        Some(uri) => modules
            .and_then(|modules| modules.source(uri))
            .ok_or("The Rust column module changed")?
            .source
            .as_ref(),
        None => rust.map_or_else(|| session.authored_source(), |(source, _)| source),
    };
    let index = LineIndex::new(source);
    let mut report = serde_json::to_value(&model).map_err(|error| error.to_string())?;
    if let Some(source) = columns.source {
        report["sourceRange"] = json!(index.range(source.location.span).ok());
        if let Some(uri) = source.location.uri {
            report["sourceUri"] = json!(uri);
        }
        if let Some(factory) = source.factory {
            report["factory"] = json!(factory);
        }
    }
    if let Some(rows) = report["columns"].as_array_mut() {
        for (row, column) in rows.iter_mut().zip(&model.columns) {
            ranges(row, column, &index);
        }
    }
    Ok(accepted_report(session, QUERY, report))
}

pub fn rust_column_source_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<Vec<TextPatch>, String> {
    let plan = rust_column_declaration_plan(session, source, projection, args)?;
    if plan.source.uri.is_some() {
        return Err("Apply this edit to its owning Rust module".into());
    }
    Ok(plan.patches)
}

pub fn rust_column_declaration_plan(
    session: &DesignerSession,
    source: &str,
    projection: &RsxProjection<'_>,
    args: &Value,
) -> Result<RsxSourceEdit, String> {
    ensure_revision(session, args)?;
    let table = node_id(args, "table")?;
    require_model(
        args,
        &columns_report(session, table, Some((source, projection)))?,
    )?;
    let change = serde_json::from_value(args.clone()).map_err(|error| error.to_string())?;
    session.rust_column_plan(table, projection, &change)
}

fn ranges(row: &mut Value, column: &AuthoredColumn, index: &LineIndex) {
    row["range"] = json!(index.range(column.source).ok());
    if let Some(fields) = row["fields"].as_array_mut() {
        for (row, field) in fields.iter_mut().zip(&column.fields) {
            row["range"] = json!(field.source.and_then(|span| index.range(span).ok()));
        }
    }
    if let Some(children) = row["children"].as_array_mut() {
        for (row, column) in children.iter_mut().zip(&column.children) {
            ranges(row, column, index);
        }
    }
}
