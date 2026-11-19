use super::{AuthoredColumn, ColumnChoice, ColumnKind, TableColumns, TableKind, fields};
use crate::layout_attributes::framework_namespace;
use crate::{Result, component_property_attribute, component_property_elements, error, require};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind};

pub(super) struct Collection<'a> {
    document: &'a Document,
    owner: &'a Node,
    parent: &'a Node,
    kind: TableKind,
    columns: Vec<&'a Node>,
    reason: Option<&'static str>,
}

pub(super) fn table_kind(node: &Node) -> Option<TableKind> {
    if node.kind != NodeKind::Element || !native_namespace(node.namespace_uri.as_deref()) {
        return None;
    }
    match node.local_name() {
        "TableView" => Some(TableKind::TableView),
        "TreeDataGrid" => Some(TableKind::TreeDataGrid),
        _ => None,
    }
}

fn native_namespace(namespace: Option<&str>) -> bool {
    framework_namespace(namespace)
        || matches!(
            namespace,
            Some(
                "using:FerroUI.Controls.TreeDataGrid"
                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls.TreeDataGrid"
                    | "clr-namespace:FerroUI.Controls.TreeDataGrid;assembly=FerroUI.Controls.TreeDataGrid"
            )
        )
}

impl<'a> Collection<'a> {
    pub(super) fn new(document: &'a Document, table: NodeId) -> Result<Self> {
        require(
            document.is_valid(),
            "Repair the XAML before editing columns",
        )?;
        let ast = document.ast();
        let owner = ast
            .node(table)
            .ok_or_else(|| error("The table was removed"))?;
        let kind = table_kind(owner).ok_or_else(|| error("Select a TableView or TreeDataGrid"))?;
        let property = kind.property();
        let explicit = component_property_elements(document, table, property)?;
        let parent = explicit
            .first()
            .and_then(|node| ast.node(*node))
            .unwrap_or(owner);
        let find_kind = |node: &&Node| column_kind(kind, node).is_some();
        let children = |node: &Node| {
            node.children
                .iter()
                .filter_map(|id| ast.node(*id))
                .collect::<Vec<_>>()
        };
        let inline = children(owner)
            .into_iter()
            .filter(find_kind)
            .collect::<Vec<_>>();
        let direct = children(parent);
        let columns = if explicit.is_empty() {
            if kind == TableKind::TreeDataGrid {
                inline.clone()
            } else {
                Vec::new()
            }
        } else {
            direct.iter().copied().filter(find_kind).collect()
        };
        let mut runtime_collection =
            component_property_attribute(document, table, property)?.is_some();
        if kind == TableKind::TreeDataGrid {
            for source in ["Source", "Model"] {
                runtime_collection |= component_property_attribute(document, table, source)?
                    .is_some()
                    || !component_property_elements(document, table, source)?.is_empty();
            }
        }
        let other_content = (parent.id != owner.id || kind == TableKind::TreeDataGrid)
            && direct.iter().any(|node| {
                (node.kind == NodeKind::Element && column_kind(kind, node).is_none())
                    || (matches!(node.kind, NodeKind::Text | NodeKind::CData)
                        && node
                            .text
                            .as_deref()
                            .is_some_and(|text| !text.trim().is_empty()))
            });
        let reason = if runtime_collection {
            Some("The application supplies this column collection. Open its source to edit it.")
        } else if other_content
            || explicit.len() > 1
            || (!explicit.is_empty() && !inline.is_empty())
        {
            Some("Edit this column collection in source to preserve its custom definitions.")
        } else {
            None
        };
        Ok(Self {
            document,
            owner,
            parent,
            kind,
            columns,
            reason,
        })
    }

    pub(super) fn model(&self) -> Result<TableColumns> {
        let columns = self
            .columns
            .iter()
            .map(|node| self.describe(node))
            .collect::<Result<Vec<_>>>()?;
        let available = self
            .kind
            .column_types()
            .iter()
            .copied()
            .filter(|kind| {
                *kind != ColumnKind::Expander || !columns.iter().any(|column| column.kind == *kind)
            })
            .map(|kind| ColumnChoice {
                kind,
                label: kind.label(),
            })
            .collect();
        Ok(TableColumns {
            table: self.owner.id,
            name: self.owner.xaml_name().map(str::to_owned),
            kind: self.kind,
            columns,
            available,
            reason: self.reason.map(str::to_owned),
        })
    }

    fn describe(&self, node: &Node) -> Result<AuthoredColumn> {
        let kind = column_kind(self.kind, node).ok_or_else(|| error("The column type changed"))?;
        let fields = fields::describe(self.document, node, kind)?;
        let label = fields
            .iter()
            .find(|field| field.name == "Header" && field.editable)
            .and_then(|field| field.value.as_deref())
            .filter(|header| !header.trim().is_empty())
            .unwrap_or_else(|| kind.label())
            .to_owned();
        let mut children = Vec::new();
        if kind == ColumnKind::Expander {
            let ast = self.document.ast();
            let slot = component_property_elements(self.document, node.id, "InnerColumn")?;
            let owner = slot.first().and_then(|id| ast.node(*id)).unwrap_or(node);
            for child in owner.children.iter().filter_map(|id| ast.node(*id)) {
                if matches!(
                    column_kind(self.kind, child),
                    Some(ColumnKind::Text | ColumnKind::CheckBox | ColumnKind::Template)
                ) {
                    children.push(self.describe(child)?);
                }
            }
        }
        Ok(AuthoredColumn {
            node: node.id,
            kind,
            label,
            source: node.name_span,
            fields,
            children,
            can_duplicate: kind != ColumnKind::Expander
                && self.columns.iter().any(|column| column.id == node.id)
                && !self
                    .document
                    .ast()
                    .subtree(node.id)
                    .any(|item| item.xaml_name().is_some()),
        })
    }

    pub(super) fn column(&self, id: NodeId) -> Result<(usize, &Node)> {
        self.columns
            .iter()
            .copied()
            .enumerate()
            .find(|(_, column)| column.id == id)
            .ok_or_else(|| error("Choose a top-level column in this table"))
    }

    pub(super) fn insert(&self, index: usize, source: String) -> Result<Edit> {
        require(
            index <= self.columns.len(),
            "The column insertion index changed",
        )?;
        let children = self.document.ast().element_children(self.parent.id);
        if self.kind == TableKind::TableView && self.parent.id == self.owner.id {
            let name = self.qualify("TableView.Columns")?;
            return Ok(Edit::InsertChild {
                parent: self.owner.id,
                index: 0,
                source: format!("<{name}>{source}</{name}>"),
            });
        }
        let columns = self.columns.iter().map(|node| node.id).collect::<Vec<_>>();
        Ok(Edit::InsertChild {
            parent: self.parent.id,
            index: child_index(&children, &columns, index),
            source,
        })
    }

    pub(super) fn move_column(&self, column: NodeId, index: usize) -> Result<Vec<Edit>> {
        let (before, _) = self.column(column)?;
        require(index < self.columns.len(), "The column destination changed")?;
        if before == index {
            return Ok(Vec::new());
        }
        let mut children = self.document.ast().element_children(self.parent.id);
        children.retain(|&id| id != column);
        let columns = self
            .columns
            .iter()
            .filter_map(|node| (node.id != column).then_some(node.id))
            .collect::<Vec<_>>();
        Ok(vec![Edit::MoveNode {
            node: column,
            parent: self.parent.id,
            index: child_index(&children, &columns, index),
        }])
    }

    pub(super) fn fragment(&self, kind: ColumnKind, header: Option<&str>) -> Result<String> {
        let tag = self.qualify(kind.element())?;
        let header = header.unwrap_or_else(|| kind.label());
        require(
            header.len() <= 4096 && !header.contains(['\r', '\n']),
            "Enter a single-line column header of up to 4096 bytes",
        )?;
        let header = if header.starts_with('{') {
            format!("{{}}{header}")
        } else {
            header.to_owned()
        };
        let header = header
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;");
        let start = if kind == ColumnKind::RowHeader && header == kind.label() {
            format!("<{tag} Width=\"44\"")
        } else {
            format!("<{tag} Header=\"{header}\"")
        };
        match kind {
            ColumnKind::Template => {
                let property = self.qualify("TreeDataGridTemplateColumn.CellTemplate")?;
                let template = self.qualify("DataTemplate")?;
                let text = self.qualify("TextBlock")?;
                Ok(format!(
                    "{start}><{property}><{template}><{text} Text=\"{{ReflectionBinding}}\" /></{template}></{property}></{tag}>"
                ))
            }
            ColumnKind::Expander => Ok(format!(
                "{start} ChildrenBinding=\"{{ReflectionBinding Children}}\"><{} Binding=\"{{ReflectionBinding}}\" /></{tag}>",
                self.qualify(ColumnKind::Text.element())?
            )),
            ColumnKind::Text => Ok(format!("{start} Binding=\"{{ReflectionBinding}}\" />")),
            ColumnKind::CheckBox => Ok(format!(
                "{start} Binding=\"{{ReflectionBinding IsChecked}}\" />"
            )),
            _ => Ok(format!("{start} />")),
        }
    }

    fn qualify(&self, local: &str) -> Result<String> {
        let namespaces = self.document.ast().namespaces(self.parent.id);
        if native_namespace(namespaces.get("").map(String::as_str)) {
            return Ok(local.to_owned());
        }
        namespaces
            .iter()
            .find(|(prefix, namespace)| !prefix.is_empty() && native_namespace(Some(namespace)))
            .map(|(prefix, _)| format!("{prefix}:{local}"))
            .ok_or_else(|| error("Declare a FerroUI namespace for new columns"))
    }
}

fn child_index(children: &[NodeId], columns: &[NodeId], index: usize) -> usize {
    if let Some(anchor) = columns.get(index) {
        children
            .iter()
            .position(|id| id == anchor)
            .unwrap_or(children.len())
    } else if let Some(anchor) = columns.last() {
        children
            .iter()
            .position(|id| id == anchor)
            .map_or(children.len(), |index| index + 1)
    } else {
        children.len()
    }
}

fn column_kind(table: TableKind, node: &Node) -> Option<ColumnKind> {
    if node.kind != NodeKind::Element || !native_namespace(node.namespace_uri.as_deref()) {
        return None;
    }
    table
        .column_types()
        .iter()
        .copied()
        .find(|kind| kind.element() == node.local_name())
}
