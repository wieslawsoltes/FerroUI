//! Accepted source snapshots and guarded handoff from independent preview trees to
//! the authoritative document, definition scope, selection and property inspector.
use super::*;
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewHostOptions, PreviewStageConfiguration,
    PreviewStageId, PreviewStageMode, PreviewStageSelection, PreviewStageSource,
    PreviewStageTarget, RuntimeNode,
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_inspect::TreeKind;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

struct FocusContext {
    uri: String,
    definition: Option<(NodeId, DefinitionPreviewOptions)>,
    environment: PreviewEnvironment,
    design_time_data: bool,
    states: Vec<(NodeId, String, String)>,
    interactive: bool,
    selection: Vec<NodeId>,
}

impl DesignerWorkspace {
    pub(super) fn preview_stage_sources(&self) -> Vec<PreviewStageSource> {
        let session = self.session.borrow();
        let uri = session.document_uri();
        let identity = super::project::document_identity(uri).unwrap_or_else(|_| uri.into());
        let mut sources = self
            .project
            .borrow()
            .tabs
            .iter()
            .filter(|(key, _)| **key != identity)
            .map(|(_, tab)| PreviewStageSource {
                uri: tab.preview_uri.clone(),
                document: tab.document.snapshot_without_history(),
                pending: false,
            })
            .collect::<Vec<_>>();
        sources.push(PreviewStageSource {
            uri: uri.into(),
            document: session.document().snapshot_without_history(),
            pending: self.pending_source.get(),
        });
        sources
    }

    pub(super) fn preview_stage_host_options(&self) -> (PreviewHostOptions, u64) {
        let session = self.session.borrow();
        (
            session.preview_host_options(),
            session.preview_host_options_epoch(),
        )
    }

    /// An optimistic identity for automation and asynchronous stage-file dialogs.
    /// Reading it does not construct previews, apply drafts or change selection.
    pub fn preview_stage_model(&self) -> String {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.preview_stage_epoch().hash(&mut hash);
        self.preview_stages_visible().hash(&mut hash);
        let session = self.session.borrow();
        session.document_uri().hash(&mut hash);
        session.preview_host_options_epoch().hash(&mut hash);
        drop(session);
        for source in self.preview_stage_sources() {
            source.uri.hash(&mut hash);
            source.pending.hash(&mut hash);
            source.document.revision().hash(&mut hash);
            source.document.source().hash(&mut hash);
            (Arc::as_ptr(&source.document.ast_snapshot()) as usize).hash(&mut hash);
        }
        if self.pending_source.get() {
            self.source.text().hash(&mut hash);
        }
        format!("{:016x}", hash.finish())
    }

    pub fn preview_stage_runtime_tree(
        &self,
        id: PreviewStageId,
        kind: TreeKind,
    ) -> Result<Vec<RuntimeNode>, String> {
        let mut rows = self.preview_stages.board.borrow().runtime_tree(id, kind)?;
        if !self.preview_stage_source_is_current(id) {
            for row in &mut rows {
                row.source_id = None;
            }
        }
        Ok(rows)
    }

    /// Check current buffers without synchronizing or constructing a preview.
    /// Pending editor drafts retire source mappings before the next UI refresh.
    pub fn preview_stage_source_is_current(&self, id: PreviewStageId) -> bool {
        let (_, epoch) = self.preview_stage_host_options();
        let sources = self.preview_stage_sources();
        let board = self.preview_stages.board.borrow();
        board.dependencies_are_current(id, &sources)
            && sources.iter().any(|source| {
                board.source_is_current(id, &source.uri, &source.document, source.pending, epoch)
            })
    }

    /// Image capture may use a paused frame while its accepted source and all
    /// native include dependencies remain exact. This never enables selection/input.
    pub fn preview_stage_image_source_is_current(&self, id: PreviewStageId) -> bool {
        let (_, epoch) = self.preview_stage_host_options();
        let sources = self.preview_stage_sources();
        let board = self.preview_stages.board.borrow();
        board.dependencies_are_current(id, &sources)
            && sources.iter().any(|source| {
                board.frame_is_current(id, &source.uri, &source.document, source.pending, epoch)
            })
    }

    pub fn select_preview_stage_runtime(
        self: &Rc<Self>,
        id: PreviewStageId,
        inspector_id: u64,
    ) -> Result<(), String> {
        let selection = {
            let board = self.preview_stages.board.borrow();
            let token = board.selection_token(id, inspector_id)?;
            board.resolve_selection(&token)?
        };
        self.focus_preview_stage_selection(&selection)?;
        self.preview_stages.board.borrow_mut().set_active(id)?;
        self.refresh(true);
        Ok(())
    }

    pub(super) fn focus_preview_stage_configuration(
        self: &Rc<Self>,
        configuration: &PreviewStageConfiguration,
    ) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply or repair the pending XAML before focusing a preview stage".into());
        }
        configuration.validate()?;
        let source = self
            .preview_stage_sources()
            .into_iter()
            .find(|source| source.uri == configuration.source_uri)
            .ok_or("Open the stage's source document before focusing it")?;
        if source.pending || !source.document.is_valid() {
            return Err("Repair the stage's source document before focusing it".into());
        }
        let definition = match &configuration.target {
            PreviewStageTarget::Document => None,
            PreviewStageTarget::Definition { anchor, options } => {
                let node = anchor.resolve(&source.document)?;
                crate::definition_projection::scope_for(
                    &source.document,
                    node,
                    options.clone(),
                    Vec::new(),
                )?;
                Some((node, options.clone()))
            }
        };
        let groups = VisualStateGroup::read(&source.document);
        let mut states = Vec::new();
        for state in &configuration.states {
            let owner = state.owner.resolve(&source.document)?;
            if !groups.iter().any(|group| {
                group.owner == owner
                    && group.name == state.group
                    && group
                        .states
                        .iter()
                        .any(|candidate| candidate.name == state.state)
            }) {
                return Err(format!(
                    "The authored state {} / {} no longer exists",
                    state.group, state.state
                ));
            }
            states.push((owner, state.group.clone(), state.state.clone()));
        }
        let previous = self.preview_stage_focus_context();
        let next = FocusContext {
            uri: configuration.source_uri.clone(),
            definition,
            environment: configuration.environment,
            design_time_data: configuration.design_time_data,
            states,
            interactive: configuration.mode == PreviewStageMode::Live,
            selection: Vec::new(),
        };
        if let Err(error) = self.apply_preview_stage_focus(&next) {
            let restoration = self.apply_preview_stage_focus(&previous);
            self.session.borrow_mut().select(previous.selection);
            self.refresh(true);
            return Err(match restoration {
                Ok(()) => error,
                Err(restore) => {
                    format!("{error}. Previous preview could not be restored: {restore}")
                }
            });
        }
        self.runtime_selection.set(None);
        self.refresh(true);
        Ok(())
    }

    pub(super) fn focus_preview_stage_selection(
        self: &Rc<Self>,
        selection: &PreviewStageSelection,
    ) -> Result<(), String> {
        let sources = self.preview_stage_sources();
        let current = sources.iter().any(|source| {
            !source.pending && selection.matches_source(&source.uri, &source.document)
        });
        if !current
            || self.pending_source.get()
            || !self
                .preview_stages
                .board
                .borrow()
                .dependencies_are_current(selection.stage, &sources)
        {
            return Err("The source changed after this preview was accepted; refresh the stage and select again".into());
        }
        self.focus_preview_stage_configuration(&selection.configuration)?;
        {
            let mut session = self.session.borrow_mut();
            if !selection.matches_source(session.document_uri(), session.document())
                || !super::selection_tools::authored_layer_available(&session, selection.node)
            {
                return Err("The selected control is unavailable in the active authored scope, or is hidden, locked or isolated".into());
            }
            session.select([selection.node]);
        }
        self.runtime_selection.set(None);
        self.reveal_source(selection.node);
        self.refresh(true);
        Ok(())
    }

    fn preview_stage_focus_context(&self) -> FocusContext {
        let session = self.session.borrow();
        FocusContext {
            uri: session.document_uri().into(),
            definition: session
                .definition_scope()
                .map(|scope| (scope.node, scope.options.clone())),
            environment: self.preview_environment(),
            design_time_data: session.design_time_enabled(),
            states: session.active_preview_states(),
            interactive: self.surface.interactive(),
            selection: session.selection().to_vec(),
        }
    }

    fn apply_preview_stage_focus(self: &Rc<Self>, context: &FocusContext) -> Result<(), String> {
        self.activate_document(&context.uri)?;
        {
            let mut session = self.session.borrow_mut();
            session.clear_interaction_preview();
            let same_definition = match (&context.definition, session.definition_scope()) {
                (None, None) => true,
                (Some((node, options)), Some(scope)) => {
                    scope.node == *node && scope.options == *options
                }
                _ => false,
            };
            if !same_definition {
                if let Some((node, options)) = &context.definition {
                    session.enter_definition_scope_with_options(*node, options.clone())?;
                } else {
                    session.exit_definition_scope()?;
                }
            }
            if session.design_time_enabled() != context.design_time_data {
                session.set_design_time_enabled(context.design_time_data)?;
            }
            if !matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }) {
                return Err("The source could not be loaded in the authoring preview".into());
            }
        }
        self.set_preview_environment(context.environment)?;
        self.surface.set_interactive(context.interactive);
        self.artboard.update_layout();
        {
            let mut session = self.session.borrow_mut();
            // The board can hide the authoring scroll viewer. Explicitly lay out
            // its accepted native host so a newly focused template realizes its
            // parts before source maps and the property inspector use them.
            let host = session.preview_host();
            let size =
                ferroui_base::Size::new(context.environment.width, context.environment.height);
            host.measure(size);
            host.arrange(ferroui_base::Rect::from_size(size));
            host.update_layout();
            let document = session.document().snapshot_without_history();
            session.preview.rebuild_map(&document);
            for (owner, group, state) in &context.states {
                session.preview_state(*owner, group, Some(state))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "workspace_stage_sources_tests.rs"]
mod tests;
