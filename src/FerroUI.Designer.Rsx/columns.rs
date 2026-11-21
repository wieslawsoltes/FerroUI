//! Source-owned TableView columns from the typed FerroUI View builders.
mod array;
mod fields;
#[cfg(test)]
mod tests;

use self::{array::ColumnArray, fields::SourceField};
use crate::{
    RsxProjection,
    animation::syntax::{chain, path, span, view_argument},
    source_edits::insert_field,
    tree::escape_xml,
};
use ferroui_designer_core::{Document, NodeId, SourceSpan, TextPatch};
use ferroui_designer_tools::{AuthoredColumn, ColumnChange, ColumnKind, TableColumns};
use std::{collections::BTreeMap, fmt::Write};
use syn::Expr;

pub(crate) enum ColumnSource {
    Missing,
    Computed(SourceSpan),
    Inline(InlineColumns),
}

pub(crate) struct InlineColumns {
    source: SourceSpan,
    array: ColumnArray,
    rows: Vec<Column>,
}

struct Column {
    source: SourceSpan,
    fields: BTreeMap<&'static str, SourceField>,
}

impl ColumnSource {
    pub(crate) fn read(&mut self, expression: &Expr) -> bool {
        let parsed = if matches!(self, Self::Missing) {
            InlineColumns::read(expression)
        } else {
            None
        };
        *self = parsed.map_or_else(|| Self::Computed(span(expression)), Self::Inline);
        match self {
            Self::Inline(columns) => columns.rows.iter().all(Column::can_duplicate),
            _ => false,
        }
    }

    fn source(&self) -> Option<SourceSpan> {
        match self {
            Self::Missing => None,
            Self::Computed(span) => Some(*span),
            Self::Inline(columns) => Some(columns.source),
        }
    }

    pub(crate) fn projected(&self) -> bool {
        matches!(self, Self::Inline(_))
    }

    pub(crate) fn render(&self, output: &mut String, padding: &str) {
        let Self::Inline(columns) = self else {
            return;
        };
        if columns.rows.is_empty() {
            return;
        }
        let _ = write!(output, "\n{padding}    <TableView.Columns>");
        for column in &columns.rows {
            let _ = write!(output, "\n{padding}        <TableViewColumn");
            for (name, field) in &column.fields {
                if let Some(value) = field.render_value() {
                    let _ = write!(output, " {name}=\"{}\"", escape_xml(&value, Some('"')));
                }
            }
            output.push_str(" />");
        }
        let _ = write!(output, "\n{padding}    </TableView.Columns>");
    }

    pub(crate) fn matches_layout(
        &self,
        document: &Document,
        table: NodeId,
    ) -> Result<bool, String> {
        let rows = match self {
            Self::Inline(inline) => inline.rows.as_slice(),
            _ => &[],
        };
        let model = ferroui_designer_tools::table_columns(document, table)
            .map_err(|error| error.to_string())?;
        Ok(model.reason.is_none()
            && model.columns.len() == rows.len()
            && model.columns.iter().zip(rows).all(|(column, row)| {
                column.fields.iter().all(|field| {
                    let projected = row.fields.get(field.name).and_then(|source| {
                        if field.name == "Header" {
                            source.value.clone()
                        } else {
                            source.render_value()
                        }
                    });
                    field.value == projected
                })
            }))
    }
}

impl InlineColumns {
    fn read(expression: &Expr) -> Option<Self> {
        let Expr::Call(call) = expression else {
            return None;
        };
        let Expr::Array(array) = view_argument(call, "table_columns")? else {
            return None;
        };
        if array.elems.len() > 256 {
            return None;
        }
        let rows = array
            .elems
            .iter()
            .map(Column::read)
            .collect::<Option<_>>()?;
        Some(Self {
            source: span(expression),
            array: ColumnArray::read(array),
            rows,
        })
    }
}

impl Column {
    fn read(expression: &Expr) -> Option<Self> {
        let (factory, methods) = chain(expression)?;
        if !factory.args.is_empty()
            || !matches!(
                path(&factory.func)?.as_str(),
                "ferroui_view::table_column" | "ferroui_view::prelude::table_column"
            )
        {
            return None;
        }
        let mut fields = BTreeMap::new();
        for method in methods {
            let field = SourceField::read(method)?;
            if fields.insert(field.name(), field).is_some() {
                return None;
            }
        }
        Some(Self {
            source: span(expression),
            fields,
        })
    }

    fn can_duplicate(&self) -> bool {
        self.fields.values().all(|field| field.value.is_some())
    }

    fn describe(&self, column: &mut AuthoredColumn) {
        column.source = self.source;
        column.can_duplicate = self.can_duplicate();
        for field in &mut column.fields {
            if let Some(source) = self.fields.get(field.name) {
                source.describe(field);
            }
        }
    }
}

impl RsxProjection<'_> {
    /// Attach original Rust locations and writable values to the native projected table model.
    pub fn map_column_model(
        &self,
        document: &Document,
        model: &mut TableColumns,
    ) -> Option<SourceSpan> {
        let source = self
            .column_owner(document, model.table)
            .and_then(|index| self.nodes[index].columns.as_ref());
        match source {
            Some(ColumnSource::Missing) if model.columns.is_empty() => {}
            Some(ColumnSource::Inline(columns)) if columns.rows.len() == model.columns.len() => {
                for (row, column) in columns.rows.iter().zip(&mut model.columns) {
                    row.describe(column);
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
        source.and_then(ColumnSource::source)
    }

    /// Reparse the draft before allowing either editor host to commit its original Rust patches.
    pub fn edit_columns(
        &self,
        document: &Document,
        model: &TableColumns,
        change: &ColumnChange,
    ) -> Result<Vec<TextPatch>, String> {
        if let Some(reason) = &model.reason {
            return Err(reason.clone());
        }
        let index = self
            .column_owner(document, model.table)
            .ok_or("Select an authored RSX table")?;
        let owner = &self.nodes[index];
        let patches = match owner.columns.as_ref() {
            Some(ColumnSource::Missing) => match change {
                ColumnChange::Insert {
                    index: 0,
                    kind: ColumnKind::Table,
                    header,
                } => {
                    let column = column_fragment(header.as_deref())?;
                    vec![insert_field(
                        self.source,
                        owner.open,
                        &format!("columns: ::ferroui_view::table_columns([{column}])"),
                    )]
                }
                _ => return Err("Choose Add Column to create the authored Rust collection".into()),
            },
            Some(ColumnSource::Inline(columns)) => self.column_patches(model, columns, change)?,
            _ => return Err("Edit the Rust column collection in its source".into()),
        };
        if patches.is_empty() {
            return Ok(patches);
        }
        let candidate = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let next = self.catalog.project(&candidate, self.invocation.start)?;
        if next.invocation.start != self.invocation.start
            || !next
                .nodes
                .get(index)
                .and_then(|node| node.columns.as_ref())
                .is_some_and(ColumnSource::projected)
        {
            return Err("The edit could not preserve the authored Rust column collection".into());
        }
        Document::new(next.xaml()).map_err(|error| error.to_string())?;
        Ok(patches)
    }

    fn column_patches(
        &self,
        model: &TableColumns,
        columns: &InlineColumns,
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
                kind: ColumnKind::Table,
                header,
            } if columns.rows.len() < 256 => {
                columns
                    .array
                    .insert(self.source, *index, &column_fragment(header.as_deref())?)
            }
            ColumnChange::Insert { .. } => {
                Err("Choose a TableView column in a list with fewer than 256 entries".into())
            }
            ColumnChange::Move { column, index } => {
                columns
                    .array
                    .move_member(self.source, index_of(*column)?, *index)
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
                    .insert(self.source, index + 1, &self.source[row.source.range()])
            }
            ColumnChange::Set { column, field, .. } | ColumnChange::Clear { column, field } => {
                let index = index_of(*column)?;
                let owner = &columns.rows[index];
                let field = model.columns[index]
                    .fields
                    .iter()
                    .find(|entry| entry.name == field)
                    .ok_or("Choose a mapped column property")?;
                let value = match change {
                    ColumnChange::Set { value, .. } => Some(value.as_str()),
                    _ => None,
                };
                Ok(fields::change(
                    self.source,
                    owner.fields.get(field.name),
                    owner.source.end,
                    field,
                    value,
                )?
                .into_iter()
                .collect())
            }
        }
    }

    fn column_owner(&self, document: &Document, table: NodeId) -> Option<usize> {
        let owner = document.ast().node(table)?;
        self.preview_markup()
            .controls
            .binary_search(&owner.span.start)
            .ok()
    }
}

fn column_fragment(header: Option<&str>) -> Result<String, String> {
    let header = header.unwrap_or("Column");
    if header.len() > 4096 || !fields::column_text(header) {
        return Err("Enter a single-line column header of up to 4096 bytes".into());
    }
    Ok(format!("::ferroui_view::table_column().header({header:?})"))
}
