//! Structured attribute bindings use Studio's source-preserving draft and property transaction.
use crate::component_session::{
    property_binding_extension, property_relative_source_extension, property_schema,
};
use crate::editors::{
    binding_value_source, PropertyBindingDraft, PropertyBindingOption, PropertyBindingSource,
    BINDING_MODES, BINDING_TRIGGERS,
};
use crate::{DesignerSession, NodeId};
use ferroui_designer_core::MarkupValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(tag = "field", rename_all = "camelCase")]
pub(crate) enum BindingChange {
    Path {
        value: String,
    },
    Mode {
        value: String,
    },
    Trigger {
        value: String,
    },
    Delay {
        value: Option<i32>,
    },
    Source {
        value: BindingSource,
    },
    Option {
        name: PropertyBindingOption,
        value: Option<BindingValue>,
    },
}

impl BindingChange {
    fn apply(self, draft: &mut PropertyBindingDraft) -> Result<(), String> {
        match self {
            Self::Path { value } => {
                draft.set_path(&value);
                Ok(())
            }
            Self::Mode { value } => draft.set_mode(&value),
            Self::Trigger { value } => draft.set_update_source_trigger(&value),
            Self::Delay { value } => draft.set_delay(value),
            Self::Source { value } => draft.set_source(value.try_into()?),
            Self::Option { name, value } => {
                draft.set_option(name, value.map(TryInto::try_into).transpose()?)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum BindingSource {
    DataContext,
    Element {
        name: String,
    },
    SelfValue,
    TemplatedParent,
    Ancestor {
        type_name: String,
        level: i32,
        logical_tree: bool,
    },
    Explicit {
        value: BindingValue,
    },
}

impl From<PropertyBindingSource> for BindingSource {
    fn from(source: PropertyBindingSource) -> Self {
        match source {
            PropertyBindingSource::DataContext => Self::DataContext,
            PropertyBindingSource::ElementName(name) => Self::Element { name },
            PropertyBindingSource::SelfValue => Self::SelfValue,
            PropertyBindingSource::TemplatedParent => Self::TemplatedParent,
            PropertyBindingSource::Ancestor {
                type_name,
                level,
                logical_tree,
            } => Self::Ancestor {
                type_name,
                level,
                logical_tree,
            },
            PropertyBindingSource::Explicit(value) => Self::Explicit {
                value: value.into(),
            },
        }
    }
}

impl TryFrom<BindingSource> for PropertyBindingSource {
    type Error = String;

    fn try_from(source: BindingSource) -> Result<Self, Self::Error> {
        Ok(match source {
            BindingSource::DataContext => Self::DataContext,
            BindingSource::Element { name } => Self::ElementName(name),
            BindingSource::SelfValue => Self::SelfValue,
            BindingSource::TemplatedParent => Self::TemplatedParent,
            BindingSource::Ancestor {
                type_name,
                level,
                logical_tree,
            } => Self::Ancestor {
                type_name,
                level,
                logical_tree,
            },
            BindingSource::Explicit { value } => Self::Explicit(value.try_into()?),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum BindingValue {
    Text { value: String },
    Expression { value: String },
}

impl From<MarkupValue> for BindingValue {
    fn from(value: MarkupValue) -> Self {
        match value {
            MarkupValue::Literal(value) => Self::Text { value },
            value => Self::Expression {
                value: binding_value_source(&value),
            },
        }
    }
}

impl TryFrom<BindingValue> for MarkupValue {
    type Error = String;

    fn try_from(value: BindingValue) -> Result<Self, Self::Error> {
        match value {
            BindingValue::Text { value } => Ok(Self::Literal(value)),
            BindingValue::Expression { value } => match Self::parse(value.trim())? {
                value @ Self::Extension(_) => Ok(value),
                _ => Err("Enter a XAML markup extension for an expression".into()),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BindingModel {
    pub(crate) revision: u64,
    pub(crate) authored: Option<String>,
    pub(crate) extension: String,
    pub(crate) relative_sources: bool,
    pub(crate) fields: Option<BindingFields>,
    pub(crate) modes: &'static [&'static str],
    pub(crate) triggers: &'static [&'static str],
    pub(crate) options: BTreeMap<&'static str, Option<BindingValue>>,
    pub(crate) element_names: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct BindingFields {
    path: Option<String>,
    mode: Option<String>,
    trigger: Option<String>,
    delay: Option<i32>,
    source: Option<BindingSource>,
}

struct BindingTarget {
    authored: Option<String>,
    extension: String,
    relative: Option<String>,
}

impl BindingTarget {
    fn new(session: &DesignerSession, node: NodeId, property: &str) -> Result<Self, String> {
        let document = session.document();
        let element = document
            .ast()
            .node(node)
            .filter(|element| element.kind == ferroui_designer_core::NodeKind::Element)
            .ok_or("Select a source control")?;
        let info = property_schema().property(
            element.local_name(),
            element.namespace_uri.as_deref(),
            property,
        );
        let descriptor = session
            .property_tools()
            .descriptors(document, node)
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|entry| entry.name == property);
        if property == "Name"
            || property.contains(':')
            || !document.is_valid()
            || info
                .as_ref()
                .is_some_and(|info| info.read_only || info.event)
            || descriptor
                .as_ref()
                .is_some_and(|descriptor| descriptor.read_only)
            || info.is_none() && descriptor.is_none()
        {
            return Err(format!(
                "{property} does not support attribute binding on this control"
            ));
        }
        let property_elements =
            ferroui_designer_tools::component_property_elements(document, node, property)
                .map_err(|error| error.to_string())?;
        if !property_elements.is_empty() {
            return Err("Edit this object-valued property in source to preserve its graph".into());
        }
        let authored = session
            .property_tools()
            .state(document, &[node], property)
            .map_err(|error| error.to_string())?
            .value;
        if property == "Content"
            && authored.is_none()
            && element
                .children
                .iter()
                .filter_map(|child| document.ast().node(*child))
                .any(|child| {
                    child.is_element()
                        || child
                            .text
                            .as_deref()
                            .is_some_and(|text| !text.trim().is_empty())
                })
        {
            return Err("Edit the existing inline content in source before binding Content".into());
        }
        let extension = property_binding_extension(document, node, authored.as_deref());
        if extension.is_empty() {
            return Err("Declare a framework namespace exposing Binding on this control".into());
        }
        let relative = property_relative_source_extension(document, node);
        Ok(Self {
            authored,
            extension,
            relative,
        })
    }

    fn draft(&self) -> Option<PropertyBindingDraft> {
        let value = self.authored.as_deref()?;
        Some(
            PropertyBindingDraft::new(value, &self.extension)
                .ok()?
                .with_relative_source_extension(self.relative.as_deref()),
        )
    }
}

impl DesignerSession {
    pub(crate) fn binding_model(
        &self,
        node: NodeId,
        property: &str,
    ) -> Result<BindingModel, String> {
        let target = BindingTarget::new(self, node, property)?;
        let draft = target.draft();
        let fields = draft.as_ref().map(|draft| BindingFields {
            path: draft.literal("Path").map(str::to_owned),
            mode: draft.literal("Mode").map(str::to_owned),
            trigger: draft.literal("UpdateSourceTrigger").map(str::to_owned),
            delay: draft.literal("Delay").and_then(|delay| delay.parse().ok()),
            source: draft.source_specification().map(Into::into),
        });
        let options = PropertyBindingOption::ALL
            .into_iter()
            .map(|option| {
                let name = option.name();
                (
                    name,
                    draft
                        .as_ref()
                        .and_then(|draft| draft.argument(name))
                        .cloned()
                        .map(Into::into),
                )
            })
            .collect();
        Ok(BindingModel {
            revision: self.document().revision(),
            authored: target.authored,
            extension: target.extension,
            relative_sources: target.relative.is_some(),
            fields,
            modes: BINDING_MODES,
            triggers: BINDING_TRIGGERS,
            options,
            element_names: crate::element_names::names_in_scope(self.document(), node),
        })
    }

    pub(crate) fn edit_binding(
        &mut self,
        node: NodeId,
        property: &str,
        changes: Vec<BindingChange>,
        replace: bool,
    ) -> Result<(), String> {
        let target = BindingTarget::new(self, node, property)?;
        let mut draft = match target.draft() {
            Some(draft) => draft,
            None if target.authored.is_none() || replace => {
                PropertyBindingDraft::empty(&target.extension)?
                    .with_relative_source_extension(target.relative.as_deref())
            }
            None => {
                return Err("Choose to replace the existing value before creating a binding".into())
            }
        };
        for change in changes {
            change.apply(&mut draft)?;
        }
        draft.validate()?;
        let value = draft.source();
        if target.authored.as_deref() == Some(&value) {
            return Ok(());
        }
        self.set_property(&[node], property, &value)
    }
}
