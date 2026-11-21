use super::{CommandSelection, CommandValue, valid_path};
use crate::{LanguageError, LineIndex, Result, Schema, require};
use ferroui_designer_core::{Attribute, Document, Edit, MarkupValue, Node};

pub(super) struct AuthoredValue<'a> {
    attribute: Option<&'a Attribute>,
    element: Option<&'a Node>,
    binding: Option<Binding>,
}

struct Binding {
    extension: String,
    path: String,
    named_path: bool,
}

impl<'a> AuthoredValue<'a> {
    pub(super) fn read(
        schema: &Schema,
        document: &'a Document,
        owner: &Node,
        property: &str,
    ) -> Result<Self> {
        let attribute =
            ferroui_designer_tools::component_property_attribute(document, owner.id, property)
                .map_err(|error| LanguageError(error.to_string()))?;
        let nodes =
            ferroui_designer_tools::component_property_elements(document, owner.id, property)
                .map_err(|error| LanguageError(error.to_string()))?;
        require(
            nodes.len() + usize::from(attribute.is_some()) <= 1,
            "Repair the duplicate command property in source",
        )?;
        let element = nodes.first().and_then(|node| document.ast().node(*node));
        let binding =
            attribute.and_then(|attribute| Binding::parse(schema, document, owner, attribute));
        Ok(Self {
            attribute,
            element,
            binding,
        })
    }

    fn editable(&self) -> bool {
        self.element.is_none() && (self.attribute.is_none() || self.binding.is_some())
    }

    pub(super) fn model(&self, index: &LineIndex) -> Result<CommandValue> {
        let authored = self
            .attribute
            .map(|attribute| attribute.value.clone())
            .or_else(|| self.element.map(|element| element.name.clone()));
        let range = self
            .attribute
            .map(|attribute| attribute.value_span)
            .or_else(|| self.element.map(|element| element.name_span))
            .map(|span| index.range(span))
            .transpose()?;
        Ok(CommandValue {
            authored,
            range,
            editable: self.editable(),
        })
    }

    pub(super) fn plan(
        self,
        document: &Document,
        owner: &Node,
        property: &str,
        selection: &CommandSelection,
        edits: &mut Vec<Edit>,
    ) -> Result<()> {
        if matches!(selection, CommandSelection::Keep) {
            return Ok(());
        }
        require(
            self.editable(),
            "Edit the authored command expression in source",
        )?;
        match selection {
            CommandSelection::Bind { path } => {
                if self
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.path == *path)
                {
                    return Ok(());
                }
                let binding = self
                    .binding
                    .or_else(|| Binding::default_at(document, owner))
                    .ok_or_else(|| {
                        LanguageError("Declare a framework XAML namespace exposing Binding".into())
                    })?;
                let name = self
                    .attribute
                    .map_or_else(|| property.to_owned(), |attribute| attribute.name.clone());
                let parameter = if binding.named_path { "Path=" } else { "" };
                edits.push(Edit::SetAttribute {
                    node: owner.id,
                    name,
                    value: format!("{{{} {parameter}{path}}}", binding.extension),
                });
            }
            CommandSelection::Clear => {
                if let Some(attribute) = self.attribute {
                    edits.push(Edit::RemoveAttribute {
                        node: owner.id,
                        name: attribute.name.clone(),
                    });
                }
            }
            CommandSelection::Keep => {}
        }
        Ok(())
    }
}

impl Binding {
    fn default_at(document: &Document, owner: &Node) -> Option<Self> {
        let namespaces = document.ast().namespaces(owner.id);
        let extension = if !namespaces.contains_key("") {
            "Binding".into()
        } else {
            let (prefix, _) = namespaces
                .iter()
                .find(|(_, uri)| uri.as_str() == crate::FRAMEWORK_NAMESPACE)?;
            if prefix.is_empty() {
                "Binding".into()
            } else {
                format!("{prefix}:Binding")
            }
        };
        Some(Self {
            extension,
            path: String::new(),
            named_path: false,
        })
    }

    fn parse(
        schema: &Schema,
        document: &Document,
        owner: &Node,
        attribute: &Attribute,
    ) -> Option<Self> {
        let MarkupValue::Extension(extension) = attribute.markup().ok()? else {
            return None;
        };
        let (prefix, name) = extension
            .name
            .split_once(':')
            .unwrap_or(("", &extension.name));
        let base = name.strip_suffix("Extension").unwrap_or(name);
        let namespaces = document.ast().namespaces(owner.id);
        let namespace = namespaces.get(prefix).map(String::as_str);
        // A project extension can share a local name with a framework extension.
        let native_namespace = namespace == Some(crate::FRAMEWORK_NAMESPACE)
            || prefix.is_empty() && namespace.is_none()
            || namespace.is_some_and(|namespace| {
                schema
                    .type_info(name, Some(namespace))
                    .is_some_and(|info| info.namespace == crate::FRAMEWORK_NAMESPACE)
            });
        if !native_namespace || !matches!(base, "Binding" | "CompiledBinding" | "ReflectionBinding")
        {
            return None;
        }
        let (path, named_path) = match extension.arguments.as_slice() {
            [] => (String::new(), false),
            [argument] if argument.name.as_deref().is_none_or(|name| name == "Path") => {
                let MarkupValue::Literal(path) = &argument.value else {
                    return None;
                };
                if !path.is_empty() && !valid_path(path) {
                    return None;
                }
                (path.clone(), argument.name.is_some())
            }
            _ => return None,
        };
        Some(Self {
            extension: extension.name,
            path,
            named_path,
        })
    }
}
