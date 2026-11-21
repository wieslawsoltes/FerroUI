use crate::{DefinitionPreviewOptions, DefinitionScope, DesignerSession};
use ferroui_designer_core::NodeId;

impl DesignerSession {
    /// Inspect and author a reusable definition in a native sample host. Entering a
    /// scope changes no source, history, resource ownership or collaboration state.
    pub fn enter_definition_scope(&mut self, node: NodeId) -> Result<(), String> {
        self.enter_definition_scope_with_options(node, DefinitionPreviewOptions::default())
    }

    pub fn enter_definition_scope_with_options(
        &mut self,
        node: NodeId,
        options: DefinitionPreviewOptions,
    ) -> Result<(), String> {
        self.enter_definition(node, options, None)
    }

    pub(crate) fn enter_definition(
        &mut self,
        node: NodeId,
        options: DefinitionPreviewOptions,
        item_template: Option<crate::item_template_samples::ItemTemplatePreview>,
    ) -> Result<(), String> {
        if !self.document.is_valid() {
            return Err("Repair the XAML source before opening a definition preview".into());
        }
        let return_selection = self.preview.definition_scope.as_ref().map_or_else(
            || self.selection.clone(),
            |scope| scope.return_selection.clone(),
        );
        let mut scope = crate::definition_projection::scope_for(
            &self.document,
            node,
            options,
            return_selection,
        )?;
        scope.item_template = item_template;
        self.stop_definition_preview();
        self.preview
            .set_definition_scope(Some(scope), &self.document)?;
        self.definition_scope_epoch = self.definition_scope_epoch.wrapping_add(1);
        self.refresh_layer_visuals();
        let first_visual = self
            .document
            .ast()
            .subtree(node)
            .find(|node| self.preview.visual(node.id).is_some())
            .map(|node| node.id);
        self.select([first_visual.unwrap_or(node)]);
        Ok(())
    }

    pub fn exit_definition_scope(&mut self) -> Result<(), String> {
        if self.rust_template_property().is_some() {
            return self.exit_rust_template();
        }
        let Some(scope) = self.preview.definition_scope.as_ref() else {
            return Ok(());
        };
        if !self.document.is_valid() {
            return Err("Repair the XAML source before returning to the document preview".into());
        }
        let selection = scope.return_selection.clone();
        self.stop_definition_preview();
        self.preview.set_definition_scope(None, &self.document)?;
        self.definition_scope_epoch = self.definition_scope_epoch.wrapping_add(1);
        self.refresh_layer_visuals();
        self.select(selection);
        Ok(())
    }

    pub fn definition_scope(&self) -> Option<&DefinitionScope> {
        self.preview.definition_scope.as_ref()
    }

    pub(crate) fn definition_consumer(&self) -> Option<&str> {
        self.rust.as_ref().and_then(|rust| rust.template_consumer())
            .or_else(|| self.included_template_consumer())
    }

    /// Changes after accepted scope, design-data and preview-environment transitions,
    /// including returning to the same definition. Source revisions alone cannot retire
    /// an editor or spatial gesture when native preview conditions change.
    pub fn definition_scope_epoch(&self) -> u64 {
        self.definition_scope_epoch
    }

    pub fn definition_preview_options(&self) -> DefinitionPreviewOptions {
        self.definition_scope()
            .map(|scope| scope.options.clone())
            .unwrap_or_default()
    }

    fn stop_definition_preview(&mut self) {
        self.animation.player.pause();
        self.animation.native.clear();
        self.preview.clear_animation();
        self.forget_interaction_preview();
    }
}
