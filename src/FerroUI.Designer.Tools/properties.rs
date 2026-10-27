use crate::{GridLength, Result, ToolError, error, require};
use ferroui_designer_core::{Document, Edit, MarkupValue, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "property_values.rs"]
mod values;
pub use values::PropertyValueShape;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PropertyKind {
    Text,
    Boolean,
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
        allow_auto: bool,
    },
    Integer {
        minimum: i64,
        maximum: i64,
    },
    Enum(Vec<String>),
    Flags(Vec<String>),
    Typed(PropertyValueShape),
    Thickness,
    Color,
    GridLength,
    GridDefinitions,
    ObjectReference,
    Transform,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyDescriptor {
    pub name: String,
    pub category: String,
    pub kind: PropertyKind,
    pub read_only: bool,
    pub attached: bool,
    pub animatable: bool,
}
impl PropertyDescriptor {
    pub fn new(name: impl Into<String>, category: impl Into<String>, kind: PropertyKind) -> Self {
        let name = name.into();
        let attached = name.contains('.');
        let animatable = matches!(
            kind,
            PropertyKind::Number { .. }
                | PropertyKind::Thickness
                | PropertyKind::Color
                | PropertyKind::Transform
        );
        Self {
            name,
            category: category.into(),
            kind,
            read_only: false,
            attached,
            animatable,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpressionPolicy {
    #[default]
    PreserveExpressions,
    ReplaceExpressions,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PropertyState {
    pub value: Option<String>,
    pub mixed: bool,
    pub authored: usize,
    pub expression: bool,
}
/// Property validation is independent of native editor widgets. A host can register a custom
/// value editor first and use its ID to choose an associated native control factory.
pub trait ValueEditor {
    fn id(&self) -> &str;
    fn supports(&self, property: &PropertyDescriptor) -> bool;
    fn validate(&self, property: &PropertyDescriptor, value: &str) -> Result<()>;
}
pub struct EditorRegistry {
    editors: Vec<Box<dyn ValueEditor>>,
}
impl Default for EditorRegistry {
    fn default() -> Self {
        Self::built_in()
    }
}
impl EditorRegistry {
    pub fn built_in() -> Self {
        Self {
            editors: vec![Box::new(BuiltinEditor)],
        }
    }
    pub fn register(&mut self, editor: Box<dyn ValueEditor>) {
        self.editors.insert(0, editor);
    }
    pub fn editor(&self, property: &PropertyDescriptor) -> Option<&dyn ValueEditor> {
        self.editors
            .iter()
            .find(|editor| editor.supports(property))
            .map(Box::as_ref)
    }
    pub fn validate(&self, property: &PropertyDescriptor, value: &str) -> Result<()> {
        require(
            !property.read_only,
            format!("{} is read-only", property.name),
        )?;
        if is_expression(value) {
            MarkupValue::parse(value).map_err(ToolError)?;
            return Ok(());
        }
        self.editor(property)
            .ok_or_else(|| ToolError(format!("No editor registered for {}", property.name)))?
            .validate(property, value)
    }
}
#[derive(Default)]
pub struct PropertyRegistry {
    types: BTreeMap<String, BTreeMap<String, PropertyDescriptor>>,
}
impl PropertyRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn built_in() -> Self {
        let mut r = Self::new();
        for name in ["Name", "Classes", "Tag", "ToolTip.Tip"] {
            r.add("*", name, "Identity", PropertyKind::Text);
        }
        for name in [
            "Width",
            "Height",
            "MinWidth",
            "MinHeight",
            "MaxWidth",
            "MaxHeight",
        ] {
            r.add(
                "*",
                name,
                "Layout",
                PropertyKind::Number {
                    minimum: Some(0.0),
                    maximum: None,
                    allow_auto: matches!(name, "Width" | "Height"),
                },
            );
        }
        for name in ["Margin", "Padding", "BorderThickness", "CornerRadius"] {
            r.add("*", name, "Layout", PropertyKind::Thickness);
        }
        for name in ["HorizontalAlignment", "HorizontalContentAlignment"] {
            r.enumeration("*", name, "Layout", &["Left", "Center", "Right", "Stretch"]);
        }
        for name in ["VerticalAlignment", "VerticalContentAlignment"] {
            r.enumeration("*", name, "Layout", &["Top", "Center", "Bottom", "Stretch"]);
        }
        for name in ["IsVisible", "IsEnabled", "ClipToBounds", "Focusable"] {
            r.add("*", name, "Appearance", PropertyKind::Boolean);
        }
        for name in ["Background", "Foreground", "BorderBrush", "Fill", "Stroke"] {
            r.add("*", name, "Brush", PropertyKind::Color);
        }
        r.add(
            "*",
            "Opacity",
            "Appearance",
            PropertyKind::Number {
                minimum: Some(0.0),
                maximum: Some(1.0),
                allow_auto: false,
            },
        );
        r.add("*", "RenderTransform", "Transform", PropertyKind::Transform);
        r.add(
            "*",
            "RenderTransformOrigin",
            "Transform",
            PropertyKind::Text,
        );
        for name in ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"] {
            r.add("@Canvas", name, "Canvas", finite_number());
        }
        for name in ["Grid.Row", "Grid.Column"] {
            r.add("@Grid", name, "Grid", integer(0));
        }
        for name in ["Grid.RowSpan", "Grid.ColumnSpan"] {
            r.add("@Grid", name, "Grid", integer(1));
        }
        for name in ["RowDefinitions", "ColumnDefinitions"] {
            r.add("Grid", name, "Grid", PropertyKind::GridDefinitions);
        }
        r.add("Grid", "ShowGridLines", "Grid", PropertyKind::Boolean);
        for panel in ["Grid", "UniformGrid", "FlexPanel"] {
            for name in ["RowSpacing", "ColumnSpacing"] {
                r.add(panel, name, "Layout", nonnegative());
            }
        }
        for panel in [
            "StackPanel",
            "ReversibleStackPanel",
            "WrapPanel",
            "VirtualizingStackPanel",
        ] {
            r.enumeration(panel, "Orientation", "Layout", &["Horizontal", "Vertical"]);
        }
        for panel in ["StackPanel", "ReversibleStackPanel"] {
            r.add(panel, "Spacing", "Layout", finite_number());
        }
        r.add(
            "ReversibleStackPanel",
            "ReverseOrder",
            "Layout",
            PropertyKind::Boolean,
        );
        for name in ["ItemWidth", "ItemHeight"] {
            r.add(
                "WrapPanel",
                name,
                "Layout",
                PropertyKind::Number {
                    minimum: Some(0.0),
                    maximum: None,
                    allow_auto: true,
                },
            );
        }
        for name in ["ItemSpacing", "LineSpacing"] {
            r.add("WrapPanel", name, "Layout", nonnegative());
        }
        r.enumeration(
            "WrapPanel",
            "ItemsAlignment",
            "Layout",
            &["Start", "Center", "End", "Justify", "Stretch", "StretchAll"],
        );
        r.add("DockPanel", "LastChildFill", "Dock", PropertyKind::Boolean);
        for name in ["HorizontalSpacing", "VerticalSpacing"] {
            r.add("DockPanel", name, "Dock", nonnegative());
        }
        r.enumeration(
            "@DockPanel",
            "DockPanel.Dock",
            "Dock",
            &["Left", "Top", "Right", "Bottom"],
        );
        for name in ["Rows", "Columns", "FirstColumn"] {
            r.add("UniformGrid", name, "Uniform grid", integer(0));
        }
        r.enumeration(
            "FlexPanel",
            "Direction",
            "Flex",
            &["Row", "RowReverse", "Column", "ColumnReverse"],
        );
        r.enumeration(
            "FlexPanel",
            "Wrap",
            "Flex",
            &["NoWrap", "Wrap", "WrapReverse"],
        );
        r.enumeration(
            "FlexPanel",
            "JustifyContent",
            "Flex",
            &[
                "FlexStart",
                "FlexEnd",
                "Center",
                "SpaceBetween",
                "SpaceAround",
                "SpaceEvenly",
            ],
        );
        r.enumeration(
            "FlexPanel",
            "AlignItems",
            "Flex",
            &["FlexStart", "FlexEnd", "Center", "Stretch"],
        );
        r.enumeration(
            "FlexPanel",
            "AlignContent",
            "Flex",
            &[
                "FlexStart",
                "FlexEnd",
                "Center",
                "Stretch",
                "SpaceBetween",
                "SpaceAround",
                "SpaceEvenly",
            ],
        );
        r.enumeration(
            "@FlexPanel",
            "Flex.AlignSelf",
            "Flex",
            &["FlexStart", "FlexEnd", "Center", "Stretch"],
        );
        r.add(
            "@FlexPanel",
            "Flex.Order",
            "Flex",
            PropertyKind::Integer {
                minimum: i32::MIN as i64,
                maximum: i32::MAX as i64,
            },
        );
        r.add("@FlexPanel", "Flex.Basis", "Flex", PropertyKind::GridLength);
        for name in ["Flex.Grow", "Flex.Shrink"] {
            r.add("@FlexPanel", name, "Flex", nonnegative());
        }
        for name in [
            "LeftOf",
            "RightOf",
            "Above",
            "Below",
            "AlignLeftWith",
            "AlignRightWith",
            "AlignTopWith",
            "AlignBottomWith",
            "AlignHorizontalCenterWith",
            "AlignVerticalCenterWith",
        ] {
            r.add(
                "@RelativePanel",
                &format!("RelativePanel.{name}"),
                "Relative",
                PropertyKind::ObjectReference,
            );
        }
        for name in [
            "AlignLeftWithPanel",
            "AlignRightWithPanel",
            "AlignTopWithPanel",
            "AlignBottomWithPanel",
            "AlignHorizontalCenterWithPanel",
            "AlignVerticalCenterWithPanel",
        ] {
            r.add(
                "@RelativePanel",
                &format!("RelativePanel.{name}"),
                "Relative",
                PropertyKind::Boolean,
            );
        }
        r.add(
            "VirtualizingStackPanel",
            "CacheLength",
            "Virtualization",
            nonnegative(),
        );
        r.add(
            "DateTimePickerPanel",
            "ItemHeight",
            "Generated layout",
            nonnegative(),
        );
        r.add(
            "DateTimePickerPanel",
            "PanelType",
            "Generated layout",
            PropertyKind::Text,
        );
        r.add(
            "DateTimePickerPanel",
            "ItemFormat",
            "Generated layout",
            PropertyKind::Text,
        );
        r.add(
            "DateTimePickerPanel",
            "ShouldLoop",
            "Generated layout",
            PropertyKind::Boolean,
        );
        r
    }
    fn add(&mut self, owner: &str, name: &str, category: &str, kind: PropertyKind) {
        self.register(owner, PropertyDescriptor::new(name, category, kind));
    }
    fn enumeration(&mut self, owner: &str, name: &str, category: &str, values: &[&str]) {
        self.add(
            owner,
            name,
            category,
            PropertyKind::Enum(values.iter().map(|s| (*s).into()).collect()),
        );
    }
    /// Use "*" for common properties and "@PanelType" for child attached properties.
    pub fn register(
        &mut self,
        owner: impl Into<String>,
        property: PropertyDescriptor,
    ) -> Option<PropertyDescriptor> {
        self.types
            .entry(owner.into())
            .or_default()
            .insert(property.name.clone(), property)
    }
    pub fn descriptors(
        &self,
        document: &Document,
        node: NodeId,
    ) -> Result<Vec<PropertyDescriptor>> {
        let element = document
            .ast()
            .node(node)
            .ok_or_else(|| ToolError("Unknown property target".into()))?;
        require(element.is_element(), "Property target must be an element")?;
        let mut descriptors = self.types.get("*").cloned().unwrap_or_default();
        if let Some(properties) = self.types.get(local_name(&element.name)) {
            descriptors.extend(properties.clone());
        }
        let mut parent = element.parent;
        while let Some(id) = parent {
            let p = document.ast().node(id).unwrap();
            if p.kind != ferroui_designer_core::NodeKind::PropertyElement {
                if let Some(properties) = self.types.get(&format!("@{}", local_name(&p.name))) {
                    descriptors.extend(properties.clone());
                }
                break;
            }
            parent = p.parent;
        }
        for attribute in &element.attributes {
            if !attribute.is_namespace() {
                descriptors
                    .entry(attribute.name.clone())
                    .or_insert_with(|| {
                        PropertyDescriptor::new(&attribute.name, "Authored", PropertyKind::Text)
                    });
            }
        }
        Ok(descriptors.into_values().collect())
    }
    pub fn state(
        &self,
        document: &Document,
        nodes: &[NodeId],
        property: &str,
    ) -> Result<PropertyState> {
        require(!nodes.is_empty(), "Select at least one object")?;
        let mut result = PropertyState::default();
        let mut initial = false;
        for node in nodes {
            let node = document
                .ast()
                .node(*node)
                .ok_or_else(|| ToolError("Unknown property target".into()))?;
            let value = crate::components::attributes_for(document, node, property)?
                .into_iter()
                .next()
                .map(|attribute| attribute.value.clone());
            if !initial {
                result.value = value.clone();
                initial = true;
            } else if result.value != value {
                result.mixed = true;
            }
            if let Some(value) = value {
                result.authored += 1;
                result.expression |= is_expression(&value);
            }
        }
        if result.mixed {
            result.value = None;
        }
        Ok(result)
    }
    pub fn plan(
        &self,
        document: &Document,
        nodes: &[NodeId],
        property: &str,
        value: Option<&str>,
        policy: ExpressionPolicy,
    ) -> Result<Vec<Edit>> {
        self.plan_with(
            document,
            nodes,
            property,
            value,
            policy,
            &EditorRegistry::built_in(),
        )
    }
    pub fn plan_with(
        &self,
        document: &Document,
        nodes: &[NodeId],
        property: &str,
        value: Option<&str>,
        policy: ExpressionPolicy,
        editors: &EditorRegistry,
    ) -> Result<Vec<Edit>> {
        require(
            document.is_valid(),
            "Property editing requires valid source",
        )?;
        require(!nodes.is_empty(), "Select at least one property target")?;
        let mut edits = vec![];
        let mut seen = BTreeSet::new();
        for id in nodes {
            if !seen.insert(*id) {
                continue;
            }
            let element = document
                .ast()
                .node(*id)
                .ok_or_else(|| ToolError("Unknown property target".into()))?;
            let descriptor = self
                .descriptors(document, *id)?
                .into_iter()
                .find(|p| p.name == property)
                .unwrap_or_else(|| PropertyDescriptor::new(property, "Custom", PropertyKind::Text));
            require(!descriptor.read_only, format!("{property} is read-only"))?;
            let attribute = crate::components::attributes_for(document, element, property)?
                .into_iter()
                .next();
            if policy == ExpressionPolicy::PreserveExpressions
                && attribute.is_some_and(|attribute| is_expression(&attribute.value))
                && attribute.map(|attribute| attribute.value.as_str()) != value
            {
                return Err(ToolError(format!(
                    "{property} is an expression; explicitly opt into replacing it"
                )));
            }
            require(
                crate::components::property_elements(document, element, property)?.is_empty(),
                "Property is authored as a property element; edit that node to preserve its object graph",
            )?;
            if let Some(value) = value {
                editors.validate(&descriptor, value)?;
                edits.push(Edit::SetAttribute {
                    node: *id,
                    name: attribute
                        .map_or(property, |attribute| attribute.name.as_str())
                        .into(),
                    value: value.into(),
                });
            } else {
                edits.push(Edit::RemoveAttribute {
                    node: *id,
                    name: attribute
                        .map_or(property, |attribute| attribute.name.as_str())
                        .into(),
                });
            }
        }
        let mut candidate = document.clone();
        candidate
            .apply("Validate property edits", edits.clone())
            .map_err(error)?;
        Ok(edits)
    }
}
struct BuiltinEditor;
impl ValueEditor for BuiltinEditor {
    fn id(&self) -> &str {
        "builtin"
    }
    fn supports(&self, _: &PropertyDescriptor) -> bool {
        true
    }
    fn validate(&self, property: &PropertyDescriptor, value: &str) -> Result<()> {
        let literal = value.strip_prefix("{}").unwrap_or(value);
        match &property.kind {
            PropertyKind::Text | PropertyKind::ObjectReference | PropertyKind::Transform => Ok(()),
            PropertyKind::Boolean => require(
                literal.eq_ignore_ascii_case("true") || literal.eq_ignore_ascii_case("false"),
                "Boolean value must be True or False",
            ),
            PropertyKind::Number {
                minimum,
                maximum,
                allow_auto,
            } => {
                if *allow_auto && matches!(literal, "Auto" | "NaN") {
                    return Ok(());
                }
                let number = crate::layout::number(literal)?;
                require(
                    minimum.is_none_or(|min| number >= min)
                        && maximum.is_none_or(|max| number <= max),
                    format!("{} is outside the property range", property.name),
                )
            }
            PropertyKind::Integer { minimum, maximum } => {
                let value = literal
                    .parse::<i64>()
                    .map_err(|_| ToolError("Expected an integer".into()))?;
                require(
                    value >= *minimum && value <= *maximum,
                    "Integer is outside the property range",
                )
            }
            PropertyKind::Enum(values) => require(
                values.iter().any(|v| v == literal),
                format!("Choose one of {}", values.join(", ")),
            ),
            PropertyKind::Flags(values) => require(
                literal.split(',').all(|part| {
                    values
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(part.trim()))
                }),
                format!("Choose flags from {}", values.join(", ")),
            ),
            PropertyKind::Typed(shape) => shape.validate(literal),
            PropertyKind::Thickness => {
                crate::layout::thickness(literal)?;
                Ok(())
            }
            PropertyKind::GridLength => {
                GridLength::parse(literal)?;
                Ok(())
            }
            PropertyKind::GridDefinitions => {
                require(!literal.is_empty(), "Grid definitions cannot be empty")?;
                for length in literal.split(',') {
                    GridLength::parse(length)?;
                }
                Ok(())
            }
            PropertyKind::Color => {
                if let Some(hex) = literal.strip_prefix('#') {
                    require(
                        matches!(hex.len(), 3 | 4 | 6 | 8)
                            && hex.bytes().all(|b| b.is_ascii_hexdigit()),
                        "Use #RGB, #ARGB, #RRGGBB or #AARRGGBB",
                    )
                } else {
                    require(
                        !literal.is_empty() && literal.chars().all(|c| c.is_ascii_alphabetic()),
                        "Color must be a hexadecimal color, named color or XAML expression",
                    )
                }
            }
        }
    }
}
fn finite_number() -> PropertyKind {
    PropertyKind::Number {
        minimum: None,
        maximum: None,
        allow_auto: false,
    }
}
fn nonnegative() -> PropertyKind {
    PropertyKind::Number {
        minimum: Some(0.0),
        maximum: None,
        allow_auto: false,
    }
}
fn integer(minimum: i64) -> PropertyKind {
    PropertyKind::Integer {
        minimum,
        maximum: i32::MAX as i64,
    }
}
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
fn is_expression(source: &str) -> bool {
    let source = source.trim_start();
    source.starts_with('{') && !source.starts_with("{}")
}

#[cfg(test)]
#[path = "property_identity_tests.rs"]
mod identity_tests;
