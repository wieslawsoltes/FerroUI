use super::AuthoredRsxFrame;
use ferroui_designer_animation::{StateGroupEdit, StatePreviewRoute, VisualStateGroup};
use ferroui_designer_core::{NodeId, SourceSpan, TextPatch};

impl AuthoredRsxFrame {
    pub(crate) fn hosted_state_owner(&self, owner: NodeId) -> Option<(u32, bool)> {
        self.state_owners
            .get(&owner)
            .map(|mapping| (mapping.line, mapping.can_create))
    }

    pub(crate) fn hosted_state_span(
        &self,
        group: &VisualStateGroup,
        route: Option<&StatePreviewRoute>,
    ) -> Option<SourceSpan> {
        let authored = self.states.get(&(group.owner, group.name.clone()))?;
        if let Some(route) = route.filter(|route| !route.endpoint) {
            return authored
                .transition_span(route.transition.from.as_deref(), &route.transition.to);
        }
        Some(authored.declaration)
    }

    pub(crate) fn plan_hosted_state_group(
        &self,
        edit: &StateGroupEdit,
    ) -> Result<Vec<TextPatch>, String> {
        let (owner, name) = edit.identity();
        let index = self
            .state_owners
            .get(&owner)
            .ok_or("Choose a Rust control with editable inline states")?
            .ordinal;
        let projection = self.catalog.project(&self.source, self.invocation.start)?;
        match edit {
            StateGroupEdit::Upsert { model } => projection.upsert_state_group(index, model),
            StateGroupEdit::Remove { .. } => projection.remove_state_group(index, name),
        }
    }

    pub(crate) fn patch_hosted_state_values(
        &self,
        group: &VisualStateGroup,
    ) -> Result<Vec<TextPatch>, String> {
        self.states
            .get(&(group.owner, group.name.clone()))
            .ok_or("The state declaration changed")?
            .patch_model(group)
    }
}
