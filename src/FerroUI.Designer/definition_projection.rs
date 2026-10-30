//! A design-only native host for one authored definition. The projection is never
//! committed; every copied element retains an explicit identity in the original AST.
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, NodeKind};
use ferroui_designer_tools::{component_fragment, ComponentCatalog, ComponentKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const FRAMEWORK: &str = "https://github.com/ferroui";
const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionPreviewOptions {
    /// Qualified XAML type, resolved in the selected definition's namespace scope.
    pub target_type: Option<String>,
    pub sample_content: String,
    pub width: f64,
    pub height: f64,
}
impl Default for DefinitionPreviewOptions {
    fn default() -> Self {
        Self {
            target_type: None,
            sample_content: "Preview content".into(),
            width: 320.0,
            height: 180.0,
        }
    }
}
impl DefinitionPreviewOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !self.width.is_finite()
            || !self.height.is_finite()
            || !(1.0..=8192.0).contains(&self.width)
            || !(1.0..=8192.0).contains(&self.height)
        {
            return Err("Preview dimensions must be between 1 and 8192".into());
        }
        if self.sample_content.len() > 65_536 {
            return Err("Preview sample content is too long".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefinitionScope {
    pub node: NodeId,
    pub title: String,
    pub kind: ComponentKind,
    pub options: DefinitionPreviewOptions,
    pub(crate) return_selection: Vec<NodeId>,
}

pub(crate) struct DefinitionProjection {
    pub document: Document,
    pub identities: BTreeMap<NodeId, NodeId>,
    pub notices: Vec<String>,
}

pub(crate) fn scope_for(
    document: &Document,
    node: NodeId,
    options: DefinitionPreviewOptions,
    return_selection: Vec<NodeId>,
) -> Result<DefinitionScope, String> {
    options.validate()?;
    let entry = ComponentCatalog::from_document(document)
        .map_err(|e| e.to_string())?
        .entries
        .into_iter()
        .find(|entry| entry.node == node)
        .ok_or("Select an authored component, template, style, theme or resource")?;
    Ok(DefinitionScope {
        node,
        title: entry.display_name,
        kind: entry.kind,
        options,
        return_selection,
    })
}

pub(crate) fn project(
    document: &Document,
    scope: &DefinitionScope,
) -> Result<DefinitionProjection, String> {
    scope.options.validate()?;
    let selected = document
        .ast()
        .node(scope.node)
        .ok_or("The definition was removed")?;
    let mut builder = Builder::new(document, scope.node, &scope.options);
    // Keep resource lookup layered. Flattening visible keys would change the meaning of
    // an outer resource that depends on a key shadowed by the selected inner scope.
    let mut ancestors = Vec::new();
    let mut parent = selected.parent;
    while let Some(id) = parent {
        let node = document
            .ast()
            .node(id)
            .ok_or("Missing definition ancestor")?;
        ancestors.push(id);
        parent = node.parent;
    }
    ancestors.reverse();
    let mut frames = 0;
    for ancestor in ancestors {
        let owner = document.ast().node(ancestor).unwrap();
        let resource_properties = property_children(document, owner, "Resources")?;
        let style_properties = property_children(document, owner, "Styles")?;
        let dictionary_root = owner.id == document.ast().root()
            && owner.local_name() == "ResourceDictionary"
            && owner.namespace_uri.as_deref() == Some(FRAMEWORK);
        if resource_properties.is_empty() && style_properties.is_empty() && !dictionary_root {
            continue;
        }
        builder.source.push_str("<fd:Border>\n");
        frames += 1;
        if dictionary_root || !resource_properties.is_empty() {
            builder.source.push_str("<fd:Border.Resources>\n");
            if dictionary_root {
                builder.fragment(owner.id, None, false)?;
            } else {
                for property in resource_properties {
                    builder.property_content(property)?;
                }
            }
            builder.source.push_str("</fd:Border.Resources>\n");
        }
        if !style_properties.is_empty() {
            builder.source.push_str("<fd:Border.Styles>\n");
            for property in style_properties {
                builder.property_content(property)?;
            }
            builder.source.push_str("</fd:Border.Styles>\n");
        }
    }
    builder.source.push_str(
        "<fd:Border Padding=\"24\" HorizontalAlignment=\"Left\" VerticalAlignment=\"Top\">\n",
    );
    let key = unique_key(document);
    match scope.kind {
        ComponentKind::UserControl | ComponentKind::CustomControl => {
            builder.fragment(selected.id, None, true)?;
        }
        ComponentKind::Style => {
            builder.source.push_str("<fd:Border.Styles>\n");
            builder.fragment(selected.id, None, false)?;
            builder.source.push_str("</fd:Border.Styles>\n");
            builder.sample_control(selected, &scope.options, None)?;
        }
        ComponentKind::ResourceDictionary => {
            builder.source.push_str("<fd:Border.Resources>\n");
            builder.fragment(selected.id, None, false)?;
            builder.source.push_str("</fd:Border.Resources>\n");
            builder.dictionary_gallery(selected.id, &scope.options)?;
        }
        kind => {
            builder.source.push_str("<fd:Border.Resources>\n");
            builder.fragment(selected.id, Some(&key), false)?;
            builder.source.push_str("</fd:Border.Resources>\n");
            match kind {
                ComponentKind::ControlTheme => {
                    builder.sample_control(selected, &scope.options, Some(("Theme", &key)))?
                }
                ComponentKind::ControlTemplate => {
                    builder.sample_control(selected, &scope.options, Some(("Template", &key)))?
                }
                ComponentKind::DataTemplate => {
                    builder.source.push_str(&format!(
                        "<fd:ContentControl Width=\"{}\" Height=\"{}\" Content=\"{}\" ContentTemplate=\"{{fd:StaticResource {key}}}\" />\n",
                        scope.options.width, scope.options.height, escape(&scope.options.sample_content)));
                }
                _ => builder.resource_sample(selected, &key, &scope.options),
            }
        }
    }
    builder.source.push_str("</fd:Border>\n");
    for _ in 0..frames {
        builder.source.push_str("</fd:Border>\n");
    }
    builder.source.push_str("</fd:Border>");
    builder.finish(format!(
        "Editing definition: {}. Changes author the original document.",
        scope.title
    ))
}

struct Builder<'a> {
    original: &'a Document,
    scope: NodeId,
    options: &'a DefinitionPreviewOptions,
    source: String,
    offsets: BTreeMap<usize, NodeId>,
}
impl<'a> Builder<'a> {
    fn new(original: &'a Document, scope: NodeId, options: &'a DefinitionPreviewOptions) -> Self {
        Self {
            original,
            scope,
            options,
            offsets: BTreeMap::new(),
            source: format!(
                "<fd:Border xmlns:fd=\"{FRAMEWORK}\" xmlns:fx=\"{XAML}\" HorizontalAlignment=\"Left\" VerticalAlignment=\"Top\">\n"
            ),
        }
    }
    fn property_content(&mut self, node: &Node) -> Result<(), String> {
        for child in &node.children {
            let child = self
                .original
                .ast()
                .node(*child)
                .ok_or("Missing resource child")?;
            if child.is_element() {
                self.fragment(child.id, None, false)?;
            }
        }
        Ok(())
    }
    fn fragment(&mut self, node: NodeId, key: Option<&str>, visual: bool) -> Result<(), String> {
        let source = component_fragment(self.original, node).map_err(|e| e.to_string())?;
        let mut fragment = Document::new(source).map_err(|e| e.to_string())?;
        let root = fragment.ast().node(fragment.ast().root()).unwrap();
        let mut edits = Vec::new();
        for attr in &root.attributes {
            if attr.namespace_uri.as_deref() == Some(XAML)
                && (attr.name.rsplit(':').next() == Some("Class")
                    || (visual && attr.name.rsplit(':').next() == Some("Key")))
            {
                edits.push(Edit::RemoveAttribute {
                    node: root.id,
                    name: attr.name.clone(),
                });
            }
        }
        if let Some(key) = key {
            let name = root
                .attributes
                .iter()
                .find(|a| {
                    a.namespace_uri.as_deref() == Some(XAML)
                        && a.name.rsplit(':').next() == Some("Key")
                })
                .map(|a| a.name.clone());
            let name = match name {
                Some(name) => name,
                None => {
                    let namespaces = fragment.ast().namespaces(root.id);
                    let prefix = namespaces
                        .iter()
                        .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == XAML)
                        .map(|(prefix, _)| prefix.clone())
                        .unwrap_or_else(|| {
                            let mut prefix = "fx".to_owned();
                            while namespaces.contains_key(&prefix) {
                                prefix.push('_');
                            }
                            edits.push(Edit::SetAttribute {
                                node: root.id,
                                name: format!("xmlns:{prefix}"),
                                value: XAML.into(),
                            });
                            prefix
                        });
                    format!("{prefix}:Key")
                }
            };
            edits.push(Edit::SetAttribute {
                node: root.id,
                name,
                value: key.into(),
            });
        }
        // The native loader resolves TemplateBinding against its ambient target while
        // compiling a deferred template. Moving a resource beneath our Border host
        // would otherwise make that Border the target of every untyped template,
        // including eagerly compiled siblings that are not the selected definition.
        // Pin only the design copy to its authored/inferred target; source and node
        // structure remain unchanged.
        let originals = self
            .original
            .ast()
            .subtree(node)
            .filter(|node| node.is_element())
            .collect::<Vec<_>>();
        let projected = fragment
            .ast()
            .nodes_in_source_order()
            .filter(|node| node.is_element())
            .collect::<Vec<_>>();
        if originals.len() != projected.len() {
            return Err("A definition projection changed its element structure".into());
        }
        for (original, projected) in originals.iter().zip(&projected) {
            if original.local_name() != "ControlTemplate"
                || !crate::definition_manipulation::is_template_owner(self.original, original.id)
                || ferroui_designer_tools::component_property_attribute(
                    self.original,
                    original.id,
                    "TargetType",
                )
                .map_err(|error| error.to_string())?
                .is_some()
                || !property_children(self.original, original, "TargetType")?.is_empty()
            {
                continue;
            }
            let explicit = (original.id == self.scope)
                .then_some(self.options.target_type.as_deref())
                .flatten();
            let (namespace, name) = target_type(self.original, original, explicit)?;
            let namespaces = fragment.ast().namespaces(projected.id);
            let prefix = namespaces
                .iter()
                .find(|(prefix, uri)| !prefix.is_empty() && **uri == namespace)
                .map(|(prefix, _)| prefix.clone())
                .unwrap_or_else(|| {
                    let mut prefix = "fd_target".to_owned();
                    while namespaces.contains_key(&prefix) {
                        prefix.push('_');
                    }
                    edits.push(Edit::SetAttribute {
                        node: projected.id,
                        name: format!("xmlns:{prefix}"),
                        value: namespace,
                    });
                    prefix
                });
            edits.push(Edit::SetAttribute {
                node: projected.id,
                name: "TargetType".into(),
                value: format!("{prefix}:{name}"),
            });
        }
        if !edits.is_empty() {
            fragment
                .apply("Preview projection", edits)
                .map_err(|e| e.to_string())?;
        }
        let projected = fragment
            .ast()
            .nodes_in_source_order()
            .filter(|n| n.is_element())
            .collect::<Vec<_>>();
        if originals.len() != projected.len() {
            return Err("A definition projection changed its element structure".into());
        }
        let start = self.source.len();
        for (original, projected) in originals.into_iter().zip(projected) {
            if original.id == self.scope
                || self
                    .original
                    .ast()
                    .is_descendant_of(original.id, self.scope)
            {
                self.offsets
                    .insert(start + projected.span.start, original.id);
            }
        }
        self.source.push_str(fragment.source());
        self.source.push('\n');
        Ok(())
    }
    fn sample_control(
        &mut self,
        node: &Node,
        options: &DefinitionPreviewOptions,
        reference: Option<(&str, &str)>,
    ) -> Result<(), String> {
        let (namespace, name) = target_type(self.original, node, options.target_type.as_deref())?;
        let content_property = match name.as_str() {
            "TextBox" | "TextBlock" | "SelectableTextBlock" => "Text",
            "Button" | "ToggleButton" | "CheckBox" | "RadioButton" | "ContentControl"
            | "UserControl" | "Label" => "Content",
            _ => "",
        };
        let content = if content_property.is_empty() {
            String::new()
        } else {
            format!(
                " {content_property}=\"{}\"",
                escape(&options.sample_content)
            )
        };
        let reference = reference.map_or_else(String::new, |(property, key)| {
            format!(" {property}=\"{{fd:StaticResource {key}}}\"")
        });
        self.source.push_str(&format!(
            "<dt:{name} xmlns:dt=\"{}\" Width=\"{}\" Height=\"{}\"{content}{reference} />\n",
            escape(&namespace),
            options.width,
            options.height
        ));
        Ok(())
    }
    fn resource_sample(&mut self, node: &Node, key: &str, options: &DefinitionPreviewOptions) {
        let dimensions = format!("Width=\"{}\" Height=\"{}\"", options.width, options.height);
        let reference = format!("{{fd:StaticResource {key}}}");
        let sample = match node.local_name() {
            name if name.ends_with("Brush") => {
                format!("<fd:Border {dimensions} Background=\"{reference}\" />")
            }
            "Color" => format!(
                "<fd:Border {dimensions}><fd:Border.Background><fd:SolidColorBrush Color=\"{reference}\" /></fd:Border.Background></fd:Border>"
            ),
            "Geometry" | "StreamGeometry" | "PathGeometry" | "RectangleGeometry"
            | "EllipseGeometry" => format!(
                "<fd:Path {dimensions} Data=\"{reference}\" Fill=\"#FF628DD7\" Stretch=\"Uniform\" />"
            ),
            "Bitmap" | "BitmapSource" | "DrawingImage" => {
                format!("<fd:Image {dimensions} Source=\"{reference}\" Stretch=\"Uniform\" />")
            }
            "Thickness" => format!(
                "<fd:Border {dimensions} Padding=\"{reference}\" Background=\"#FFE1E9F7\"><fd:Border Background=\"#FF93AFE0\" /></fd:Border>"
            ),
            "CornerRadius" => format!(
                "<fd:Border {dimensions} CornerRadius=\"{reference}\" Background=\"#FF93AFE0\" />"
            ),
            "FontFamily" => format!(
                "<fd:TextBlock {dimensions} FontFamily=\"{reference}\" Text=\"{}\" FontSize=\"24\" TextWrapping=\"Wrap\" />",
                escape(&options.sample_content)
            ),
            _ => format!(
                "<fd:TextBlock {dimensions} Text=\"{reference}\" FontSize=\"22\" TextWrapping=\"Wrap\" />"
            ),
        };
        self.source.push_str(&sample);
        self.source.push('\n');
    }
    fn dictionary_gallery(
        &mut self,
        dictionary: NodeId,
        options: &DefinitionPreviewOptions,
    ) -> Result<(), String> {
        let entries =
            ferroui_designer_tools::visible_component_resources(self.original, dictionary)
                .map_err(|e| e.to_string())?;
        self.source.push_str("<fd:StackPanel Spacing=\"16\">\n");
        let mut count = 0;
        for entry in entries
            .into_iter()
            .filter(|entry| self.original.ast().is_descendant_of(entry.node, dictionary))
        {
            let Some(key) = entry.key.filter(|key| safe_key(key)) else {
                continue;
            };
            if !matches!(
                entry.kind,
                ComponentKind::ControlTheme
                    | ComponentKind::ControlTemplate
                    | ComponentKind::DataTemplate
                    | ComponentKind::Resource
            ) {
                continue;
            }
            if count == 16 {
                break;
            }
            count += 1;
            let node = self.original.ast().node(entry.node).unwrap();
            self.source.push_str(&format!(
                "<fd:StackPanel Spacing=\"6\"><fd:TextBlock Text=\"{}\" FontSize=\"12\" />\n",
                escape(&key)
            ));
            let mut options = options.clone();
            options.height = options.height.min(100.0);
            match entry.kind {
                ComponentKind::ControlTheme => self.sample_control(node, &options, Some(("Theme", &key)))?,
                ComponentKind::ControlTemplate => self.sample_control(node, &options, Some(("Template", &key)))?,
                ComponentKind::DataTemplate => self.source.push_str(&format!("<fd:ContentControl Width=\"{}\" Content=\"{}\" ContentTemplate=\"{{fd:StaticResource {key}}}\" />\n", options.width, escape(&options.sample_content))),
                _ => self.resource_sample(node, &key, &options),
            }
            self.source.push_str("</fd:StackPanel>\n");
        }
        if count == 0 {
            self.source.push_str("<fd:TextBlock Text=\"Select a resource in the Library to inspect its value or edit its definition.\" TextWrapping=\"Wrap\" Width=\"320\" />\n");
        }
        self.source.push_str("</fd:StackPanel>\n");
        Ok(())
    }
    fn finish(self, notice: String) -> Result<DefinitionProjection, String> {
        let document = Document::new(self.source).map_err(|e| e.to_string())?;
        let identities = document
            .ast()
            .elements()
            .filter_map(|node| {
                self.offsets
                    .get(&node.span.start)
                    .map(|original| (node.id, *original))
            })
            .collect();
        Ok(DefinitionProjection {
            document,
            identities,
            notices: vec![notice],
        })
    }
}

fn property_children<'a>(
    document: &'a Document,
    owner: &Node,
    property: &str,
) -> Result<Vec<&'a Node>, String> {
    ferroui_designer_tools::component_property_elements(document, owner.id, property)
        .map(|nodes| {
            nodes
                .into_iter()
                .filter_map(|id| document.ast().node(id))
                .collect()
        })
        .map_err(|error| error.to_string())
}
fn target_type(
    document: &Document,
    node: &Node,
    explicit: Option<&str>,
) -> Result<(String, String), String> {
    let mut owner = Some(node);
    let mut value = explicit
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
    let mut scope = node.id;
    while value.is_none() {
        let Some(current) = owner else {
            break;
        };
        value = ferroui_designer_tools::component_property_attribute(
            document,
            current.id,
            "TargetType",
        )
        .map_err(|error| error.to_string())?
        .map(|attribute| attribute.value.clone());
        if value.is_none() && current.local_name() == "Style" {
            value = ferroui_designer_tools::component_property_attribute(
                document, current.id, "Selector",
            )
            .map_err(|error| error.to_string())?
            .map(|attribute| attribute.value.as_str())
            .and_then(|selector| {
                selector
                    .split(|c: char| {
                        c == '.' || c == '#' || c == ':' || c.is_whitespace() || c == '>'
                    })
                    .next()
            })
            .filter(|v| !v.is_empty() && *v != "^")
            .map(|value| value.replace('|', ":"));
        }
        if value.is_some() {
            scope = current.id;
            break;
        }
        // Only an inline Template property establishes a containing control's type.
        // A Grid that merely owns Resources is not the target of a keyed template.
        if current.kind == NodeKind::PropertyElement
            && current.local_name().rsplit('.').next() == Some("Template")
        {
            if let Some(control) = current.parent.and_then(|id| document.ast().node(id)) {
                if control.kind == NodeKind::Element
                    && !matches!(control.local_name(), "Setter" | "Style" | "ControlTheme")
                {
                    return Ok((
                        control
                            .namespace_uri
                            .clone()
                            .unwrap_or_else(|| FRAMEWORK.into()),
                        control.local_name().into(),
                    ));
                }
            }
        }
        if current.local_name() == "ResourceDictionary"
            || (current.kind == NodeKind::PropertyElement
                && current.local_name().rsplit('.').next() == Some("Resources"))
        {
            break;
        }
        owner = current.parent.and_then(|id| document.ast().node(id));
    }
    let Some(value) = value else {
        if node.local_name() == "ControlTemplate"
            && crate::definition_manipulation::is_template_owner(document, node.id)
        {
            let mut targets = BTreeSet::new();
            for reference in
                ferroui_designer_tools::component_resource_references(document, node.id)
                    .map_err(|error| error.to_string())?
            {
                let Some(owner) = document.ast().node(reference.node) else {
                    continue;
                };
                if owner.kind != NodeKind::Element {
                    continue;
                }
                let template = ferroui_designer_tools::component_property_attribute(
                    document, owner.id, "Template",
                )
                .map_err(|error| error.to_string())?;
                if template.is_some_and(|attribute| attribute.name == reference.property) {
                    targets.insert((
                        owner
                            .namespace_uri
                            .clone()
                            .unwrap_or_else(|| FRAMEWORK.into()),
                        owner.local_name().to_owned(),
                    ));
                }
            }
            if targets.len() == 1 {
                return Ok(targets.into_iter().next().unwrap());
            }
        }
        return Ok((FRAMEWORK.into(), "Button".into()));
    };
    let value = match MarkupValue::parse(value.trim())? {
        MarkupValue::Literal(value) => value,
        MarkupValue::Extension(extension)
            if extension.name.rsplit(':').next() == Some("Type")
                && document
                    .ast()
                    .namespaces(scope)
                    .get(
                        extension
                            .name
                            .split_once(':')
                            .map_or("", |(prefix, _)| prefix),
                    )
                    .is_some_and(|uri| uri == XAML) =>
        {
            extension
                .arguments
                .first()
                .and_then(|a| {
                    if let MarkupValue::Literal(value) = &a.value {
                        Some(value.clone())
                    } else {
                        None
                    }
                })
                .ok_or("TargetType requires a concrete registered XAML type")?
        }
        _ => return Err("Choose a concrete preview target type for this definition".into()),
    };
    let (prefix, name) = value.split_once(':').unwrap_or(("", &value));
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
    {
        return Err("Preview target must be a registered XAML type name".into());
    }
    let namespace = document
        .ast()
        .namespaces(scope)
        .get(prefix)
        .cloned()
        .ok_or_else(|| format!("Unknown preview type namespace prefix {prefix}"))?;
    Ok((namespace, name.into()))
}
fn unique_key(document: &Document) -> String {
    let keys = document
        .ast()
        .elements()
        .flat_map(|node| node.attributes.iter())
        .filter(|a| {
            a.namespace_uri.as_deref() == Some(XAML) && a.name.rsplit(':').next() == Some("Key")
        })
        .map(|a| a.value.as_str())
        .collect::<BTreeSet<_>>();
    let mut key = "__FerroDesignerDefinition".to_owned();
    while keys.contains(key.as_str()) {
        key.push('_');
    }
    key
}
fn safe_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "definition_projection_tests.rs"]
mod tests;
