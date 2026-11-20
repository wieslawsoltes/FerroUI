//! Read and edit one XAML data-template slot using its resolved source declaration.
use super::*;
use ferroui_designer_core::SourceSpan;

#[path = "item_template_edits.rs"]
mod edits;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemTemplateMember {
    ItemTemplate,
    SelectionBoxItemTemplate,
    MenuItemTemplate,
    CellTemplate,
    CellEditingTemplate,
}

impl ItemTemplateMember {
    pub const ALL: [Self; 5] = [
        Self::ItemTemplate,
        Self::SelectionBoxItemTemplate,
        Self::MenuItemTemplate,
        Self::CellTemplate,
        Self::CellEditingTemplate,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::ItemTemplate => "ItemTemplate",
            Self::SelectionBoxItemTemplate => "SelectionBoxItemTemplate",
            Self::MenuItemTemplate => "MenuItemTemplate",
            Self::CellTemplate => "CellTemplate",
            Self::CellEditingTemplate => "CellEditingTemplate",
        }
    }
}

/// The host resolves inherited member identities using its XAML schema before asking
/// for a slot. Retain the exact attribute or property element for source edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemTemplateDeclaration {
    Unset,
    Attribute(String),
    Element(NodeId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ItemTemplateValue {
    Unset,
    Inline {
        definition: NodeId,
    },
    Resource {
        key: String,
        dynamic: bool,
        definition: Option<NodeId>,
    },
    Expression,
}

impl ItemTemplateValue {
    pub fn definition(&self) -> Option<NodeId> {
        match self {
            Self::Inline { definition } => Some(*definition),
            Self::Resource { definition, .. } => *definition,
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemTemplateSlot {
    pub owner: NodeId,
    pub property: ItemTemplateMember,
    pub value: ItemTemplateValue,
    pub span: Option<SourceSpan>,
    pub reason: Option<String>,
    pub can_reset: bool,
    #[serde(skip)]
    declaration: ItemTemplateDeclaration,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemTemplateResource {
    pub node: NodeId,
    pub key: String,
    pub target_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ItemTemplateChange {
    CreateInline,
    AssignResource { resource: NodeId },
    AssignIncludedResource { key: String },
    CopyInline,
    Reset,
}

impl ItemTemplateChange {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CreateInline => "Create inline item template",
            Self::AssignResource { .. } | Self::AssignIncludedResource { .. } => "Assign item template resource",
            Self::CopyInline => "Make inline item template copy",
            Self::Reset => "Reset item template",
        }
    }
}

impl ItemTemplateSlot {
    pub fn read(
        document: &Document,
        owner: NodeId,
        property: ItemTemplateMember,
        declaration: ItemTemplateDeclaration,
        context: Option<&ComponentResourceContext>,
    ) -> Result<Self> {
        valid_document(document)?;
        let node = element(document, owner)?;
        let member = |name: &str| name.rsplit([':', '.']).next() == Some(property.name());
        let (span, value) = match &declaration {
            ItemTemplateDeclaration::Unset => (None, ItemTemplateValue::Unset),
            ItemTemplateDeclaration::Attribute(name) => {
                let attribute = node
                    .attribute_info(name)
                    .filter(|attribute| member(&attribute.name))
                    .ok_or_else(|| ToolError("The template attribute changed".into()))?;
                let value = attribute_resource(document, owner, attribute)
                    .map(|(key, dynamic)| resource_value(document, owner, key, dynamic, context))
                    .transpose()?
                    .unwrap_or(ItemTemplateValue::Expression);
                (Some(attribute.value_span), value)
            }
            ItemTemplateDeclaration::Element(id) => {
                let property_node = element(document, *id)?;
                require(
                    property_node.kind == NodeKind::PropertyElement
                        && property_node.parent == Some(owner)
                        && member(&property_node.name),
                    "The template property element changed",
                )?;
                let value = object_value(document, owner, property_node, context)?;
                (Some(property_node.name_span), value)
            }
        };
        let reason = matches!(value, ItemTemplateValue::Expression)
            .then(|| "Edit the authored template expression in source.".into());
        let can_reset = !matches!(
            value,
            ItemTemplateValue::Unset | ItemTemplateValue::Expression
        );
        Ok(Self {
            owner,
            property,
            value,
            span,
            reason,
            can_reset,
            declaration,
        })
    }

    pub fn can_copy_inline(&self, document: &Document) -> bool {
        matches!(
            &self.value,
            ItemTemplateValue::Resource { key, definition: Some(id), .. }
                if resolve_component_resource(document, self.owner, key) == Some(*id)
        )
    }
}

pub fn item_template_resources(
    document: &Document,
    owner: NodeId,
) -> Result<Vec<ItemTemplateResource>> {
    Ok(visible_component_resources(document, owner)?
        .into_iter()
        .filter(|entry| document.ast().node(entry.node).is_some_and(is_template))
        .filter_map(|entry| {
            Some(ItemTemplateResource {
                node: entry.node,
                key: entry.key?,
                target_type: entry.target_type,
            })
        })
        .collect())
}

pub fn keyed_item_template(node: &Node) -> Option<String> {
    is_template(node).then(|| literal_key(node)).flatten()
}

fn is_template(node: &Node) -> bool {
    framework(node) && matches!(node.local_name(), "DataTemplate" | "TreeDataTemplate")
}

fn attribute_resource(
    document: &Document,
    owner: NodeId,
    attribute: &ferroui_designer_core::Attribute,
) -> Option<(String, bool)> {
    let MarkupValue::Extension(extension) = attribute.markup().ok()? else {
        return None;
    };
    let dynamic = references::extension_kind(document, owner, &extension.name)?;
    let argument = &extension.arguments[references::resource_argument(&extension)?];
    let MarkupValue::Literal(key) = &argument.value else {
        return None;
    };
    Some((key.clone(), dynamic))
}

fn object_value(
    document: &Document,
    owner: NodeId,
    property: &Node,
    context: Option<&ComponentResourceContext>,
) -> Result<ItemTemplateValue> {
    let children = document.ast().element_children(property.id);
    let [child] = children.as_slice() else {
        return Ok(ItemTemplateValue::Expression);
    };
    let node = element(document, *child)?;
    if is_template(node) {
        return Ok(ItemTemplateValue::Inline {
            definition: node.id,
        });
    }
    let dynamic = match node.local_name() {
        "StaticResource" | "StaticResourceExtension" if framework(node) => false,
        "DynamicResource" | "DynamicResourceExtension" if framework(node) => true,
        _ => return Ok(ItemTemplateValue::Expression),
    };
    match references::object_resource_key(document, node)? {
        Some((_, key)) => resource_value(document, owner, key, dynamic, context),
        None => Ok(ItemTemplateValue::Expression),
    }
}

fn resource_value(
    document: &Document,
    owner: NodeId,
    key: String,
    dynamic: bool,
    context: Option<&ComponentResourceContext>,
) -> Result<ItemTemplateValue> {
    let definition = component_resource_resolution(document, owner, &key, context)?
        .resolved()
        .filter(|id| document.ast().node(*id).is_some_and(is_template));
    Ok(ItemTemplateValue::Resource {
        key,
        dynamic,
        definition,
    })
}

#[cfg(test)]
#[path = "item_template_tests.rs"]
mod tests;
