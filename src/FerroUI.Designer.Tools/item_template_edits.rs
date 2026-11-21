//! Source-preserving edits for a checked item template declaration.
use super::*;

pub struct ControlTemplatePart<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    pub namespace: &'a str,
}

impl ItemTemplateSlot {
    pub fn plan_control_template(&self, document: &Document, parts: &[ControlTemplatePart<'_>]) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        require(self.property == ItemTemplateMember::Template && matches!(self.value, ItemTemplateValue::Unset),
            "Reset the current control template or make an inline copy")?;
        let owner = element(document, self.owner)?;
        let source = starter(document, owner, self.property, parts);
        validated(document, vec![insert(owner, self.property.name(), &source)])
    }

    pub fn plan(&self, document: &Document, change: ItemTemplateChange) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        let owner = element(document, self.owner)?;
        let property = self.property.name();
        let mut edits = self.clear();
        match change {
            ItemTemplateChange::Reset => {
                require(
                    self.can_reset || matches!(self.value, ItemTemplateValue::Unset),
                    "This slot requires an item template",
                )?;
            }
            ItemTemplateChange::CreateInline => {
                require(self.property != ItemTemplateMember::Template, "Create a control template from its registered parts")?;
                require(
                    matches!(self.value, ItemTemplateValue::Unset),
                    "Reset the current template or make an inline copy",
                )?;
                edits.push(insert(owner, property, &starter(document, owner, self.property, &[])));
            }
            ItemTemplateChange::AssignResource { resource } => {
                let node = element(document, resource)?;
                let key = keyed_template(node, self.property)
                    .ok_or_else(|| ToolError("Select an authored resource compatible with this template property".into()))?;
                require(
                    resolve_component_resource(document, owner.id, &key) == Some(resource),
                    "The template resource is hidden or ambiguous in this item's scope",
                )?;
                return self.assign_key(document, &key);
            }
            ItemTemplateChange::AssignIncludedResource { .. } => {
                return Err(ToolError("Resolve the included resource from the accepted source graph".into()));
            }
            ItemTemplateChange::CopyInline => {
                require(
                    self.can_copy_inline(document),
                    "Copy requires a local template that resolves consistently across themes",
                )?;
                let original = self.value.definition().expect("copyable resource");
                edits.push(insert(
                    owner,
                    property,
                    &unkeyed_fragment(document, original)?,
                ));
                validate_copy(document, owner, property, original, &edits)?;
            }
        }
        validated(document, edits)
    }

    /// The host verifies the winning resource in its accepted inclusion graph
    /// before supplying the key. Local assignment uses the same source edits.
    pub fn assign_key(&self, document: &Document, key: &str) -> Result<Vec<Edit>> {
        self.ensure_editable()?;
        if matches!(&self.value, ItemTemplateValue::Resource { key: current, dynamic: false, .. } if current == key) {
            return Ok(Vec::new());
        }
        let value = references::reference_expression(document, self.owner, key, false)?;
        let mut edits = self.clear();
        let name = if let ItemTemplateDeclaration::Attribute(name) = &self.declaration {
            edits.clear();
            name.clone()
        } else {
            self.property.name().into()
        };
        edits.push(Edit::SetAttribute { node: self.owner, name, value });
        validated(document, edits)
    }

    pub(super) fn ensure_editable(&self) -> Result<()> {
        self.reason.as_ref().map_or(Ok(()), |reason| Err(ToolError(reason.clone())))
    }

    pub(super) fn clear(&self) -> Vec<Edit> {
        let edit = match &self.declaration {
            ItemTemplateDeclaration::Unset => return Vec::new(),
            ItemTemplateDeclaration::Attribute(name) => Edit::RemoveAttribute {
                node: self.owner,
                name: name.clone(),
            },
            ItemTemplateDeclaration::Element(node) => Edit::RemoveNode { node: *node },
        };
        vec![edit]
    }
}

fn starter(document: &Document, owner: &Node, property: ItemTemplateMember, parts: &[ControlTemplatePart<'_>]) -> String {
    let namespaces = document.ast().namespaces(owner.id);
    let authored = namespaces.iter().find(|(_, uri)| uri.as_str() == FRAMEWORK);
    let (prefix, namespace) = match authored {
        Some((prefix, _)) if prefix.is_empty() => (String::new(), String::new()),
        Some((prefix, _)) => (format!("{prefix}:"), String::new()),
        None if property == ItemTemplateMember::Template => {
            let prefix = (0..).map(|index| format!("fu{index}"))
                .find(|prefix| !namespaces.contains_key(prefix)).expect("finite authored namespaces");
            (format!("{prefix}:"), format!(" xmlns:{prefix}=\"{FRAMEWORK}\""))
        }
        None => (String::new(), format!(" xmlns=\"{FRAMEWORK}\"")),
    };
    if property == ItemTemplateMember::Template {
        let body = if parts.is_empty() {
            format!("<{prefix}Border />")
        } else {
            let mut body = format!("<{prefix}Grid xmlns:x=\"{XAML}\">");
            for part in parts {
                body.push_str(&format!("<{} xmlns=\"using:{}\" x:Name=\"{}\" />", part.type_name, part.namespace, part.name));
            }
            body.push_str(&format!("</{prefix}Grid>"));
            body
        };
        format!("<{prefix}ControlTemplate{namespace} TargetType=\"{}\">{body}</{prefix}ControlTemplate>", owner.name)
    } else {
        format!("<{prefix}DataTemplate{namespace}><{prefix}TextBlock Text=\"{{{prefix}Binding}}\" /></{prefix}DataTemplate>")
    }
}

fn insert(owner: &Node, property: &str, source: &str) -> Edit {
    let wrapper = format!("{}.{property}", owner.name);
    Edit::InsertChild {
        parent: owner.id,
        index: 0,
        source: format!("<{wrapper}>{source}</{wrapper}>"),
    }
}

fn unkeyed_fragment(document: &Document, node: NodeId) -> Result<String> {
    let mut fragment = Document::new(component_fragment(document, node)?).map_err(error)?;
    strip_template_key(&mut fragment)?;
    Ok(fragment.valid_source().into())
}

pub(super) fn strip_template_key(fragment: &mut Document) -> Result<()> {
    let root = element(fragment, fragment.ast().root())?;
    if let Some(key) = key_attribute(root) {
        let edit = Edit::RemoveAttribute {
            node: root.id,
            name: key.name.clone(),
        };
        fragment
            .apply("Remove copied template key", vec![edit])
            .map_err(error)?;
    }
    Ok(())
}

fn validate_copy(
    document: &Document,
    owner: &Node,
    property: &str,
    original: NodeId,
    edits: &[Edit],
) -> Result<()> {
    let mut candidate = document.clone();
    candidate
        .apply("Validate inline item template", edits.to_vec())
        .map_err(error)?;
    let owner = element(&candidate, owner.id)?;
    let copied = property_elements(&candidate, owner, property)?
        .first()
        .and_then(|property| {
            candidate
                .ast()
                .element_children(property.id)
                .first()
                .copied()
        })
        .ok_or_else(|| ToolError("The inline template was not created".into()))?;
    references::validate_copied_references(document, original, &candidate, copied)
}
