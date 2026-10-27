//! Source-only component operations. Each public plan validates its complete candidate.
use super::*;
use ferroui_designer_core::SourceSpan;

pub fn plan_component_action(document: &Document, action: &ComponentAction) -> Result<Vec<Edit>> {
    valid_document(document)?;
    let edits = match action {
        ComponentAction::CreateResource { owner, key, source } => {
            create_resource(document, *owner, key, source)?
        }
        ComponentAction::DuplicateResource { resource, key } => {
            let owner = resource_owner(document, *resource).ok_or_else(|| {
                ToolError("Only authored dictionary entries can be duplicated as resources".into())
            })?;
            create_resource(
                document,
                owner,
                key,
                &component_fragment(document, *resource)?,
            )?
        }
        ComponentAction::RenameResource { resource, key } => {
            references::rename_resource(document, *resource, key)?
        }
        ComponentAction::RemoveResource { resource } => {
            require(
                resource_owner(document, *resource).is_some(),
                "Select an authored resource entry",
            )?;
            require(
                component_resource_references(document, *resource)?.is_empty(),
                "The resource is still referenced; update or reset its uses before deleting it",
            )?;
            vec![Edit::RemoveNode { node: *resource }]
        }
        ComponentAction::ApplyResource {
            resource,
            targets,
            property,
            dynamic,
        } => apply_resource(document, *resource, targets, property, *dynamic)?,
        ComponentAction::SetProperty {
            node,
            property,
            value,
        } => scalar_property(document, *node, property, value.as_deref())?,
        ComponentAction::SetObjectProperty {
            node,
            property,
            source,
        } => object_properties(document, &[*node], property, source.as_deref())?,
        ComponentAction::SetSetter {
            owner,
            property,
            value,
        } => setter(document, *owner, property, value.as_deref(), false)?,
        ComponentAction::SetSetterObject {
            owner,
            property,
            source,
        } => setter(document, *owner, property, source.as_deref(), true)?,
        ComponentAction::SetResourceValue { resource, value } => {
            resource_value(document, *resource, value)?
        }
        ComponentAction::ReplaceDefinition { node, source } => {
            replace_definition(document, *node, source)?
        }
        ComponentAction::CreateTheme {
            owner,
            key,
            target_type,
            based_on,
        } => {
            type_name(target_type)?;
            let (ui, xaml, declarations) = creation_namespaces(document, *owner);
            let based = if let Some(base) = based_on {
                let base_node = element(document, *base)?;
                require(
                    framework(base_node) && base_node.local_name() == "ControlTheme",
                    "BasedOn requires a ControlTheme",
                )?;
                let base_key = literal_key(base_node)
                    .ok_or_else(|| ToolError("BasedOn requires an explicit string key".into()))?;
                require(
                    resolve_component_resource(document, *owner, &base_key) == Some(*base),
                    "The base theme is not visible in this resource scope",
                )?;
                format!(
                    " BasedOn=\"{}\"",
                    escape(&references::reference_expression(
                        document, *owner, &base_key, false
                    )?)
                )
            } else {
                String::new()
            };
            let target = type_expression(target_type, &xaml);
            create_resource(
                document,
                *owner,
                key,
                &format!(
                    "<{ui}:ControlTheme{declarations} TargetType=\"{}\"{based} />",
                    escape(&target)
                ),
            )?
        }
        ComponentAction::CreateTemplate {
            owner,
            key,
            target_type,
            content,
            data_template,
        } => {
            let (ui, xaml, declarations) = creation_namespaces(document, *owner);
            let name = if *data_template {
                "DataTemplate"
            } else {
                "ControlTemplate"
            };
            let property = if *data_template {
                "DataType"
            } else {
                "TargetType"
            };
            let target = if let Some(target) = target_type {
                type_name(target)?;
                format!(
                    " {property}=\"{}\"",
                    escape(&type_expression(target, &xaml))
                )
            } else {
                String::new()
            };
            let fragment = format!("<{ui}:{name}{declarations}{target}>{content}</{ui}:{name}>");
            let parsed = context_fragment(document, *owner, &fragment)?;
            let root = parsed.ast().element_children(parsed.ast().root())[0];
            let roots = parsed
                .ast()
                .element_children(root)
                .into_iter()
                .filter(|id| {
                    parsed
                        .ast()
                        .node(*id)
                        .is_some_and(|node| node.kind == NodeKind::Element)
                })
                .count();
            require(roots == 1, "A template needs exactly one visual root")?;
            create_resource(document, *owner, key, &fragment)?
        }
        ComponentAction::CopyTemplate {
            template,
            owner,
            key,
        } => {
            let node = element(document, *template)?;
            let template = if framework(node)
                && matches!(
                    node.local_name(),
                    "ControlTemplate" | "DataTemplate" | "TreeDataTemplate"
                ) {
                *template
            } else {
                let templates = document
                    .ast()
                    .subtree(*template)
                    .filter(|node| {
                        framework(node)
                            && matches!(
                                node.local_name(),
                                "ControlTemplate" | "DataTemplate" | "TreeDataTemplate"
                            )
                    })
                    .collect::<Vec<_>>();
                require(
                    templates.len() == 1,
                    "Select one authored template to make an editable copy",
                )?;
                templates[0].id
            };
            let edits = create_resource(
                document,
                *owner,
                key,
                &component_fragment(document, template)?,
            )?;
            let mut candidate = document.clone();
            candidate
                .apply("Validate template copy", edits.clone())
                .map_err(error)?;
            let container = resource_container(&candidate, *owner)?
                .ok_or_else(|| ToolError("Missing copied template dictionary".into()))?;
            let copied = candidate
                .ast()
                .element_children(container)
                .into_iter()
                .find(|id| {
                    candidate
                        .ast()
                        .node(*id)
                        .is_some_and(|node| literal_key(node).as_deref() == Some(key))
                })
                .ok_or_else(|| ToolError("Missing copied template".into()))?;
            references::validate_copied_references(document, template, &candidate, copied)?;
            edits
        }
        ComponentAction::ExtractProperty {
            node,
            property,
            owner,
            key,
        } => extract_property(document, *node, property, *owner, key)?,
    };
    validated(document, edits)
}

/// Replace/reset a single object or collection property across a selection atomically.
/// Input fragments may use namespaces inherited from the target, or declare their own.
pub fn plan_property_object(
    document: &Document,
    nodes: &[NodeId],
    property: &str,
    source: Option<&str>,
) -> Result<Vec<Edit>> {
    valid_document(document)?;
    validated(
        document,
        object_properties(document, nodes, property, source)?,
    )
}

pub(crate) fn validate_key(key: &str) -> Result<()> {
    require(
        !key.trim().is_empty()
            && key.len() <= 4096
            && key.chars().all(|c| {
                matches!(c, '\t' | '\n' | '\r') || c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}'
            }),
        "Resource keys must be nonempty XML text",
    )
}

pub(crate) fn ensure_key_available(
    document: &Document,
    owner: NodeId,
    key: &str,
    except: Option<NodeId>,
) -> Result<()> {
    validate_key(key)?;
    let container = resource_container(document, owner)?;
    let children = container.map_or_else(Vec::new, |id| document.ast().element_children(id));
    require(
        !children
            .into_iter()
            .filter(|id| Some(*id) != except)
            .filter_map(|id| document.ast().node(id))
            .any(|node| literal_key(node).as_deref() == Some(key)),
        "A resource with this key already exists in this dictionary",
    )
}

fn resource_container(document: &Document, owner: NodeId) -> Result<Option<NodeId>> {
    let node = element(document, owner)?;
    if framework(node) && node.local_name() == "ResourceDictionary" {
        require(
            !external_dictionary(document, node),
            "Open the external dictionary before editing its resources",
        )?;
        return Ok(Some(owner));
    }
    if node.kind == NodeKind::PropertyElement && node.local_name().ends_with(".Resources") {
        let owner = node
            .parent
            .and_then(|id| document.ast().node(id))
            .ok_or_else(|| ToolError("The resource property has no owner".into()))?;
        require(
            property_elements(document, owner, "Resources")?
                .iter()
                .any(|property| property.id == node.id),
            "Select the owner's Resources property or a ResourceDictionary",
        )?;
        return dictionary_container(document, node);
    }
    let resources = property_elements(document, node, "Resources")?;
    if let Some(resources) = resources.first() {
        dictionary_container(document, resources)
    } else {
        Ok(None)
    }
}

fn dictionary_container(document: &Document, node: &Node) -> Result<Option<NodeId>> {
    let children = document.ast().element_children(node.id);
    let dictionaries = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| {
            framework(child)
                && child.local_name() == "ResourceDictionary"
                && key_attribute(child).is_none()
        })
        .collect::<Vec<_>>();
    require(
        dictionaries.len() <= 1,
        "The Resources property has more than one dictionary",
    )?;
    if let Some(dictionary) = dictionaries.first() {
        require(
            children.len() == 1,
            "The Resources property mixes a dictionary object with resource entries",
        )?;
        require(
            !external_dictionary(document, dictionary),
            "Open the external dictionary before editing its resources",
        )?;
        Ok(Some(dictionary.id))
    } else {
        Ok(Some(node.id))
    }
}

fn create_resource(
    document: &Document,
    owner: NodeId,
    key: &str,
    source: &str,
) -> Result<Vec<Edit>> {
    ensure_key_available(document, owner, key, None)?;
    let fragment = keyed_fragment(document, owner, source, key)?;
    let container = resource_container(document, owner)?;
    if let Some(container) = container {
        Ok(vec![Edit::InsertChild {
            parent: container,
            index: document.ast().element_children(container).len(),
            source: fragment,
        }])
    } else {
        let node = element(document, owner)?;
        let name = format!("{}.Resources", node.name);
        Ok(vec![Edit::InsertChild {
            parent: owner,
            index: 0,
            source: format!("<{name}>\n{fragment}\n</{name}>"),
        }])
    }
}

fn keyed_fragment(document: &Document, owner: NodeId, source: &str, key: &str) -> Result<String> {
    let mut candidate = context_fragment(document, owner, source)?;
    let roots = candidate.ast().element_children(candidate.ast().root());
    require(
        roots.len() == 1,
        "A resource requires exactly one object element",
    )?;
    let root = roots[0];
    require(
        candidate
            .ast()
            .node(root)
            .is_some_and(|node| node.kind == NodeKind::Element),
        "A resource cannot be a property element",
    )?;
    let element = candidate.ast().node(root).expect("root");
    let mut edits = Vec::new();
    let name = if let Some(attribute) = key_attribute(element) {
        attribute.name.clone()
    } else {
        let namespaces = candidate.ast().namespaces(root);
        let prefix = namespaces
            .iter()
            .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == XAML)
            .map(|(prefix, _)| prefix.clone())
            .unwrap_or_else(|| fresh_prefix(&namespaces, "x"));
        if namespaces.get(&prefix).map(String::as_str) != Some(XAML) {
            edits.push(Edit::SetAttribute {
                node: root,
                name: format!("xmlns:{prefix}"),
                value: XAML.into(),
            });
        }
        format!("{prefix}:Key")
    };
    edits.push(Edit::SetAttribute {
        node: root,
        name,
        value: references::escaped_literal(key),
    });
    candidate
        .apply("Set resource identity", edits)
        .map_err(error)?;
    component_fragment(&candidate, root)
}

fn context_fragment(document: &Document, node: NodeId, source: &str) -> Result<Document> {
    element(document, node)?;
    let mut declarations = String::new();
    for (prefix, uri) in document.ast().namespaces(node) {
        if prefix == "xml" {
            continue;
        }
        let name = if prefix.is_empty() {
            "xmlns".into()
        } else {
            format!("xmlns:{prefix}")
        };
        declarations.push_str(&format!(" {name}=\"{}\"", escape(&uri)));
    }
    let candidate = Document::new(format!(
        "<ComponentFragment{declarations}>{source}</ComponentFragment>"
    ))
    .map_err(error)?;
    require(
        candidate
            .ast()
            .node(candidate.ast().root())
            .expect("root")
            .children
            .iter()
            .filter_map(|id| candidate.ast().node(*id))
            .all(|node| {
                node.is_element()
                    || !matches!(node.kind, NodeKind::Text | NodeKind::CData)
                    || node.text.as_deref().unwrap_or_default().trim().is_empty()
            }),
        "An object fragment cannot contain unattached text",
    )?;
    Ok(candidate)
}

fn scalar_property(
    document: &Document,
    node: NodeId,
    property: &str,
    value: Option<&str>,
) -> Result<Vec<Edit>> {
    let element = element(document, node)?;
    property_name(property)?;
    require(
        !(property.starts_with("xmlns")
            || property.rsplit(':').next() == Some("Key") && property.contains(':')),
        "Use resource rename to change a key; namespace declarations are edited in source",
    )?;
    let attributes = attributes_for(document, element, property)?;
    let objects = property_elements(document, element, property)?;
    let implicit = implicit_property_children(document, element, property);
    let mut edits = Vec::new();
    for object in objects {
        edits.push(Edit::RemoveNode { node: object.id });
    }
    for object in implicit {
        edits.push(Edit::RemoveNode { node: object.id });
    }
    if let Some(value) = value {
        if value.starts_with('{') {
            MarkupValue::parse(value).map_err(ToolError)?;
        }
        let name = attributes
            .first()
            .map_or(property, |attribute| attribute.name.as_str());
        if attributes
            .first()
            .is_none_or(|attribute| attribute.value != value)
        {
            edits.push(Edit::SetAttribute {
                node,
                name: name.into(),
                value: value.into(),
            });
        }
    } else {
        for attribute in attributes {
            edits.push(Edit::RemoveAttribute {
                node,
                name: attribute.name.clone(),
            });
        }
    }
    Ok(edits)
}

fn object_properties(
    document: &Document,
    nodes: &[NodeId],
    property: &str,
    source: Option<&str>,
) -> Result<Vec<Edit>> {
    property_name(property)?;
    require(
        !property.starts_with("xmlns") && !property.contains(":Key"),
        "Identity directives do not accept object values",
    )?;
    let mut edits = Vec::new();
    for node in unique_targets(document, nodes)? {
        let element = element(document, node)?;
        let attributes = attributes_for(document, element, property)?;
        let objects = property_elements(document, element, property)?;
        let implicit = implicit_property_children(document, element, property);
        let index = objects
            .first()
            .copied()
            .or_else(|| implicit.first().copied())
            .and_then(|property| {
                document
                    .ast()
                    .element_children(node)
                    .iter()
                    .position(|id| *id == property.id)
            })
            .unwrap_or(0);
        let name = objects
            .first()
            .map(|property| property.name.clone())
            .unwrap_or_else(|| {
                if property.contains('.') {
                    property.into()
                } else {
                    format!("{}.{}", element.name, property)
                }
            });
        for attribute in attributes {
            edits.push(Edit::RemoveAttribute {
                node,
                name: attribute.name.clone(),
            });
        }
        for object in objects {
            edits.push(Edit::RemoveNode { node: object.id });
        }
        for object in implicit {
            edits.push(Edit::RemoveNode { node: object.id });
        }
        if let Some(source) = source {
            let candidate = context_fragment(document, node, source)?;
            require(
                !candidate
                    .ast()
                    .element_children(candidate.ast().root())
                    .is_empty(),
                "An object property needs at least one object element",
            )?;
            edits.push(Edit::InsertChild {
                parent: node,
                index,
                source: format!("<{name}>{source}</{name}>"),
            });
        }
    }
    Ok(edits)
}

fn apply_resource(
    document: &Document,
    resource: NodeId,
    targets: &[NodeId],
    property: &str,
    dynamic: bool,
) -> Result<Vec<Edit>> {
    let resource_node = element(document, resource)?;
    let key = literal_key(resource_node)
        .ok_or_else(|| ToolError("Applying a resource requires an explicit string key".into()))?;
    let mut edits = Vec::new();
    for target in unique_targets(document, targets)? {
        require(
            resolve_component_resource(document, target, &key) == Some(resource),
            "The selected resource is hidden, ambiguous or outside the target's scope",
        )?;
        // Resource application is explicit replacement, including an existing object graph.
        edits.extend(scalar_property(
            document,
            target,
            property,
            Some(&references::reference_expression(
                document, target, &key, dynamic,
            )?),
        )?);
    }
    Ok(edits)
}

fn setters(document: &Document, owner: NodeId) -> Result<(NodeId, Vec<NodeId>)> {
    let owner = element(document, owner)?;
    require(
        framework(owner) && matches!(owner.local_name(), "ControlTheme" | "Style"),
        "Setters belong to an authored ControlTheme or Style",
    )?;
    let containers = property_elements(document, owner, "Setters")?;
    require(
        containers.is_empty()
            || !document
                .ast()
                .element_children(owner.id)
                .into_iter()
                .any(|id| {
                    document
                        .ast()
                        .node(id)
                        .is_some_and(|node| framework(node) && node.local_name() == "Setter")
                }),
        "The definition has both implicit and explicit setters; resolve the ambiguous containers in source",
    )?;
    let container = containers.first().map_or(owner.id, |node| node.id);
    Ok((
        container,
        document
            .ast()
            .element_children(container)
            .into_iter()
            .filter(|id| {
                document
                    .ast()
                    .node(*id)
                    .is_some_and(|node| framework(node) && node.local_name() == "Setter")
            })
            .collect(),
    ))
}

fn setter(
    document: &Document,
    owner: NodeId,
    property: &str,
    value: Option<&str>,
    object: bool,
) -> Result<Vec<Edit>> {
    property_name(property)?;
    let (container, setters) = setters(document, owner)?;
    let requested = setter_property_identity(document, owner, owner, property)?;
    let mut matching = Vec::new();
    for id in setters {
        if let Some(attribute) = component_property_attribute(document, id, "Property")? {
            let existing = setter_property_identity(document, owner, id, &attribute.value)?;
            if requested.member == existing.member
                && requested.owner == existing.owner
                && match requested.owner.as_deref() {
                    Some(owner) => same_owner_namespace(
                        owner,
                        requested.namespace.as_deref(),
                        existing.namespace.as_deref(),
                    ),
                    None => requested.namespace == existing.namespace,
                }
            {
                matching.push(id);
            }
        }
    }
    require(
        matching.len() <= 1,
        "Multiple setters target this property; edit the ambiguous declarations in source",
    )?;
    if let Some(existing) = matching.first() {
        if value.is_none() {
            return Ok(vec![Edit::RemoveNode { node: *existing }]);
        }
        return if object {
            object_properties(document, &[*existing], "Value", value)
        } else {
            scalar_property(document, *existing, "Value", value)
        };
    }
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let (ui, _, declarations) = creation_namespaces(document, owner);
    let source = if object {
        let candidate = context_fragment(document, owner, value)?;
        require(
            !candidate
                .ast()
                .element_children(candidate.ast().root())
                .is_empty(),
            "A setter object needs an element",
        )?;
        format!(
            "<{ui}:Setter{declarations} Property=\"{}\"><{ui}:Setter.Value>{value}</{ui}:Setter.Value></{ui}:Setter>",
            escape(property)
        )
    } else {
        if value.starts_with('{') {
            MarkupValue::parse(value).map_err(ToolError)?;
        }
        format!(
            "<{ui}:Setter{declarations} Property=\"{}\" Value=\"{}\" />",
            escape(property),
            escape(value)
        )
    };
    Ok(vec![Edit::InsertChild {
        parent: container,
        index: document.ast().element_children(container).len(),
        source,
    }])
}

/// Setter property paths are interpreted in each declaration's namespace scope. An
/// unqualified theme member belongs to its TargetType; attached members keep their
/// explicitly named owner. This prevents alias spelling from creating duplicate setters.
fn setter_property_identity(
    document: &Document,
    owner: NodeId,
    at: NodeId,
    property: &str,
) -> Result<PropertyIdentity> {
    let mut identity = PropertyIdentity::query(document, at, property)?;
    if identity.owner.is_some() || identity.namespace.is_some() {
        return Ok(identity);
    }
    let Some(target) = component_property_attribute(document, owner, "TargetType")? else {
        return Ok(identity);
    };
    let target = match target.markup().map_err(ToolError)? {
        MarkupValue::Literal(name) => name,
        MarkupValue::Extension(extension) => {
            let (prefix, name) = extension
                .name
                .rsplit_once(':')
                .unwrap_or(("", &extension.name));
            if name != "Type"
                || document
                    .ast()
                    .namespaces(owner)
                    .get(prefix)
                    .map(String::as_str)
                    != Some(XAML)
                || extension.arguments.len() != 1
                || !matches!(
                    extension.arguments[0].name.as_deref(),
                    None | Some("TypeName")
                )
            {
                return Ok(identity);
            }
            let MarkupValue::Literal(name) = &extension.arguments[0].value else {
                return Ok(identity);
            };
            name.clone()
        }
    };
    let (prefix, name) = target.rsplit_once(':').unwrap_or(("", &target));
    let Some(namespace) = document.ast().namespaces(owner).get(prefix).cloned() else {
        return Ok(identity);
    };
    identity.owner = Some(name.into());
    identity.namespace = Some(namespace);
    Ok(identity)
}

fn resource_value(document: &Document, resource: NodeId, value: &str) -> Result<Vec<Edit>> {
    let node = element(document, resource)?;
    require(
        resource_owner(document, resource).is_some() || resource == document.ast().root(),
        "Select a text-valued resource",
    )?;
    require(
        document.ast().element_children(resource).is_empty(),
        "Use the object editor for a resource with object children",
    )?;
    let text = escape(value);
    if let Some(close) = node.closing_span {
        // Keep comments and processing instructions inside scalar resources.
        let content = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .filter(|node| matches!(node.kind, NodeKind::Text | NodeKind::CData))
            .collect::<Vec<_>>();
        let mut edits = content
            .iter()
            .rev()
            .map(|node| Edit::Text {
                span: node.span,
                replacement: String::new(),
            })
            .collect::<Vec<_>>();
        if let Some(first) = content.first() {
            if let Some(Edit::Text { replacement, .. }) = edits.last_mut() {
                *replacement = text;
            }
            let _ = first;
        } else {
            edits.push(Edit::Text {
                span: SourceSpan::new(close.start, close.start),
                replacement: text,
            });
        }
        Ok(edits)
    } else {
        let source = document.source()[node.span.range()].to_owned();
        let opening = source
            .strip_suffix("/>")
            .ok_or_else(|| ToolError("Malformed scalar resource".into()))?;
        Ok(vec![Edit::Text {
            span: node.span,
            replacement: format!("{opening}>{text}</{}>", node.name),
        }])
    }
}

fn replace_definition(document: &Document, node: NodeId, source: &str) -> Result<Vec<Edit>> {
    let original = element(document, node)?;
    let parsed = context_fragment(document, node, source)?;
    let roots = parsed.ast().element_children(parsed.ast().root());
    require(
        roots.len() == 1,
        "A definition needs exactly one root element",
    )?;
    let replacement = parsed.ast().node(roots[0]).expect("root");
    require(
        replacement.kind == NodeKind::Element,
        "A definition cannot be a property element",
    )?;
    require(
        directive(original, "Key") == directive(replacement, "Key"),
        "Keep the definition's resource key; use Rename resource to update references",
    )?;
    require(
        original.xaml_name() == replacement.xaml_name(),
        "Keep the definition's name while replacing its object source",
    )?;
    Ok(vec![Edit::Text {
        span: original.span,
        replacement: component_fragment(&parsed, replacement.id)?,
    }])
}

fn extract_property(
    document: &Document,
    node: NodeId,
    property: &str,
    owner: NodeId,
    key: &str,
) -> Result<Vec<Edit>> {
    let element = element(document, node)?;
    let object = component_property_object(document, node, property)?;
    let source = if let Some(source) = object {
        source
    } else {
        let attributes = attributes_for(document, element, property)?;
        let attribute = attributes.first().ok_or_else(|| {
            ToolError("Extracting a resource requires an authored property value".into())
        })?;
        let MarkupValue::Literal(value) = attribute.markup().map_err(ToolError)? else {
            return Err(ToolError(
                "The property is already an expression; edit its defining resource instead".into(),
            ));
        };
        let (ui, xaml, declarations) = creation_namespaces(document, owner);
        match property {
            "Background" | "Foreground" | "BorderBrush" | "Fill" | "Stroke" | "CaretBrush"
            | "SelectionBrush" => format!(
                "<{ui}:SolidColorBrush{declarations} Color=\"{}\" />",
                escape(&references::escaped_literal(&value))
            ),
            "Color" => format!("<{ui}:Color{declarations}>{}</{ui}:Color>", escape(&value)),
            "Margin" | "Padding" | "BorderThickness" => format!(
                "<{ui}:Thickness{declarations}>{}</{ui}:Thickness>",
                escape(&value)
            ),
            "CornerRadius" => format!(
                "<{ui}:CornerRadius{declarations}>{}</{ui}:CornerRadius>",
                escape(&value)
            ),
            "Width" | "Height" | "Opacity" | "FontSize" | "Spacing" | "MinWidth" | "MinHeight"
            | "MaxWidth" | "MaxHeight" => {
                require(
                    value.parse::<f64>().is_ok_and(|value| value.is_finite()),
                    "This numeric property is not a finite literal",
                )?;
                format!(
                    "<{xaml}:Double{declarations}>{}</{xaml}:Double>",
                    escape(&value)
                )
            }
            "Text" | "Content" | "Watermark" | "PlaceholderText" | "Tag" => format!(
                "<{xaml}:String{declarations}>{}</{xaml}:String>",
                escape(&value)
            ),
            _ => {
                return Err(ToolError(
                    "Choose an explicit resource type for this property before extracting it"
                        .into(),
                ));
            }
        }
    };
    let mut edits = create_resource(document, owner, key, &source)?;
    // Resolve against the candidate so a nested scope cannot silently shadow the new key.
    let mut candidate = document.clone();
    candidate
        .apply("Stage extracted resource", edits.clone())
        .map_err(error)?;
    let resource = resolve_component_resource(&candidate, node, key).ok_or_else(|| {
        ToolError("The extracted resource would not be visible at this property".into())
    })?;
    let container = resource_container(&candidate, owner)?
        .ok_or_else(|| ToolError("Missing extracted dictionary".into()))?;
    require(
        candidate.ast().node(resource).and_then(|node| node.parent) == Some(container),
        "A nearer resource would shadow the extracted property",
    )?;
    // The target keeps its identity across insertion; scalar_property only addresses IDs.
    edits.extend(scalar_property(
        &candidate,
        node,
        property,
        Some(&references::reference_expression(
            &candidate, node, key, false,
        )?),
    )?);
    Ok(edits)
}

fn creation_namespaces(document: &Document, owner: NodeId) -> (String, String, String) {
    let mut namespaces = document.ast().namespaces(owner);
    let ui = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == FRAMEWORK)
        .map(|(prefix, _)| prefix.clone())
        .unwrap_or_else(|| fresh_prefix(&namespaces, "ui"));
    namespaces.insert(ui.clone(), FRAMEWORK.into());
    let xaml = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && uri.as_str() == XAML)
        .map(|(prefix, _)| prefix.clone())
        .unwrap_or_else(|| fresh_prefix(&namespaces, "x"));
    (
        ui.clone(),
        xaml.clone(),
        format!(" xmlns:{ui}=\"{FRAMEWORK}\" xmlns:{xaml}=\"{XAML}\""),
    )
}
fn fresh_prefix(namespaces: &BTreeMap<String, String>, base: &str) -> String {
    if !namespaces.contains_key(base) {
        return base.into();
    }
    for index in 1.. {
        let name = format!("{base}{index}");
        if !namespaces.contains_key(&name) {
            return name;
        }
    }
    unreachable!()
}
fn type_name(value: &str) -> Result<()> {
    require(!value.trim().is_empty(), "Choose a target type")?;
    if value.starts_with('{') {
        MarkupValue::parse(value).map_err(ToolError)?;
        Ok(())
    } else {
        property_name(value)
    }
}
fn type_expression(value: &str, xaml: &str) -> String {
    if value.starts_with('{') {
        value.into()
    } else {
        format!("{{{xaml}:Type {value}}}")
    }
}
