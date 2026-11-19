use super::{AuthoredColumn, ColumnKind};
use crate::{
    GridLength, Result, ToolError, component_property_attribute, component_property_elements,
    error, require,
};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, SourceSpan};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnFieldKind {
    Text,
    Length,
    Boolean,
    Choice,
    Binding,
    Object,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnField {
    pub name: &'static str,
    pub kind: ColumnFieldKind,
    pub value: Option<String>,
    pub choices: &'static [&'static str],
    pub source: Option<SourceSpan>,
    pub editable: bool,
    pub can_clear: bool,
}

use ColumnFieldKind::*;
const BOOL: &[&str] = &["True", "False"];
const ALIGN: &[&str] = &["Left", "Center", "Right", "Stretch"];
const TEXT_ALIGN: &[&str] = &[
    "Left",
    "Center",
    "Right",
    "Justify",
    "Start",
    "End",
    "DetectFromContent",
];
const WRAP: &[&str] = &["NoWrap", "Wrap", "WrapWithOverflow"];

type Field = (&'static str, ColumnFieldKind, &'static [&'static str]);
const COMMON: &[Field] = &[("Header", Text, &[]), ("Width", Length, &[])];
const TABLE: &[Field] = &[
    ("Binding", Binding, &[]),
    ("IsVisible", Boolean, BOOL),
    ("CanUserResize", Boolean, BOOL),
    ("HorizontalContentAlignment", Choice, ALIGN),
    ("CellTemplate", Object, &[]),
    ("HeaderTemplate", Object, &[]),
    ("CellTheme", Object, &[]),
    ("HeaderTheme", Object, &[]),
];
const TREE: &[Field] = &[
    ("MinWidth", Length, &[]),
    ("MaxWidth", Length, &[]),
    ("CanUserResize", Boolean, BOOL),
    ("CanUserSortColumn", Boolean, BOOL),
    ("AllowTriStateSorting", Boolean, BOOL),
];
const TEXT: &[Field] = &[
    ("Binding", Binding, &[]),
    ("IsReadOnly", Boolean, BOOL),
    ("StringFormat", Text, &[]),
    ("TextAlignment", Choice, TEXT_ALIGN),
    ("TextWrapping", Choice, WRAP),
    ("IsTextSearchEnabled", Boolean, BOOL),
    ("TextTrimming", Object, &[]),
];
const CHECKBOX: &[Field] = &[
    ("Binding", Binding, &[]),
    ("IsReadOnly", Boolean, BOOL),
    ("IsThreeState", Boolean, BOOL),
];
const TEMPLATE: &[Field] = &[
    ("CellTemplate", Object, &[]),
    ("CellEditingTemplate", Object, &[]),
    ("CellTemplateResourceKey", Object, &[]),
    ("CellEditingTemplateResourceKey", Object, &[]),
    ("TextSearchBinding", Binding, &[]),
];
const EXPANDER: &[Field] = &[
    ("ChildrenBinding", Binding, &[]),
    ("HasChildrenBinding", Binding, &[]),
    ("IsExpandedBinding", Binding, &[]),
];

fn definitions(kind: ColumnKind) -> impl Iterator<Item = Field> {
    let specific = match kind {
        ColumnKind::Table => TABLE,
        ColumnKind::Text => TEXT,
        ColumnKind::CheckBox => CHECKBOX,
        ColumnKind::Template => TEMPLATE,
        ColumnKind::Expander => EXPANDER,
        ColumnKind::RowHeader => &[],
    };
    let tree = if kind == ColumnKind::Table { &[] } else { TREE };
    COMMON.iter().chain(specific).chain(tree).copied()
}

pub(super) fn describe(
    document: &Document,
    node: &Node,
    column: ColumnKind,
) -> Result<Vec<ColumnField>> {
    definitions(column)
        .map(|(name, kind, choices)| {
            let attribute = component_property_attribute(document, node.id, name)?;
            let object = component_property_elements(document, node.id, name)?
                .first()
                .and_then(|id| document.ast().node(*id));
            let mut editable = kind != Object && object.is_none();
            let value = if let Some(attr) = attribute {
                match attr.markup() {
                    Ok(MarkupValue::Literal(literal)) => {
                        editable &= kind != Binding;
                        Some(literal)
                    }
                    Ok(value) => {
                        editable &= kind == Binding && binding(document, node.id, &value);
                        Some(attr.value.clone())
                    }
                    Err(_) => {
                        editable = false;
                        Some(attr.value.clone())
                    }
                }
            } else {
                None
            };
            let source = attribute
                .map(|attr| attr.span)
                .or_else(|| object.map(|node| node.name_span));
            let required = matches!(
                (column, name),
                (ColumnKind::Expander, "ChildrenBinding")
                    | (ColumnKind::Text | ColumnKind::CheckBox, "Binding")
            );
            Ok(ColumnField {
                name,
                kind,
                value,
                choices,
                source,
                editable,
                can_clear: editable && source.is_some() && !required,
            })
        })
        .collect()
}

pub(super) fn plan(
    document: &Document,
    columns: &[AuthoredColumn],
    id: NodeId,
    property: &str,
    value: Option<&String>,
) -> Result<Vec<Edit>> {
    let owner = columns
        .iter()
        .flat_map(|column| std::iter::once(column).chain(&column.children))
        .find(|column| column.node == id)
        .ok_or_else(|| error("The column is outside this table"))?;
    let field = owner
        .fields
        .iter()
        .find(|field| field.name == property)
        .ok_or_else(|| error("Choose a supported column property"))?;
    require(
        field.editable,
        "Open the authored expression or object value in source",
    )?;
    let authored = component_property_attribute(document, id, property)?;
    let name = authored.map_or_else(|| property.to_owned(), |attr| attr.name.clone());
    match value {
        None if authored.is_none() => Ok(Vec::new()),
        None => {
            require(field.can_clear, "This column requires a binding")?;
            Ok(vec![Edit::RemoveAttribute { node: id, name }])
        }
        Some(value) => {
            let value = normalize(document, id, field, value)?;
            if authored.is_some_and(|attr| attr.value == value) {
                return Ok(Vec::new());
            }
            Ok(vec![Edit::SetAttribute {
                node: id,
                name,
                value,
            }])
        }
    }
}

fn normalize(
    document: &Document,
    node: NodeId,
    field: &ColumnField,
    value: &str,
) -> Result<String> {
    require(
        value.len() <= 4096 && !value.contains(['\r', '\n']),
        "Enter a single-line column value of up to 4096 bytes",
    )?;
    match field.kind {
        Text => Ok(if value.starts_with('{') {
            format!("{{}}{value}")
        } else {
            value.to_owned()
        }),
        Length => GridLength::parse(value).map(GridLength::to_xaml),
        Boolean | Choice => field
            .choices
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(value.trim()))
            .map(|candidate| (*candidate).to_owned())
            .ok_or_else(|| error("Choose a supported column value")),
        Binding => {
            let value = value.trim();
            let parsed = MarkupValue::parse(value).map_err(ToolError)?;
            require(
                binding(document, node, &parsed),
                "Enter a FerroUI Binding, CompiledBinding, or ReflectionBinding expression",
            )?;
            Ok(value.to_owned())
        }
        Object => Err(error("Edit object values in source")),
    }
}

fn binding(document: &Document, node: NodeId, value: &MarkupValue) -> bool {
    let MarkupValue::Extension(extension) = value else {
        return false;
    };
    let namespace = document.ast().namespaces(node);
    let (prefix, name) = extension
        .name
        .split_once(':')
        .unwrap_or(("", &extension.name));
    (prefix.is_empty() || namespace.contains_key(prefix))
        && crate::layout_attributes::framework_namespace(namespace.get(prefix).map(String::as_str))
        && matches!(
            name.strip_suffix("Extension").unwrap_or(name),
            "Binding" | "CompiledBinding" | "ReflectionBinding"
        )
}
