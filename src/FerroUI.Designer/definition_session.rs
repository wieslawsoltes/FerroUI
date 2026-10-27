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
        if !self.document.is_valid() {
            return Err("Repair the XAML source before opening a definition preview".into());
        }
        let return_selection = self.preview.definition_scope.as_ref().map_or_else(
            || self.selection.clone(),
            |scope| scope.return_selection.clone(),
        );
        let scope = crate::definition_projection::scope_for(
            &self.document,
            node,
            options,
            return_selection,
        )?;
        self.stop_definition_preview();
        self.preview
            .set_definition_scope(Some(scope), &self.document)?;
        self.definition_scope_epoch = self.definition_scope_epoch.wrapping_add(1);
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
        self.select(selection);
        Ok(())
    }

    pub fn definition_scope(&self) -> Option<&DefinitionScope> {
        self.preview.definition_scope.as_ref()
    }

    /// Changes after every accepted preview-scope transition, including returning to
    /// the same definition. Source revisions alone cannot retire a retained editor.
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
