//! Typed command bindings share the language service's registered DataContext paths.
use crate::{
    LanguageError, LanguageService, Position, PropertyInfo, Range, Result, Schema, require,
};
use ferroui_designer_core::{Document, Node};
use serde::{Deserialize, Serialize};

mod authored;
#[cfg(test)]
mod tests;

use authored::AuthoredValue;

#[derive(Clone, Debug, Deserialize)]
pub struct CommandBindingRequest {
    pub version: i64,
    pub position: Position,
    #[serde(default)]
    pub prefix: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandBindingEdit {
    pub property: String,
    pub command: CommandSelection,
    pub parameter: CommandSelection,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CommandSelection {
    Keep,
    Clear,
    Bind { path: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandBindingModel {
    pub slots: Vec<CommandSlot>,
    pub members: Vec<CommandMember>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSlot {
    pub property: String,
    pub binding: CommandValue,
    pub parameter: Option<CommandValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandValue {
    pub authored: Option<String>,
    pub range: Option<Range>,
    pub editable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandMember {
    pub path: String,
    pub type_name: String,
    pub command: bool,
    pub children: bool,
}

impl LanguageService {
    pub fn command_bindings(
        &self,
        uri: &str,
        request: &CommandBindingRequest,
    ) -> Result<CommandBindingModel> {
        let (document, owner) = self.command_binding_site(uri, request)?;
        let schema = self.schema_for(uri);
        let prefix = &request.prefix;
        require(
            prefix.is_empty() || valid_path(prefix),
            "Choose a registered binding path",
        )?;
        let index = self.index(uri)?;
        let slots = schema
            .properties(&owner.name, owner.namespace_uri.as_deref())
            .into_values()
            .filter(|property| writable_command(property))
            .map(|property| {
                let binding =
                    AuthoredValue::read(schema, document, owner, &property.name)?.model(index)?;
                let parameter = parameter_property(schema, owner, &property.name)
                    .map(|parameter| {
                        AuthoredValue::read(schema, document, owner, &parameter.name)?.model(index)
                    })
                    .transpose()?;
                Ok(CommandSlot {
                    property: property.name,
                    binding,
                    parameter,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let members = if slots.is_empty() {
            Vec::new()
        } else {
            let partial = if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}.")
            };
            crate::binding::members(schema, document, owner.id, "{Binding}", &partial)
                .into_iter()
                .filter(|(name, property)| !property.event && crate::is_xaml_identifier(name))
                .map(|(name, property)| {
                    let path = format!("{partial}{name}");
                    let command = command_type(&property);
                    let children = !command
                        && !crate::binding::members(
                            schema,
                            document,
                            owner.id,
                            "{Binding}",
                            &format!("{path}."),
                        )
                        .is_empty();
                    CommandMember {
                        path,
                        type_name: property.type_name,
                        command,
                        children,
                    }
                })
                .collect()
        };
        Ok(CommandBindingModel { slots, members })
    }

    pub fn plan_command_binding(
        &self,
        uri: &str,
        request: &CommandBindingRequest,
        change: &CommandBindingEdit,
    ) -> Result<crate::WorkspaceEdit> {
        let uri = self.document_uri(uri)?;
        let (document, owner) = self.command_binding_site(&uri, request)?;
        let schema = self.schema_for(&uri);
        require(
            schema
                .property(
                    &owner.name,
                    owner.namespace_uri.as_deref(),
                    &change.property,
                )
                .is_some_and(|property| writable_command(&property)),
            "Choose a writable command property",
        )?;
        let mut edits = Vec::new();
        if let CommandSelection::Bind { path } = &change.command {
            require(
                binding_member(schema, document, owner, path)
                    .is_some_and(|member| command_type(&member)),
                "Choose a registered ICommand binding path",
            )?;
        }
        AuthoredValue::read(schema, document, owner, &change.property)?.plan(
            document,
            owner,
            &change.property,
            &change.command,
            &mut edits,
        )?;
        if !matches!(change.parameter, CommandSelection::Keep) {
            let parameter =
                parameter_property(schema, owner, &change.property).ok_or_else(|| {
                    LanguageError("This command does not expose a writable parameter".into())
                })?;
            if let CommandSelection::Bind { path } = &change.parameter {
                require(
                    binding_member(schema, document, owner, path).is_some(),
                    "Choose a registered parameter binding path",
                )?;
            }
            AuthoredValue::read(schema, document, owner, &parameter.name)?.plan(
                document,
                owner,
                &parameter.name,
                &change.parameter,
                &mut edits,
            )?;
        }
        let patches = document.plan_attribute_patches(edits)?;
        if patches.is_empty() {
            return Ok(crate::WorkspaceEdit::default());
        }
        let mut candidate = document.snapshot_without_history();
        candidate.apply_text_patches(
            candidate.revision(),
            "Edit command binding",
            patches.clone(),
        )?;
        let source = super::source_review::reviewed_document(
            &uri,
            request.version,
            self.index(&uri)?,
            patches,
        )?;
        Ok(crate::WorkspaceEdit {
            document_changes: vec![source.edit],
        })
    }

    fn command_binding_site(
        &self,
        uri: &str,
        request: &CommandBindingRequest,
    ) -> Result<(&Document, &Node)> {
        require(
            self.version(uri) == Some(request.version),
            "The XAML command source changed. Reopen its binding editor",
        )?;
        let document = self
            .document(uri)
            .filter(|document| document.is_valid())
            .ok_or_else(|| {
                LanguageError("Repair the XAML before editing command bindings".into())
            })?;
        let start = self.index(uri)?.offset(request.position)?;
        let node = document
            .ast()
            .element_at(start)
            .and_then(|node| document.ast().node(node))
            .filter(|node| {
                node.name_span.start == start
                    && node.kind == ferroui_designer_core::NodeKind::Element
            })
            .ok_or_else(|| {
                LanguageError("Choose an authored control in the current XAML source".into())
            })?;
        Ok((document, node))
    }
}

fn parameter_property(schema: &Schema, owner: &Node, command: &str) -> Option<PropertyInfo> {
    schema
        .property(
            &owner.name,
            owner.namespace_uri.as_deref(),
            &format!("{command}Parameter"),
        )
        .filter(|property| !property.read_only && !property.attached && !property.event)
}

fn binding_member(
    schema: &Schema,
    document: &Document,
    owner: &Node,
    path: &str,
) -> Option<PropertyInfo> {
    if !valid_path(path) {
        return None;
    }
    let name = path.rsplit('.').next()?;
    crate::binding::members(schema, document, owner.id, "{Binding}", path)
        .remove(name)
        .filter(|property| !property.event)
}

fn valid_path(path: &str) -> bool {
    path.len() <= 4096 && path.split('.').all(crate::is_xaml_identifier)
}

fn writable_command(property: &PropertyInfo) -> bool {
    !property.read_only && !property.attached && command_type(property)
}

fn command_type(property: &PropertyInfo) -> bool {
    if property.event {
        return false;
    }
    if let Some(model) = &property.model_type {
        return model.name == "ICommand" && model.namespace == "System.Windows.Input";
    }
    let kind = crate::project_schema::unwrapped_type(&property.type_name);
    let kind = kind
        .strip_prefix("dyn ")
        .unwrap_or(kind)
        .trim_start_matches("::");
    matches!(kind, "ICommand" | "System.Windows.Input.ICommand")
        || kind.starts_with("ferroui_base::") && kind.ends_with("::ICommand")
}
