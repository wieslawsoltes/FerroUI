use super::fields::{self, SourceField, reflection_binding};
use crate::{
    animation::syntax::{chain, method_suffix, span},
    tree::escape_xml,
};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_tools::{
    AuthoredColumn, ColumnFieldKind, ColumnKind, TableColumns, TableKind,
};
use std::{collections::BTreeMap, fmt::Write};
use syn::Expr;

pub(super) struct Column {
    pub(super) source: SourceSpan,
    pub(super) append_at: usize,
    pub(super) kind: ColumnKind,
    pub(super) fields: BTreeMap<&'static str, SourceField>,
    pub(super) inner: Option<Box<Column>>,
}

#[derive(Clone, Copy)]
pub(crate) struct ColumnPosition {
    index: usize,
    inner: bool,
}

impl ColumnPosition {
    pub(super) fn find(model: &TableColumns, id: NodeId) -> Option<(Self, &AuthoredColumn)> {
        model
            .columns
            .iter()
            .enumerate()
            .find_map(|(index, column)| {
                if column.node == id {
                    return Some((
                        Self {
                            index,
                            inner: false,
                        },
                        column,
                    ));
                }
                let child = column.children.first().filter(|inner| inner.node == id)?;
                Some((Self { index, inner: true }, child))
            })
    }

    pub(super) fn at(self, columns: &[Column]) -> Option<&Column> {
        let column = columns.get(self.index)?;
        if self.inner {
            column.inner.as_deref()
        } else {
            Some(column)
        }
    }
}

impl Column {
    pub(super) fn read(
        expression: &Expr,
        table: TableKind,
        resolve: &dyn Fn(&Expr) -> Option<String>,
    ) -> Option<Self> {
        Self::parse(expression, table, false, resolve)
    }

    fn parse(
        expression: &Expr,
        table: TableKind,
        nested: bool,
        resolve: &dyn Fn(&Expr) -> Option<String>,
    ) -> Option<Self> {
        let (factory, mut methods) = chain(expression)?;
        if !factory.attrs.is_empty() || methods.iter().any(|method| !method.attrs.is_empty()) {
            return None;
        }
        let qualified = resolve(&factory.func)?;
        let parent = qualified.strip_prefix("ferroui_view::")?;
        let name = parent.strip_prefix("prelude::").unwrap_or(parent);
        let kind = match (table, name) {
            (TableKind::TableView, "table_column") => ColumnKind::Table,
            (TableKind::TreeDataGrid, "tree_data_grid_text_column") => ColumnKind::Text,
            (TableKind::TreeDataGrid, "tree_data_grid_check_box_column") => ColumnKind::CheckBox,
            (TableKind::TreeDataGrid, "tree_data_grid_template_column") => ColumnKind::Template,
            (TableKind::TreeDataGrid, "tree_data_grid_row_header_column") if !nested => {
                ColumnKind::RowHeader
            }
            (TableKind::TreeDataGrid, "tree_data_grid_expander_column") if !nested => {
                ColumnKind::Expander
            }
            _ => return None,
        };
        let inner = if kind == ColumnKind::Expander && factory.args.len() == 1 {
            Some(Box::new(Self::parse(
                &factory.args[0],
                table,
                true,
                resolve,
            )?))
        } else if kind != ColumnKind::Expander && factory.args.is_empty() {
            None
        } else {
            return None;
        };
        let source = span(expression);
        let append_at = if table == TableKind::TreeDataGrid {
            let build = methods
                .pop()
                .filter(|method| method.method == "build" && method.args.is_empty())?;
            method_suffix(build).start
        } else {
            source.end
        };
        let mut fields = BTreeMap::new();
        for method in methods {
            let field = SourceField::read(kind, method)?;
            if fields.insert(field.name(), field).is_some() {
                return None;
            }
        }
        Some(Self {
            source,
            append_at,
            kind,
            fields,
            inner,
        })
    }

    pub(super) fn can_duplicate(&self) -> bool {
        self.inner.is_none() && self.projects_values()
    }

    pub(super) fn projects_values(&self) -> bool {
        self.fields.values().all(|field| field.value.is_some())
            && self.inner.as_deref().is_none_or(Self::projects_values)
    }

    fn default(&self) -> Option<(&'static str, &'static str)> {
        match self.kind {
            ColumnKind::Text => Some(("Binding", "")),
            ColumnKind::CheckBox => Some(("Binding", "IsChecked")),
            ColumnKind::Expander => Some(("ChildrenBinding", "Children")),
            ColumnKind::RowHeader => Some(("Width", "44")),
            _ => None,
        }
    }

    fn projected_value(&self, name: &str) -> Option<String> {
        self.fields
            .get(name)
            .and_then(SourceField::render_value)
            .or_else(|| {
                let (field, fallback) = self.default().filter(|(field, _)| *field == name)?;
                Some(if field == "Width" {
                    fallback.into()
                } else {
                    reflection_binding(fallback)
                })
            })
    }

    pub(super) fn render(&self, output: &mut String, padding: &str) {
        let tag = self.kind.element();
        let _ = write!(output, "\n{padding}<{tag}");
        let default = self
            .default()
            .map(|(field, _)| field)
            .filter(|field| !self.fields.contains_key(field));
        for name in self.fields.keys().copied().chain(default) {
            if let Some(value) = self.projected_value(name) {
                let _ = write!(output, " {name}=\"{}\"", escape_xml(&value, Some('"')));
            }
        }
        if let Some(inner) = &self.inner {
            output.push('>');
            inner.render(output, &format!("{padding}    "));
        } else if self.kind == ColumnKind::Template {
            let _ = write!(
                output,
                ">\n{padding}    <{tag}.CellTemplate><DataTemplate><TextBlock Text=\"{{ReflectionBinding}}\" /></DataTemplate></{tag}.CellTemplate>"
            );
        } else {
            output.push_str(" />");
            return;
        }
        let _ = write!(output, "\n{padding}</{tag}>");
    }

    pub(super) fn matches(&self, column: &AuthoredColumn) -> bool {
        self.kind == column.kind
            && column.fields.iter().all(|field| {
                let projected = if field.kind == ColumnFieldKind::Text {
                    self.fields
                        .get(field.name)
                        .and_then(|source| source.value.clone())
                } else {
                    self.projected_value(field.name)
                };
                field.value == projected
            })
            && match (self.inner.as_deref(), column.children.as_slice()) {
                (None, []) => true,
                (Some(inner), [child]) => inner.matches(child),
                _ => false,
            }
    }

    pub(super) fn describe(&self, column: &mut AuthoredColumn, top_level: bool) {
        column.source = self.source;
        column.can_duplicate = top_level && self.can_duplicate();
        for field in &mut column.fields {
            field.source = None;
            field.can_clear = false;
            if let Some(source) = self.fields.get(field.name) {
                source.describe(field);
            } else {
                field.value = self
                    .default()
                    .filter(|(name, _)| *name == field.name)
                    .map(|(_, value)| value.to_owned());
            }
            // The native expander delegates its header and layout to its inner column.
            if self.kind == ColumnKind::Expander && field.kind != ColumnFieldKind::Binding {
                field.editable = false;
                field.can_clear = false;
            }
        }
        if let (Some(inner), [child]) = (self.inner.as_deref(), column.children.as_mut_slice()) {
            inner.describe(child, false);
            column.label = child.label.clone();
        }
    }
}

pub(super) fn fragment(kind: ColumnKind, header: Option<&str>) -> Result<String, String> {
    let header = header.unwrap_or_else(|| kind.label());
    if header.len() > 4096 || !fields::column_text(header) {
        return Err("Enter a single-line column header of up to 4096 bytes".into());
    }
    Ok(match kind {
        ColumnKind::Table => format!("::ferroui_view::table_column().header({header:?})"),
        ColumnKind::RowHeader if header == kind.label() => {
            "::ferroui_view::tree_data_grid_row_header_column().build()".into()
        }
        ColumnKind::Expander => format!(
            "::ferroui_view::tree_data_grid_expander_column({}).build()",
            fragment(ColumnKind::Text, Some(header))?
        ),
        _ => format!(
            "::ferroui_view::tree_data_grid_{}_column().header({header:?}).build()",
            match kind {
                ColumnKind::Text => "text",
                ColumnKind::CheckBox => "check_box",
                ColumnKind::Template => "template",
                ColumnKind::RowHeader => "row_header",
                _ => unreachable!(),
            }
        ),
    })
}
