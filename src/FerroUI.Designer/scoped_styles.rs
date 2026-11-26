//! Checked style and control-theme edits for an authored XAML include scope.
use crate::property_literals::setter_target_type;
use crate::scoped_style_fields::StyleField;
use crate::scoped_style_resources::StyleResources;
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind};
use ferroui_designer_language::Schema;
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::{
    component_property_elements, component_resource_expression,
    component_styles::{
        authored_base_key, plan_style_change, plan_theme_base_reference, StyleChange,
    },
    plan_component_action, ComponentAction, ComponentCatalog, ComponentEntry, ComponentKind,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScopedStyleEdit {
    Create {
        selector: String,
    },
    CreateTheme {
        key: String,
        target_type: String,
    },
    Selector {
        value: String,
    },
    Setter {
        property: String,
        value: Option<String>,
    },
    Resource {
        property: String,
        key: String,
        dynamic: bool,
    },
    Base {
        key: Option<String>,
    },
    Remove,
}

/// The source and project schema from one accepted graphical preview.
pub struct ScopedStyles<'a>(StyleResources<'a>);

impl<'a> ScopedStyles<'a> {
    pub fn new(
        document: &'a Document,
        address: &'a str,
        includes: &'a PreviewIncludes,
        schema: &'a Schema,
    ) -> Self {
        Self(StyleResources {
            document,
            address,
            includes,
            schema,
        })
    }

    pub fn catalog(&self) -> Result<Value, String> {
        let document = self.0.document;
        let definitions = Self::definitions(document)?.into_iter().map(|entry| {
            let owner = entry.owner.or_else(|| parent_element(document, entry.node).map(|node| node.id));
            json!({"node":entry.node,"name":entry.display_name,"kind":entry.kind,
                "scope":owner.and_then(|id| document.ast().node(id)).map(label), "target":entry.target_type})
        }).collect::<Vec<_>>();
        let mut style_hosts = Vec::new();
        let mut palette_scopes = Vec::new();
        for node in document.ast().elements() {
            if self.style_host(node) {
                style_hosts.push(json!({"node":node.id,"label":label(node),"type_name":node.local_name(),"nested":self.nested_host(node)}));
            }
            if self.palette_host(node)
                && (node.id == document.ast().root()
                    || self.type_is(node, "ResourceDictionary")
                    || component_property_elements(document, node.id, "Resources")
                        .is_ok_and(|elements| !elements.is_empty()))
            {
                palette_scopes.push(json!({"node":node.id,"label":label(node)}));
            }
        }
        Ok(
            json!({"definitions":definitions,"style_hosts":style_hosts,"palette_scopes":palette_scopes}),
        )
    }

    pub fn definition(&self, node: NodeId) -> Result<Value, String> {
        let document = self.0.document;
        let entry = self.entry(node)?;
        let mut names = BTreeSet::new();
        if let Some(target) = self.0.schema.style_target(document, node) {
            names.extend(
                self.0
                    .schema
                    .properties(&target.name, Some(&target.namespace))
                    .into_keys(),
            );
        }
        if let Some(target) = setter_target_type(document, node) {
            names.extend(
                target
                    .properties()
                    .iter()
                    .map(|property| property.name().to_owned()),
            );
        }
        names.retain(|name| self.field(node, name).is_some());
        let mut report = crate::style_report::definition(document, &entry, names, |name| {
            self.field(node, name).map_or_else(
                || (json!({"kind":"text","options":[],"resources":false}), false),
                |field| (field.describe(), true),
            )
        })?;
        if entry.kind == ComponentKind::ControlTheme {
            report["base"] = json!(authored_base_key(document, node).ok().flatten());
            if report["identity"]["editable"] == true {
                report["base_choices"] = json!(self.0.bases(node)?);
            }
        }
        Ok(report)
    }

    pub fn resources(&self, node: NodeId, name: &str) -> Result<Value, String> {
        self.entry(node)?;
        let field = self
            .field(node, name)
            .ok_or("Choose a known writable style property")?;
        Ok(json!({"resources":self.0.choices(node,&field)?}))
    }

    pub fn plan(&self, node: NodeId, edit: &ScopedStyleEdit) -> Result<Vec<Edit>, String> {
        let document = self.0.document;
        let owner = document
            .ast()
            .node(node)
            .ok_or("Choose an authored style or resource scope")?;
        let edits = match edit {
            ScopedStyleEdit::Create { selector } => {
                if !self.style_host(owner) {
                    return Err("Choose an authored style host".into());
                }
                self.0.schema.validate_style_selector(
                    document,
                    node,
                    selector,
                    self.nested_host(owner),
                )?;
                plan_style_change(
                    document,
                    StyleChange::Create {
                        owner: node,
                        selector,
                    },
                )
            }
            ScopedStyleEdit::CreateTheme { key, target_type } => {
                if !self.palette_host(owner) {
                    return Err("Choose an authored resource owner".into());
                }
                let ty = self
                    .0
                    .schema
                    .authored_type(document, node, target_type)
                    .ok_or("Choose a control type from the built Cargo schema")?;
                if !self
                    .0
                    .schema
                    .type_info("StyledElement", None)
                    .is_some_and(|base| self.0.schema.type_assignable_to(ty, base))
                {
                    return Err("The control theme target must be a styled element".into());
                }
                plan_component_action(
                    document,
                    &ComponentAction::CreateTheme {
                        owner: node,
                        key: key.clone(),
                        target_type: target_type.clone(),
                        based_on: None,
                    },
                )
            }
            ScopedStyleEdit::Selector { value } => {
                let nested =
                    parent_element(document, node).is_some_and(|parent| self.nested_host(parent));
                self.0
                    .schema
                    .validate_style_selector(document, node, value, nested)?;
                plan_style_change(
                    document,
                    StyleChange::SetSelector {
                        node,
                        selector: value,
                    },
                )
            }
            ScopedStyleEdit::Base { key } => {
                if let Some(key) = key {
                    self.0.base_type(node, key)?;
                }
                plan_theme_base_reference(document, node, key.as_deref())
            }
            ScopedStyleEdit::Setter { property, value } => {
                let value = value
                    .as_deref()
                    .map(|value| {
                        self.field(node, property)
                            .ok_or(
                                "Choose a writable property on the style's resolved target type",
                            )?
                            .prepare(value)
                            .map(str::to_owned)
                    })
                    .transpose()?;
                plan_component_action(
                    document,
                    &ComponentAction::SetSetter {
                        owner: node,
                        property: property.clone(),
                        value,
                    },
                )
            }
            ScopedStyleEdit::Resource {
                property,
                key,
                dynamic,
            } => {
                let field = self
                    .field(node, property)
                    .ok_or("Choose a known writable style property")?;
                self.0.compatible(node, key, &field)?;
                let value = component_resource_expression(document, node, key, *dynamic)
                    .map_err(|error| error.to_string())?;
                plan_component_action(
                    document,
                    &ComponentAction::SetSetter {
                        owner: node,
                        property: property.clone(),
                        value: Some(value),
                    },
                )
            }
            ScopedStyleEdit::Remove => plan_style_change(document, StyleChange::Remove { node }),
        }
        .map_err(|error| error.to_string())?;
        Ok(edits)
    }

    fn field(&self, node: NodeId, name: &str) -> Option<StyleField> {
        StyleField::resolve(self.0.schema, self.0.document, node, name)
    }

    fn entry(&self, node: NodeId) -> Result<ComponentEntry, String> {
        Self::definitions(self.0.document)?
            .into_iter()
            .find(|entry| entry.node == node)
            .ok_or_else(|| "Choose one authored Style or ControlTheme".into())
    }

    fn definitions(document: &Document) -> Result<Vec<ComponentEntry>, String> {
        let entries =
            ComponentCatalog::declarations(document).map_err(|error| error.to_string())?;
        Ok(entries
            .into_iter()
            .filter(|entry| {
                matches!(
                    entry.kind,
                    ComponentKind::Style | ComponentKind::ControlTheme
                )
            })
            .collect())
    }

    fn nested_host(&self, node: &Node) -> bool {
        self.type_is(node, "Style") || self.type_is(node, "ControlTheme")
    }
    fn style_host(&self, node: &Node) -> bool {
        self.nested_host(node)
            || self.type_is(node, "Styles")
            || self.type_is(node, "StyledElement")
    }
    fn palette_host(&self, node: &Node) -> bool {
        self.style_host(node) || self.type_is(node, "ResourceDictionary")
    }

    fn type_is(&self, node: &Node, kind: &str) -> bool {
        let types = self
            .0
            .schema
            .type_info(&node.name, node.namespace_uri.as_deref())
            .zip(self.0.schema.type_info(kind, None));
        types.is_some_and(|(actual, expected)| self.0.schema.type_assignable_to(actual, expected))
    }
}

fn label(node: &Node) -> &str {
    node.xaml_name().unwrap_or(node.local_name())
}

fn parent_element(document: &Document, node: NodeId) -> Option<&Node> {
    let ast = document.ast();
    let mut parent = ast.node(ast.node(node)?.parent?)?;
    if parent.kind == NodeKind::PropertyElement {
        parent = ast.node(parent.parent?)?;
    }
    Some(parent)
}
