//! Native session transaction boundary for portable component and resource authoring.
use crate::DesignerSession;
use ferroui_designer_core::{Ast, Document, Edit, MarkupValue, NodeId};
use ferroui_designer_tools::{
    ComponentAction, ComponentCatalog, ComponentEntry, ComponentKind, ComponentReference,
    ComponentResourceContext,
};
use std::sync::Arc;

/// Prefer the authored framework alias when building structured value expressions.
/// Application extensions with the same local name retain their own semantics.
pub(crate) fn property_binding_extension(
    document: &Document,
    node: NodeId,
    value: Option<&str>,
) -> String {
    let namespaces = document.ast().namespaces(node);
    let qualified = |prefix: &str| {
        if prefix.is_empty() {
            "Binding".into()
        } else {
            format!("{prefix}:Binding")
        }
    };
    if let Some(MarkupValue::Extension(extension)) =
        value.and_then(|value| MarkupValue::parse(value).ok())
    {
        let (prefix, name) = extension
            .name
            .split_once(':')
            .unwrap_or(("", &extension.name));
        if matches!(name, "Binding" | "StaticResource" | "DynamicResource")
            && namespaces
                .get(prefix)
                .is_some_and(|uri| uri == "https://github.com/ferroui")
        {
            return qualified(prefix);
        }
    }
    namespaces
        .iter()
        .find(|(_, uri)| uri.as_str() == "https://github.com/ferroui")
        .map_or_else(|| "Binding".into(), |(prefix, _)| qualified(prefix))
}

#[derive(Clone, Debug)]
pub struct ComponentEditPreview {
    pub description: String,
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    selection: Vec<NodeId>,
    generation: u64,
    resource_contexts: Vec<(NodeId, Option<ComponentResourceContext>)>,
    edits: Vec<Edit>,
}
impl ComponentEditPreview {
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        session.ensure_writable().is_ok()
            && session.document().is_valid()
            && self.uri == session.document_uri()
            && self.revision == session.document().revision()
            && Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            && self.selection == session.selection()
            && self.generation == session.generation()
            && self
                .resource_contexts
                .iter()
                .all(|(target, context)| session.component_resource_context(*target) == *context)
    }
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }
}

impl DesignerSession {
    pub fn component_catalog(&self) -> Result<ComponentCatalog, String> {
        ferroui_designer_tools::ComponentCatalog::from_document(self.document())
            .map_err(|error| error.to_string())
    }
    pub fn visible_component_resources(
        &self,
        target: NodeId,
    ) -> Result<Vec<ComponentEntry>, String> {
        match self.component_resource_context(target) {
            Some(context) => ferroui_designer_tools::visible_component_resources_in_context(
                self.document(),
                target,
                &context,
            ),
            None => ferroui_designer_tools::visible_component_resources(self.document(), target),
        }
        .map_err(|error| error.to_string())
    }

    /// Theme lookup follows the accepted native target and its complete variant chain.
    /// Opaque custom keys or an invalid/outdated preview retain conservative lookup.
    pub fn component_resource_context(&self, target: NodeId) -> Option<ComponentResourceContext> {
        if !matches!(self.preview_status(), crate::PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            return None;
        }
        let mut current = Some(target);
        let mut theme = None;
        while let Some(id) = current {
            if let Some(element) = self
                .preview
                .visual(id)
                .and_then(|visual| visual.cast::<ferroui_base::StyledElement>())
            {
                theme = element.actual_theme_variant();
                break;
            }
            current = self.document().ast().node(id).and_then(|node| node.parent);
        }
        let theme = theme.or_else(|| self.preview.host.actual_theme_variant());
        let mut context = ComponentResourceContext::default();
        if let Some(theme) = theme {
            context.theme = Some(theme.key().as_str()?.to_owned());
            let mut inherited = theme.inherit_variant();
            while let Some(variant) = inherited {
                context
                    .inherited_themes
                    .push(variant.key().as_str()?.to_owned());
                inherited = variant.inherit_variant();
            }
        }
        Some(context)
    }
    pub fn component_resource_references(
        &self,
        resource: NodeId,
    ) -> Result<Vec<ComponentReference>, String> {
        ferroui_designer_tools::component_resource_references(self.document(), resource)
            .map_err(|error| error.to_string())
    }
    pub fn prepare_component_edit(
        &self,
        action: &ComponentAction,
    ) -> Result<ComponentEditPreview, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits = if let ComponentAction::ApplyResource {
            resource,
            targets,
            property,
            dynamic,
        } = action
        {
            if targets.is_empty() {
                return Err("Select at least one target".into());
            }
            let mut edits = Vec::new();
            for target in targets
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
            {
                let action = ComponentAction::ApplyResource {
                    resource: *resource,
                    targets: vec![target],
                    property: property.clone(),
                    dynamic: *dynamic,
                };
                let planned = match self.component_resource_context(target) {
                    Some(context) => ferroui_designer_tools::plan_component_action_in_context(
                        self.document(),
                        &action,
                        &context,
                    ),
                    None => ferroui_designer_tools::plan_component_action(self.document(), &action),
                }
                .map_err(|error| error.to_string())?;
                edits.extend(planned);
            }
            edits
        } else {
            ferroui_designer_tools::plan_component_action(self.document(), action)
                .map_err(|error| error.to_string())?
        };
        let description = match action {
            ComponentAction::CreateResource { .. } => "Create resource",
            ComponentAction::DuplicateResource { .. } => "Duplicate resource",
            ComponentAction::RenameResource { .. } => "Rename resource and references",
            ComponentAction::RemoveResource { .. } => "Delete resource",
            ComponentAction::ApplyResource { .. } => "Apply resource",
            ComponentAction::SetProperty { .. } => "Edit definition property",
            ComponentAction::SetObjectProperty { .. } => "Edit definition object",
            ComponentAction::SetSetter { .. } | ComponentAction::SetSetterObject { .. } => {
                "Edit theme setter"
            }
            ComponentAction::SetResourceValue { .. } => "Edit resource value",
            ComponentAction::ReplaceDefinition { .. } => "Edit definition source",
            ComponentAction::CreateTheme { .. } => "Create control theme",
            ComponentAction::CreateTemplate { .. } => "Create template",
            ComponentAction::CopyTemplate { .. } => "Create editable template copy",
            ComponentAction::ExtractProperty { .. } => "Extract property resource",
        }
        .to_owned();
        Ok(ComponentEditPreview {
            description,
            uri: self.document_uri().to_owned(),
            revision: self.document().revision(),
            ast: self.document().ast_snapshot(),
            selection: self.selection().to_vec(),
            generation: self.generation(),
            resource_contexts: match action {
                ComponentAction::ApplyResource { targets, .. } => targets
                    .iter()
                    .map(|target| (*target, self.component_resource_context(*target)))
                    .collect(),
                _ => Vec::new(),
            },
            edits,
        })
    }
    pub fn commit_component_edit(&mut self, preview: &ComponentEditPreview) -> Result<(), String> {
        if !preview.is_current(self) {
            return Err(
                "The document, selection or editor context changed; reopen this component edit"
                    .into(),
            );
        }
        if preview.edits.is_empty() {
            return Ok(());
        }
        self.apply_at(
            preview.revision,
            &preview.description,
            preview.edits.clone(),
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }
    pub fn edit_component(&mut self, action: ComponentAction) -> Result<(), String> {
        let preview = self.prepare_component_edit(&action)?;
        self.commit_component_edit(&preview)
    }

    /// Create an independent template/theme and apply it to one instance in a single
    /// transaction. Shared definitions keep their original source and other instances.
    pub fn copy_definition_for_instance(
        &mut self,
        definition: NodeId,
        target: NodeId,
        property: &str,
        key: &str,
    ) -> Result<NodeId, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let entry = self
            .component_catalog()?
            .entries
            .into_iter()
            .find(|entry| entry.node == definition)
            .ok_or("The linked definition no longer exists")?;
        let owner = entry.owner.unwrap_or(target);
        let action = match entry.kind {
            ComponentKind::ControlTemplate | ComponentKind::DataTemplate => {
                ComponentAction::CopyTemplate {
                    template: definition,
                    owner,
                    key: key.into(),
                }
            }
            ComponentKind::ControlTheme if entry.owner.is_some() => {
                ComponentAction::DuplicateResource {
                    resource: definition,
                    key: key.into(),
                }
            }
            _ => {
                return Err(
                    "Select an authored template or named control theme to edit a copy".into(),
                );
            }
        };
        let mut edits = ferroui_designer_tools::plan_component_action(self.document(), &action)
            .map_err(|error| error.to_string())?;
        let mut candidate = self.document().clone();
        candidate
            .apply("Prepare definition copy", edits.clone())
            .map_err(|error| error.to_string())?;
        let copied = ComponentCatalog::from_document(&candidate)
            .map_err(|error| error.to_string())?
            .entries
            .into_iter()
            .find(|entry| {
                entry.key.as_deref() == Some(key)
                    && (entry.owner == Some(owner) || entry.scope == owner)
            })
            .map(|entry| entry.node)
            .ok_or("The copied definition was not found")?;
        edits.extend(
            ferroui_designer_tools::plan_component_action(
                &candidate,
                &ComponentAction::ApplyResource {
                    resource: copied,
                    targets: vec![target],
                    property: property.into(),
                    dynamic: false,
                },
            )
            .map_err(|error| error.to_string())?,
        );
        self.apply("Create definition copy and apply to instance", edits)
            .map_err(|error| error.to_string())?;
        Ok(copied)
    }
    pub fn set_property_object_source(
        &mut self,
        nodes: &[NodeId],
        property: &str,
        source: Option<&str>,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits =
            ferroui_designer_tools::plan_property_object(self.document(), nodes, property, source)
                .map_err(|error| error.to_string())?;
        if !edits.is_empty() {
            self.apply(format!("Edit {property} object"), edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "component_session_tests.rs"]
mod tests;
