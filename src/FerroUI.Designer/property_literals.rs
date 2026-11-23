//! Native literal validation and serialization for session edits and component commands.
//! Portable property descriptors cannot prove that an application's `Width` is a number.
use crate::{DesignerSession, Document, NodeId, PropertyEditorContext};
use ferroui_base::{FerroObject, FerroProperty, TypeInfo};
pub(crate) use ferroui_designer_animation::native::{registered_type, setter_target_type};
use ferroui_designer_animation::native::element_type;
#[cfg(feature = "mcp")]
use ferroui_designer_tools::PropertyComponentType;
use ferroui_inspect::Inspector;
use std::rc::Rc;

impl DesignerSession {
    pub(crate) fn canonical_property_literal(
        &self,
        node: NodeId,
        name: &str,
        value: &str,
        setter: bool,
    ) -> Result<Option<&'static str>, String> {
        if value.starts_with('{') && !value.starts_with("{}") {
            return Ok(None);
        }
        let Some(property) = self.literal_property(node, name, setter) else {
            // Unknown application types retain their own serializer and authored spelling.
            return Ok(None);
        };
        if let Some(shape) = crate::editors::property_component_shape(property.property_type_name()) {
            if property.is_read_only() {
                return Err(format!("{name} is read-only"));
            }
            shape.validate_markup(name, value).map_err(|error| error.to_string())?;
        }
        if value.strip_prefix("{}").unwrap_or(value).trim() != "Auto" {
            return Ok(None);
        }
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

    #[cfg(feature = "mcp")]
    pub(crate) fn authored_component_shape(&self, node: NodeId, name: &str) -> Option<PropertyComponentType> {
        let property = self.literal_property(node, name, false)?;
        if property.is_read_only() {
            return None;
        }
        crate::editors::property_component_shape(property.property_type_name())
    }

    fn literal_property(&self, node: NodeId, name: &str, setter: bool) -> Option<&'static FerroProperty> {
        if setter {
            return setter_target_type(self.document(), node)
                .and_then(|type_| property_on_type(self.document(), node, type_, name));
        }
        // A retained preview can describe an older document after a failed reload.
        if matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document().revision())
            && !name.contains(':')
        {
            let live = self.preview.visual(node).and_then(|visual| {
                Inspector::property(&visual.upcast::<FerroObject>(), name).ok()
            });
            if live.is_some() {
                return live;
            }
        }
        element_type(self.document(), node)
            .and_then(|type_| property_on_type(self.document(), node, type_, name))
    }
}

pub(crate) fn property_on_type(
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

#[cfg(test)]
#[path = "property_literals_tests.rs"]
mod tests;
