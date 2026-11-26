//! Native validation for authored style and palette edits. All source planning is shared
//! with the component inspector. Targets are identified by their authored namespace.
use crate::property_literals::{property_on_type, registered_type, setter_target_type};
use crate::{
    resolve_property_editor_metadata, DesignerSession, PaletteEdit, ScopedPalette, PropertyEditorContext,
    PropertyEditorMetadata,
};
use ferroui_base::media::Color;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::{FerroProperty, StyledElement};
use ferroui_designer_animation::native::{parent_style, selector_parser};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId};
use ferroui_designer_tools::component_palette::{PaletteColorKind, PaletteLayer};
use ferroui_designer_tools::component_styles::{authored_setters, plan_style_change, StyleChange};
use ferroui_designer_tools::theme_variants::HighContrastProvider;
use ferroui_designer_tools::{
    component_property_attribute, component_resource_expression, resolve_component_resource,
    ComponentAction, ComponentEntry, ComponentKind, ComponentResourceSources, ResourceValue,
};
use ferroui_markup::markup::parsers::{SelectorGrammar, SelectorSyntax};
use std::rc::Rc;

pub(crate) fn supports_styles(document: &Document, node: &Node) -> bool {
    registered_type(document, node.id, &node.name).is_some_and(|type_| {
        StyledElement::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::Style::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::Styles::TYPE.is_assignable_from(type_)
    })
}

pub(crate) fn supports_palette(document: &Document, node: &Node) -> bool {
    registered_type(document, node.id, &node.name).is_some_and(|type_| {
        StyledElement::TYPE.is_assignable_from(type_)
            || ferroui_base::controls::ResourceDictionary::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::Style::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_)
            || ferroui_base::styling::Styles::TYPE.is_assignable_from(type_)
    })
}

pub(crate) fn contrast_providers() -> Vec<HighContrastProvider> {
    HighContrastProvider::ALL
        .into_iter()
        .filter(|provider| MarkupAssembly::find(provider.assembly()).is_some())
        .collect()
}

fn validate_selector(
    document: &Document,
    node: NodeId,
    selector: &str,
    nested: bool,
) -> Result<(), String> {
    let syntax = SelectorGrammar::parse(selector).map_err(|error| error.to_string())?;
    let namespaces = document.ast().namespaces(node);
    for part in &syntax {
        let prefix = match part {
            SelectorSyntax::OfType { xmlns, .. }
            | SelectorSyntax::Is { xmlns, .. }
            | SelectorSyntax::AttachedProperty { xmlns, .. } => xmlns,
            _ => continue,
        };
        if !prefix.is_empty() && !namespaces.contains_key(prefix) {
            return Err(format!(
                "The selector namespace '{prefix}' is not declared in this scope"
            ));
        }
    }
    for selector in syntax.split(|part| matches!(part, SelectorSyntax::Comma)) {
        let has_parent = matches!(selector.first(), Some(SelectorSyntax::Nesting));
        if nested != has_parent {
            return Err(if nested {
                "Begin every nested selector with ^"
            } else {
                "The ^ selector requires an enclosing Style or ControlTheme"
            }
            .into());
        }
    }
    selector_parser(document, node)
        .parse(selector)
        .map_err(|error| error.to_string())?
        .ok_or("Enter a native style selector")
        .map(|_| ())
        .map_err(str::to_owned)
}

pub(crate) fn setter_property(
    document: &Document,
    owner: NodeId,
    name: &str,
) -> Option<&'static FerroProperty> {
    property_on_type(document, owner, setter_target_type(document, owner)?, name)
        .filter(|property| !property.is_read_only() && !property.is_direct())
}

pub(crate) fn setter_metadata(
    owner: NodeId,
    name: &str,
    property: &'static FerroProperty,
) -> (PropertyEditorContext, PropertyEditorMetadata) {
    let context = PropertyEditorContext {
        property: name.into(),
        targets: vec![owner],
        value: None,
        mixed: false,
        runtime_type: Some(property.property_type_name().into()),
        apply: Rc::new(|_| {}),
    };
    let metadata = resolve_property_editor_metadata(&context, Some(property));
    (context, metadata)
}

pub(crate) fn compatible_base(document: &Document, theme: NodeId, entry: &ComponentEntry) -> bool {
    if entry.node == theme || entry.kind != ComponentKind::ControlTheme {
        return false;
    }
    let types = setter_target_type(document, theme).zip(setter_target_type(document, entry.node));
    types.is_some_and(|(derived, base)| base.is_assignable_from(derived))
        && entry
            .key
            .as_deref()
            .is_some_and(|key| resolve_component_resource(document, theme, key) == Some(entry.node))
        && plan_style_change(
            document,
            StyleChange::SetBase {
                theme,
                base: Some(entry.node),
            },
        )
        .is_ok()
}

pub(crate) fn authored_base_value(document: &Document, theme: NodeId) -> Option<String> {
    component_property_attribute(document, theme, "BasedOn")
        .ok()
        .flatten()
        .map(|attribute| attribute.value.clone())
}

pub(crate) fn argb(color: &str) -> Result<String, String> {
    let color = Color::parse(color.trim()).map_err(|error| error.to_string())?;
    Ok(format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color.a, color.r, color.g, color.b
    ))
}

impl DesignerSession {
    pub(crate) fn ensure_style_document(&self) -> Result<(), String> {
        if self.rust.is_some() {
            return Err("Edit Rust-owned style builders in Rust source".into());
        }
        Ok(())
    }

    pub(crate) fn change_style(&mut self, change: StyleChange<'_>) -> Result<(), String> {
        self.ensure_style_edit()?;
        let document = self.document();
        let label = match change {
            StyleChange::Create { owner, selector } => {
                let node = document.ast().node(owner).ok_or("Select a style owner")?;
                if !supports_styles(document, node) {
                    return Err("Choose an authored style host".into());
                }
                let nested = registered_type(document, owner, &node.name).is_some_and(|type_| {
                    ferroui_base::styling::Style::TYPE.is_assignable_from(type_)
                        || ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_)
                });
                validate_selector(document, owner, selector, nested)?;
                "Create style"
            }
            StyleChange::SetSelector { node, selector } => {
                validate_selector(
                    document,
                    node,
                    selector,
                    parent_style(document, node).is_some(),
                )?;
                "Edit style selector"
            }
            StyleChange::SetBase { theme, base } => {
                if let Some(base) = base {
                    let types =
                        setter_target_type(document, theme).zip(setter_target_type(document, base));
                    if !types.is_some_and(|(derived, base)| base.is_assignable_from(derived)) {
                        return Err(
                            "Choose a base theme for the same control type or an ancestor".into(),
                        );
                    }
                }
                "Change control theme base"
            }
            StyleChange::Remove { .. } => "Remove style",
        };
        let edits = plan_style_change(document, change).map_err(|error| error.to_string())?;
        self.apply_style_edits(label, edits)
    }

    pub(crate) fn set_style_value(
        &mut self,
        owner: NodeId,
        property: &str,
        value: Option<&str>,
    ) -> Result<(), String> {
        self.ensure_style_edit()?;
        let value = if let Some(value) = value {
            let native = setter_property(self.document(), owner, property);
            // An existing setter can use a project serializer the isolated host does
            // not know. New properties come from the native type registry.
            if native.is_none()
                && !authored_setters(self.document(), owner)
                    .map_err(|error| error.to_string())?
                    .iter()
                    .any(|setter| setter.property == property)
            {
                return Err(
                    "Choose a writable property on the style's resolved target type".into(),
                );
            }
            let parsed = MarkupValue::parse(value).map_err(|error| error.to_string())?;
            let value = if let (Some(native), MarkupValue::Literal(_)) = (native, parsed) {
                let (context, metadata) = setter_metadata(owner, property, native);
                metadata.prepare_literal(&context, value)?.unwrap_or(value)
            } else {
                value
            };
            Some(value.into())
        } else {
            None
        };
        self.edit_component(ComponentAction::SetSetter {
            owner,
            property: property.into(),
            value,
        })
    }

    pub(crate) fn set_style_resource(
        &mut self,
        owner: NodeId,
        property: &str,
        resource: NodeId,
        dynamic: bool,
    ) -> Result<(), String> {
        let key = self
            .style_resources(owner, property)?
            .into_iter()
            .find(|entry| entry.node == resource)
            .and_then(|entry| entry.key)
            .ok_or("Choose a compatible resource visible in this style's scope")?;
        let expression = component_resource_expression(self.document(), owner, &key, dynamic)
            .map_err(|error| error.to_string())?;
        self.set_style_value(owner, property, Some(&expression))
    }

    pub(crate) fn style_resources(
        &self,
        owner: NodeId,
        property: &str,
    ) -> Result<Vec<ComponentEntry>, String> {
        self.ensure_style_document()?;
        let native = setter_property(self.document(), owner, property)
            .ok_or("Select a known writable property")?;
        let document = self.document();
        let includes = self.accepted_preview_includes().map(crate::runtime_include_sources::IncludeLookup);
        let sources = includes.as_ref().map(|includes| includes as &dyn ComponentResourceSources);
        let context = self.component_resource_context(owner);
        Ok(self
            .visible_component_resources(owner)?
            .into_iter()
            .filter_map(|mut entry| {
                let key = entry.key.as_deref()?;
                let candidate = document.ast().node(entry.node)?;
                if !ResourceValue::is_static_alias(document, candidate) {
                    return (crate::editors::resource_is_compatible(native, candidate) == Some(true))
                        .then_some(entry);
                }
                let target = ResourceValue::lookup(document, self.document_uri(), sources, owner, key, context.as_ref())
                    .ok().flatten().filter(|value| value.location().node == entry.node && value.location().uri.is_none())?
                    .alias_target(key, context.as_ref())?;
                let terminal = target.document().ast().node(target.location().node)?;
                if crate::editors::resource_is_compatible(native, terminal) != Some(true) {
                    return None;
                }
                entry.type_name = format!("{{{}}}{}", terminal.namespace_uri.as_deref().unwrap_or(""), terminal.local_name());
                Some(entry)
            })
            .collect())
    }

    pub(crate) fn change_palette_color(&mut self, node: NodeId, color: &str) -> Result<(), String> {
        self.ensure_style_edit()?;
        let edits = ScopedPalette::for_session(self).plan(&PaletteEdit::Set { node, color: color.into() })?;
        self.apply_style_edits("Edit palette color", edits)
    }

    pub(crate) fn add_palette_color(
        &mut self,
        owner: NodeId,
        layer: PaletteLayer,
        key: &str,
        kind: PaletteColorKind,
        color: &str,
        provider: Option<HighContrastProvider>,
    ) -> Result<(), String> {
        self.ensure_style_edit()?;
        let scope = self
            .document()
            .ast()
            .node(owner)
            .ok_or("Choose a palette scope")?;
        if !supports_palette(self.document(), scope) {
            return Err("Choose an authored resource owner".into());
        }
        let edits = ScopedPalette::for_session(self).plan(&PaletteEdit::Add {
            owner, layer, key: key.into(), kind, color: color.into(), provider,
        })?;
        self.apply_style_edits("Add palette color", edits)
    }

    fn apply_style_edits(
        &mut self,
        label: &str,
        edits: Vec<ferroui_designer_core::Edit>,
    ) -> Result<(), String> {
        if !edits.is_empty() {
            self.apply(label, edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn ensure_style_edit(&self) -> Result<(), String> {
        self.ensure_style_document()?;
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() {
            return Err(
                "Wait for the current native design before editing styles and colors".into(),
            );
        }
        Ok(())
    }
}
