//! Native session transaction boundary for portable component and resource authoring.
use crate::DesignerSession;
use ferroui_designer_core::{Ast, Edit, NodeId};
use ferroui_designer_tools::{
    ComponentAction, ComponentCatalog, ComponentEntry, ComponentReference,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ComponentEditPreview {
    pub description: String,
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    selection: Vec<NodeId>,
    generation: u64,
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
        ferroui_designer_tools::visible_component_resources(self.document(), target)
            .map_err(|error| error.to_string())
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
        let edits = ferroui_designer_tools::plan_component_action(self.document(), action)
            .map_err(|error| error.to_string())?;
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
