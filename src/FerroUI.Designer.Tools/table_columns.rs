//! Source-owned column collections for the two native table controls.
mod fields;
mod source;

use crate::{Result, ToolError, error, require};
use ferroui_designer_core::{Document, Edit, NodeId, SourceSpan};
use serde::{Deserialize, Serialize};

pub use fields::{ColumnField, ColumnFieldKind};
use source::Collection;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TableKind {
    TableView,
    TreeDataGrid,
}

impl TableKind {
    fn property(self) -> &'static str {
        match self {
            Self::TableView => "Columns",
            Self::TreeDataGrid => "ColumnDefinitions",
        }
    }
    fn column_types(self) -> &'static [ColumnKind] {
        use ColumnKind::*;
        match self {
            Self::TableView => &[Table],
            Self::TreeDataGrid => &[Text, CheckBox, Template, RowHeader, Expander],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnKind {
    Table,
    Text,
    CheckBox,
    Template,
    RowHeader,
    Expander,
}

impl ColumnKind {
    pub fn element(self) -> &'static str {
        match self {
            Self::Table => "TableViewColumn",
            Self::Text => "TreeDataGridTextColumn",
            Self::CheckBox => "TreeDataGridCheckBoxColumn",
            Self::Template => "TreeDataGridTemplateColumn",
            Self::RowHeader => "TreeDataGridRowHeaderColumn",
            Self::Expander => "TreeDataGridHierarchicalExpanderColumn",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Table => "Column",
            Self::Text => "Text",
            Self::CheckBox => "Checkbox",
            Self::Template => "Template",
            Self::RowHeader => "Row header",
            Self::Expander => "Hierarchy",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumns {
    pub table: NodeId,
    pub name: Option<String>,
    pub kind: TableKind,
    pub columns: Vec<AuthoredColumn>,
    pub available: Vec<ColumnChoice>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ColumnChoice {
    pub kind: ColumnKind,
    pub label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredColumn {
    pub node: NodeId,
    pub kind: ColumnKind,
    pub label: String,
    pub source: SourceSpan,
    pub fields: Vec<ColumnField>,
    pub children: Vec<AuthoredColumn>,
    pub can_duplicate: bool,
}

/// Indices are zero-based. Move indices refer to the list after removing the moving column.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ColumnChange {
    Insert {
        index: usize,
        kind: ColumnKind,
        header: Option<String>,
    },
    Move {
        column: NodeId,
        index: usize,
    },
    Duplicate {
        column: NodeId,
    },
    Remove {
        column: NodeId,
    },
    Set {
        column: NodeId,
        field: String,
        value: String,
    },
    Clear {
        column: NodeId,
        field: String,
    },
}

impl ColumnChange {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Insert { .. } => "Add column",
            Self::Move { .. } => "Move column",
            Self::Duplicate { .. } => "Duplicate column",
            Self::Remove { .. } => "Remove column",
            Self::Set { .. } => "Edit column property",
            Self::Clear { .. } => "Reset column property",
        }
    }
}

pub fn table_columns(document: &Document, table: NodeId) -> Result<TableColumns> {
    Collection::new(document, table)?.model()
}

pub fn nearest_table(document: &Document, node: NodeId) -> Option<NodeId> {
    let ast = document.ast();
    std::iter::successors(ast.node(node), |node| ast.node(node.parent?))
        .find(|node| source::table_kind(node).is_some())
        .map(|node| node.id)
}

pub fn plan_column_change(
    document: &Document,
    table: NodeId,
    change: &ColumnChange,
) -> Result<Vec<Edit>> {
    let collection = Collection::new(document, table)?;
    let model = collection.model()?;
    if let Some(reason) = model.reason {
        return Err(ToolError(reason));
    }
    let edits = match change {
        ColumnChange::Insert {
            index,
            kind,
            header,
        } => {
            require(
                model.available.iter().any(|entry| entry.kind == *kind),
                "Choose an available column type",
            )?;
            let fragment = collection.fragment(*kind, header.as_deref())?;
            vec![collection.insert(*index, fragment)?]
        }
        ColumnChange::Move { column, index } => collection.move_column(*column, *index)?,
        ColumnChange::Duplicate { column } => {
            let (index, node) = collection.column(*column)?;
            require(
                model.columns[index].can_duplicate,
                "A table can contain one hierarchy column. For named elements, duplicate in source with distinct names.",
            )?;
            vec![collection.insert(index + 1, document.source()[node.span.range()].to_owned())?]
        }
        ColumnChange::Remove { column } => {
            collection.column(*column)?;
            vec![Edit::RemoveNode { node: *column }]
        }
        ColumnChange::Set {
            column,
            field,
            value,
        } => fields::plan(document, &model.columns, *column, field, Some(value))?,
        ColumnChange::Clear { column, field } => {
            fields::plan(document, &model.columns, *column, field, None)?
        }
    };
    if !edits.is_empty() {
        let mut candidate = document.snapshot_without_history();
        candidate
            .apply(change.label(), edits.clone())
            .map_err(error)?;
    }
    Ok(edits)
}
