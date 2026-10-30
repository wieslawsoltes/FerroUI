//! Source-backed control definitions and instance snippets are distinct objects.
use super::{DesignerDefinitionAsset, DesignerWorkspace, type_token};
use ferroui_base::TypeInfo;
use ferroui_controls::Control;
use ferroui_designer_core::{Document, Edit, Node, NodeId};
use std::rc::Rc;

pub(super) use crate::native_types::registered_xml_type;

pub(super) fn class_name(node: &Node) -> Option<&str> {
    node.attributes
        .iter()
        .find(|attribute| {
            attribute.namespace_uri.as_deref()
                == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                && attribute.name.rsplit(':').next() == Some("Class")
        })
        .map(|attribute| attribute.value.as_str())
}

pub(super) fn control_instance_source(asset: &DesignerDefinitionAsset) -> Result<String, String> {
    let mut document = Document::new(&asset.source).map_err(|error| error.to_string())?;
    let root = document
        .ast()
        .node(document.ast().root())
        .ok_or("A control definition needs a root")?;
    if class_name(root).is_some() {
        let type_ = definition_control_type(asset).ok_or("Register this user control's native type and constructor before placing an instance, or provide a DesignerAsset insertion snippet")?;
        if !Control::TYPE.is_assignable_from(type_)
            || type_.namespace().is_empty()
            || type_.default_constructor().is_none()
        {
            return Err(
                "The registered definition type is not an instantiable native control".into(),
            );
        }
        return Ok(format!(
            "<dc:{} xmlns:dc=\"using:{}\" />",
            type_.name(),
            type_.namespace()
        ));
    }
    let keys = root
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.namespace_uri.as_deref()
                == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                && attribute.name.rsplit(':').next() == Some("Key")
        })
        .map(|attribute| Edit::RemoveAttribute {
            node: root.id,
            name: attribute.name.clone(),
        })
        .collect::<Vec<_>>();
    if !keys.is_empty() {
        document
            .apply("Create control instance snippet", keys)
            .map_err(|error| error.to_string())?;
    }
    Ok(document.source().to_owned())
}

pub(super) fn definition_control_type(
    asset: &DesignerDefinitionAsset,
) -> Option<&'static TypeInfo> {
    let document = Document::new(&asset.source).ok()?;
    definition_type_in_document(
        &document,
        document.ast().root(),
        asset.target_type.as_deref(),
    )
}

pub(super) fn definition_type_in_document(
    document: &Document,
    node: NodeId,
    target_type: Option<&str>,
) -> Option<&'static TypeInfo> {
    let root = document.ast().node(node)?;
    if let Some(target) = target_type.and_then(type_token) {
        if let Some(type_) = registered_type(document, root.id, &target) {
            return Some(type_);
        }
    }
    if let Some(class) = class_name(root) {
        let (namespace, name) = class.rsplit_once('.')?;
        return TypeInfo::find(namespace, name);
    }
    registered_type(document, root.id, &root.name)
}

pub(super) fn registered_type(
    document: &Document,
    node: NodeId,
    token: &str,
) -> Option<&'static TypeInfo> {
    let namespaces = document.ast().namespaces(node);
    let (prefix, name) = token.split_once(':').unwrap_or(("", token));
    let uri = namespaces.get(prefix)?;
    registered_xml_type(uri, name)
}

impl DesignerWorkspace {
    /// Open a provided control definition as a normal source tab, preserving the current
    /// document's dirty buffer and undo history. Existing tabs keep their current edits.
    pub fn open_component_document(self: &Rc<Self>, id: &str) -> Result<(), String> {
        let (asset, source) = {
            let state = self.components.borrow();
            let asset = state
                .definitions
                .get(id)
                .cloned()
                .ok_or("The definition provider is no longer available")?;
            let source = state
                .sources
                .get(id)
                .map(|provider| provider.document.source().to_owned())
                .unwrap_or_else(|| asset.source.clone());
            (asset, source)
        };
        let uri = asset
            .origin_uri
            .ok_or("Register the definition's source URI before opening it as a document")?;
        let canonical = super::super::project::document_identity(&uri)?;
        if self.document_uris().iter().any(|open| {
            super::super::project::document_identity(open)
                .ok()
                .as_deref()
                == Some(canonical.as_str())
        }) {
            self.activate_document(&uri)?;
        } else {
            self.open_document(source, uri)?;
        }
        Ok(())
    }
}
