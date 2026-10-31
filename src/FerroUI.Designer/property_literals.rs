//! Native literal serialization shared by direct session edits and component commands.
//! Portable property descriptors cannot prove that an application's `Width` is a number.
use crate::{DesignerSession, Document, NodeId, PropertyEditorContext};
use ferroui_base::{FerroObject, FerroProperty, TypeInfo};
use ferroui_designer_core::MarkupValue;
use ferroui_inspect::Inspector;
use std::rc::Rc;

const FRAMEWORK: &str = "https://github.com/ferroui";
const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

impl DesignerSession {
    pub(crate) fn canonical_property_literal(
        &self,
        node: NodeId,
        name: &str,
        value: &str,
        setter: bool,
    ) -> Result<Option<&'static str>, String> {
        if value.strip_prefix("{}").unwrap_or(value).trim() != "Auto" {
            return Ok(None);
        }
        let property = if setter {
            setter_target_type(self.document(), node)
                .and_then(|type_| property_on_type(self.document(), node, type_, name))
        } else {
            // A retained preview can describe an older document after a failed reload.
            let live = if matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document().revision())
                && !name.contains(':')
            {
                self.preview.visual(node).and_then(|visual| {
                    Inspector::property(&visual.upcast::<FerroObject>(), name).ok()
                })
            } else {
                None
            };
            live.or_else(|| {
                element_type(self.document(), node)
                    .and_then(|type_| property_on_type(self.document(), node, type_, name))
            })
        };
        let Some(property) = property else {
            // Unknown application types retain their own serializer and authored spelling.
            return Ok(None);
        };
        if ![
            std::any::TypeId::of::<f32>(),
            std::any::TypeId::of::<f64>(),
            std::any::TypeId::of::<Option<f32>>(),
            std::any::TypeId::of::<Option<f64>>(),
        ]
        .contains(&property.property_type())
        {
            return Ok(None);
        }
        let context = PropertyEditorContext {
            property: name.into(),
            targets: vec![node],
            value: None,
            mixed: false,
            runtime_type: Some(property.property_type_name().into()),
            apply: Rc::new(|_| {}),
        };
        crate::resolve_property_editor_metadata(&context, Some(property))
            .prepare_literal(&context, value)
    }
}

fn registered_type(document: &Document, node: NodeId, token: &str) -> Option<&'static TypeInfo> {
    let (prefix, name) = token.split_once(':').unwrap_or(("", token));
    let namespaces = document.ast().namespaces(node);
    let uri = match namespaces.get(prefix) {
        Some(uri) => uri.as_str(),
        None if prefix.is_empty() => FRAMEWORK,
        None => return None,
    };
    crate::native_types::registered_xml_type(uri, name)
}

fn element_type(document: &Document, node: NodeId) -> Option<&'static TypeInfo> {
    let element = document.ast().node(node)?;
    if let Some(class) = element.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some(XAML)
            && attribute.name.rsplit(':').next() == Some("Class")
    }) {
        let (namespace, name) = class.value.rsplit_once('.')?;
        return TypeInfo::find(namespace, name);
    }
    registered_type(document, node, &element.name)
}

fn property_on_type(
    document: &Document,
    node: NodeId,
    type_: &'static TypeInfo,
    name: &str,
) -> Option<&'static FerroProperty> {
    if let Some((owner, member)) = name.rsplit_once('.') {
        let owner = registered_type(document, node, owner)?;
        let property = owner.find_property(member)?;
        return (property.is_attached() || owner.is_assignable_from(type_)).then_some(property);
    }
    // A qualified ordinary name can belong to an application markup namespace.
    (!name.contains(':'))
        .then(|| type_.find_property(name))
        .flatten()
}

fn setter_target_type(document: &Document, owner: NodeId) -> Option<&'static TypeInfo> {
    let type_ = element_type(document, owner)?;
    if !ferroui_base::styling::ControlTheme::TYPE.is_assignable_from(type_) {
        return None;
    }
    let value = document.ast().node(owner)?.attribute("TargetType")?;
    let token = match MarkupValue::parse(value).ok()? {
        MarkupValue::Literal(value) => value,
        MarkupValue::Extension(extension) => {
            let (prefix, name) = extension.name.split_once(':')?;
            if name != "Type"
                || document
                    .ast()
                    .namespaces(owner)
                    .get(prefix)
                    .map(String::as_str)
                    != Some(XAML)
            {
                return None;
            }
            extension.arguments.into_iter().find_map(|argument| {
                if argument.name.is_none() || argument.name.as_deref() == Some("TypeName") {
                    if let MarkupValue::Literal(value) = argument.value {
                        return Some(value);
                    }
                }
                None
            })?
        }
    };
    registered_type(document, owner, token.trim())
}

#[cfg(test)]
#[path = "property_literals_tests.rs"]
mod tests;
