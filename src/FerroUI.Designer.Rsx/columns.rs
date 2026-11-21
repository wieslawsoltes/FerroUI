//! Source-owned columns from the typed FerroUI View builders.
mod array;
mod fields;
#[cfg(test)]
mod module_tests;
mod row;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tree_tests;

use self::{array::ColumnArray, row::Column};
use crate::{
    RsxProjection, RsxSourceEdit, RsxSourceLocation,
    animation::syntax::{path, span},
    item_templates::TemplateSource,
    local_rust::ReturnedBuilder,
    registered::Class,
    source_edits::insert_field,
};
use ferroui_designer_core::{Document, NodeId, SourceSpan, TextPatch};
use ferroui_designer_tools::{
    ColumnChange, ColumnKind, ItemTemplateMember, TableColumns, TableKind,
};
pub(crate) use row::ColumnPosition;
use std::fmt::Write;
use syn::Expr;

pub(crate) enum ColumnSource {
    Missing,
    Computed(SourceSpan),
    Mapped(MappedColumns),
}

pub(crate) struct MappedColumns {
    declaration: RsxColumnDeclaration,
    kind: TableKind,
    array: ColumnArray,
    rows: Vec<Column>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RsxColumnDeclaration {
    pub location: RsxSourceLocation,
    pub factory: Option<String>,
}

pub(crate) fn table_kind(class: &Class) -> Option<TableKind> {
    match class {
        Class::Builtin(name) if name == "TableView" => Some(TableKind::TableView),
        Class::Builtin(name) if name == "TreeDataGrid" => Some(TableKind::TreeDataGrid),
        _ => None,
    }
}

pub(crate) fn collection_member(kind: TableKind) -> (&'static str, &'static str) {
    match kind {
        TableKind::TableView => ("columns", "Columns"),
        TableKind::TreeDataGrid => ("column_definitions", "ColumnDefinitions"),
    }
}

fn builder(kind: TableKind) -> &'static str {
    match kind {
        TableKind::TableView => "table_columns",
        TableKind::TreeDataGrid => "tree_data_grid_columns",
    }
}

impl ColumnSource {
    pub(crate) fn read(
        &mut self,
        kind: TableKind,
        expression: &Expr,
        returned: Option<ReturnedBuilder<'_>>,
    ) -> bool {
        if matches!(self, Self::Computed(_)) {
            return false;
        }
        let parsed = if matches!(self, Self::Missing) {
            MappedColumns::read(kind, expression, None, &path).or_else(|| {
                let returned = returned?;
                MappedColumns::read(kind, returned.expression, Some(&returned), &|expression| {
                    returned.path(expression)
                })
            })
        } else {
            None
        };
        *self = parsed.map_or_else(|| Self::Computed(span(expression)), Self::Mapped);
        match self {
            Self::Mapped(columns) => columns.rows.iter().all(Column::projects_values),
            _ => false,
        }
    }

    fn declaration(&self) -> Option<RsxColumnDeclaration> {
        match self {
            Self::Missing => None,
            Self::Computed(span) => Some(RsxColumnDeclaration {
                location: RsxSourceLocation::local(*span),
                factory: None,
            }),
            Self::Mapped(columns) => Some(columns.declaration.clone()),
        }
    }

    pub(crate) fn projected(&self) -> bool {
        matches!(self, Self::Mapped(_))
    }

    pub(crate) fn render(&self, output: &mut String, padding: &str) {
        let Self::Mapped(columns) = self else {
            return;
        };
        if columns.rows.is_empty() {
            return;
        }
        let wrapper = match columns.kind {
            TableKind::TableView => "TableView.Columns",
            TableKind::TreeDataGrid => {
                // An empty source realizes the real headers without evaluating Rust rows.
                let _ = write!(
                    output,
                    "\n{padding}    <TreeDataGrid.ItemsSource><collections:ArrayList xmlns:collections=\"using:System.Collections\" /></TreeDataGrid.ItemsSource>"
                );
                "TreeDataGrid.ColumnDefinitions"
            }
        };
        let _ = write!(output, "\n{padding}    <{wrapper}>");
        let indent = format!("{padding}        ");
        for column in &columns.rows {
            column.render(output, &indent);
        }
        let _ = write!(output, "\n{padding}    </{wrapper}>");
    }

    pub(crate) fn matches_layout(
        &self,
        document: &Document,
        table: NodeId,
    ) -> Result<bool, String> {
        let rows = match self {
            Self::Mapped(columns) => columns.rows.as_slice(),
            _ => &[],
        };
        let model = ferroui_designer_tools::table_columns(document, table)
            .map_err(|error| error.to_string())?;
        Ok(matches_model(rows, &model))
    }
}

impl MappedColumns {
    fn read(
        kind: TableKind,
        expression: &Expr,
        returned: Option<&ReturnedBuilder<'_>>,
        resolve: &dyn Fn(&Expr) -> Option<String>,
    ) -> Option<Self> {
        let Expr::Call(call) = expression else {
            return None;
        };
        let qualified = resolve(&call.func)?;
        let name = qualified.strip_prefix("ferroui_view::")?;
        let name = name.strip_prefix("prelude::").unwrap_or(name);
        if name != builder(kind) || call.args.len() != 1 || !call.attrs.is_empty() {
            return None;
        }
        let Expr::Array(array) = call.args.first()? else {
            return None;
        };
        if array.elems.len() > 256 || !array.attrs.is_empty() {
            return None;
        }
        let rows = array
            .elems
            .iter()
            .map(|column| Column::read(column, kind, resolve))
            .collect::<Option<Vec<_>>>()?;
        if rows
            .iter()
            .filter(|column| column.kind == ColumnKind::Expander)
            .count()
            > 1
        {
            return None;
        }
        Some(Self {
            declaration: RsxColumnDeclaration {
                location: RsxSourceLocation {
                    uri: returned.and_then(ReturnedBuilder::uri).map(str::to_owned),
                    span: span(expression),
                },
                factory: returned.map(|definition| definition.name.to_owned()),
            },
            kind,
            array: ColumnArray::read(array),
            rows,
        })
    }
}

fn matches_model(rows: &[Column], model: &TableColumns) -> bool {
    model.reason.is_none()
        && model.columns.len() == rows.len()
        && rows
            .iter()
            .zip(&model.columns)
            .all(|(source, projected)| source.matches(projected))
}

impl RsxProjection<'_> {
    /// Attach original Rust locations and writable values to the native projected table model.
    pub fn map_column_model(
        &self,
        document: &Document,
        model: &mut TableColumns,
    ) -> Option<SourceSpan> {
        let source = self.map_column_declaration(document, model)?;
        if source.location.uri.is_some() {
            model.columns.clear();
            model.available.clear();
            model.reason = Some("Open the supplying Rust module to edit its columns.".into());
            return None;
        }
        Some(source.location.span)
    }

    /// Map writable fields to their declaration, which can be in a different Cargo module.
    pub fn map_column_declaration(
        &self,
        document: &Document,
        model: &mut TableColumns,
    ) -> Option<RsxColumnDeclaration> {
        let source = self
            .control_index(document, model.table)
            .and_then(|index| self.nodes[index].columns.as_ref());
        match source {
            Some(ColumnSource::Missing) if model.columns.is_empty() => {}
            Some(ColumnSource::Mapped(columns)) if matches_model(&columns.rows, model) => {
                for (row, column) in columns.rows.iter().zip(&mut model.columns) {
                    row.describe(column, true);
                    column.can_duplicate &= columns.rows.len() < 256;
                }
                if columns.rows.len() >= 256 {
                    model.available.clear();
                }
            }
            _ => {
                model.columns.clear();
                model.available.clear();
                model.reason = Some(
                    "This column collection is defined by Rust. Open its source to edit it.".into(),
                );
            }
        }
        source.and_then(ColumnSource::declaration)
    }

    /// Reparse the draft before allowing either editor host to commit its original Rust patches.
    pub fn edit_columns(
        &self,
        document: &Document,
        model: &TableColumns,
        change: &ColumnChange,
    ) -> Result<Vec<TextPatch>, String> {
        let edit = self.plan_columns(document, model, change)?;
        if edit.source.uri.is_some() {
            return Err("Apply the column edit to its owning Rust module".into());
        }
        Ok(edit.patches)
    }

    /// Plan a checked edit to the active buffer or the companion module supplying the columns.
    pub fn plan_columns(
        &self,
        document: &Document,
        model: &TableColumns,
        change: &ColumnChange,
    ) -> Result<RsxSourceEdit, String> {
        if let Some(reason) = &model.reason {
            return Err(reason.clone());
        }
        let index = self
            .control_index(document, model.table)
            .ok_or("Select an authored RSX table")?;
        let owner = &self.nodes[index];
        let kind = table_kind(&owner.class)
            .filter(|kind| *kind == model.kind)
            .ok_or("The selected table changed")?;
        let edit = match owner.columns.as_ref() {
            Some(ColumnSource::Missing) => match change {
                ColumnChange::Insert {
                    index: 0,
                    kind: column,
                    header,
                } if model.available.iter().any(|choice| choice.kind == *column) => {
                    let column = row::fragment(*column, header.as_deref())?;
                    let (field, _) = collection_member(kind);
                    let builder = builder(kind);
                    let patches = vec![insert_field(
                        self.source,
                        owner.open,
                        &format!("{field}: ::ferroui_view::{builder}([{column}])"),
                    )];
                    RsxSourceEdit {
                        source: RsxSourceLocation::local(owner.span),
                        patches,
                    }
                }
                _ => return Err("Choose Add Column to create the authored Rust collection".into()),
            },
            Some(ColumnSource::Mapped(columns)) => {
                let source = columns.declaration.location.clone();
                let before = self
                    .declaration_source(&source)
                    .ok_or("The column declaration changed")?;
                let patches = Self::column_patches(before, model, columns, change)?;
                RsxSourceEdit { source, patches }
            }
            _ => return Err("Edit the Rust column collection in its source".into()),
        };
        if edit.patches.is_empty() {
            return Ok(edit);
        }
        let (candidate, caret, modules) = self.declaration_draft(&edit)?;
        let next = self
            .catalog
            .project_with_modules(&candidate, caret, modules.as_ref())?;
        if next.invocation.start != caret
            || !next
                .nodes
                .get(index)
                .and_then(|node| node.columns.as_ref())
                .is_some_and(|columns| {
                    matches!(columns, ColumnSource::Mapped(columns)
                    if columns.declaration.location.uri == edit.source.uri)
                })
        {
            return Err("The edit could not preserve the authored Rust column collection".into());
        }
        Document::new(next.xaml()).map_err(|error| error.to_string())?;
        Ok(edit)
    }

    fn column_patches(
        source: &str,
        model: &TableColumns,
        columns: &MappedColumns,
        change: &ColumnChange,
    ) -> Result<Vec<TextPatch>, String> {
        if columns.rows.len() != model.columns.len() {
            return Err("Refresh the native table and its authored columns before editing".into());
        }
        let index_of = |id| {
            model
                .columns
                .iter()
                .position(|column| column.node == id)
                .ok_or("The selected column changed")
        };
        match change {
            ColumnChange::Insert {
                index,
                kind,
                header,
            } if columns.rows.len() < 256
                && model.available.iter().any(|choice| choice.kind == *kind) =>
            {
                columns
                    .array
                    .insert(source, *index, &row::fragment(*kind, header.as_deref())?)
            }
            ColumnChange::Insert { .. } => {
                Err("Choose an available column type in a list with fewer than 256 entries".into())
            }
            ColumnChange::Move { column, index } => {
                columns
                    .array
                    .move_member(source, index_of(*column)?, *index)
            }
            ColumnChange::Remove { column } => Ok(columns.array.remove(index_of(*column)?)),
            ColumnChange::Duplicate { column } => {
                let index = index_of(*column)?;
                let row = &columns.rows[index];
                if !row.can_duplicate() || columns.rows.len() >= 256 {
                    return Err("Duplicate a column with mapped literals in a list with fewer than 256 entries".into());
                }
                columns
                    .array
                    .insert(source, index + 1, &source[row.source.range()])
            }
            ColumnChange::Set { column, field, .. } | ColumnChange::Clear { column, field } => {
                let (position, column) =
                    ColumnPosition::find(model, *column).ok_or("The selected column changed")?;
                let owner = position
                    .at(&columns.rows)
                    .filter(|row| row.kind == column.kind)
                    .ok_or("The selected column changed")?;
                let field = column
                    .fields
                    .iter()
                    .find(|entry| entry.name == field)
                    .ok_or("Choose a mapped column property")?;
                let value = match change {
                    ColumnChange::Set { value, .. } => Some(value.as_str()),
                    _ => None,
                };
                Ok(fields::change(
                    source,
                    owner.kind,
                    owner.fields.get(field.name),
                    owner.append_at,
                    field,
                    value,
                )?
                .into_iter()
                .collect())
            }
        }
    }

    pub(crate) fn control_index(&self, document: &Document, node: NodeId) -> Option<usize> {
        let owner = document.ast().node(node)?;
        self.preview_markup()
            .controls
            .binary_search(&owner.span.start)
            .ok()
    }

    pub(crate) fn template_column(
        &self,
        document: &Document,
        node: NodeId,
    ) -> Option<(usize, ColumnPosition, ColumnKind)> {
        if !matches!(
            document.ast().node(node)?.local_name(),
            "TableViewColumn" | "TreeDataGridTemplateColumn"
        ) {
            return None;
        }
        let table = ferroui_designer_tools::nearest_table(document, node)?;
        let index = self.control_index(document, table)?;
        let ColumnSource::Mapped(columns) = self.nodes[index].columns.as_ref()? else {
            return None;
        };
        if columns.declaration.factory.is_some() {
            return None;
        }
        let model = ferroui_designer_tools::table_columns(document, table).ok()?;
        if model.reason.is_some() || columns.rows.len() != model.columns.len() {
            return None;
        }
        let (position, column) = ColumnPosition::find(&model, node)?;
        (position.at(&columns.rows)?.kind == column.kind).then_some((index, position, column.kind))
    }

    pub(crate) fn column_template(
        &self,
        table: usize,
        column: ColumnPosition,
        property: ItemTemplateMember,
    ) -> Option<(usize, Option<TemplateSource>, bool)> {
        let ColumnSource::Mapped(columns) = self.nodes.get(table)?.columns.as_ref()? else {
            return None;
        };
        let row = column.at(&columns.rows)?;
        let key = match property {
            ItemTemplateMember::CellTemplate => "CellTemplateResourceKey",
            ItemTemplateMember::CellEditingTemplate => "CellEditingTemplateResourceKey",
            _ => return None,
        };
        Some((
            row.append_at,
            row.fields
                .get(property.name())
                .and_then(|field| field.template),
            !row.fields.contains_key(key) && !row.fields.contains_key("CellTemplateResourceKey"),
        ))
    }
}
