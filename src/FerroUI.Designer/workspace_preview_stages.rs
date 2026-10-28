//! Workspace presentation for independent native preview scenarios. The board is
//! global to the workspace; pinned source views remain visible across document tabs.
use super::*;
use crate::{
    PreviewEnvironment, PreviewFlowDirection, PreviewStageAction, PreviewStageBoard,
    PreviewStageComparison, PreviewStageConfiguration, PreviewStageEditor, PreviewStageId,
    PreviewStageInfo, PreviewStageManifest, PreviewStageMode, PreviewStageStatus,
    PreviewStageTarget, PreviewStageView, PreviewTheme,
};
use ferroui_controls::Flyout;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct PreviewStagesUi {
    pub(super) board: RefCell<PreviewStageBoard>,
    visible: Cell<bool>,
    refreshing: Cell<bool>,
    view: RefCell<Option<Rc<PreviewStageView>>>,
    attached: RefCell<BTreeSet<PreviewStageId>>,
    mapped_epoch: Cell<u64>,
    editor: RefCell<Option<Rc<PreviewStageEditor>>>,
    editor_epoch: Cell<Option<u64>>,
    flyout: RefCell<Option<Ref<Flyout>>>,
}

#[cfg(test)]
#[path = "preview_stage_ui_tests.rs"]
mod tests;

impl Default for PreviewStagesUi {
    fn default() -> Self {
        Self {
            board: RefCell::new(PreviewStageBoard::new()),
            visible: Cell::new(false),
            refreshing: Cell::new(false),
            view: RefCell::new(None),
            attached: RefCell::new(BTreeSet::new()),
            mapped_epoch: Cell::new(0),
            editor: RefCell::new(None),
            editor_epoch: Cell::new(None),
            flyout: RefCell::new(None),
        }
    }
}

impl DesignerWorkspace {
    pub fn preview_stages(&self) -> Vec<PreviewStageInfo> {
        self.preview_stages.board.borrow().stages()
    }
    pub fn preview_stage(&self, id: PreviewStageId) -> Option<PreviewStageInfo> {
        self.preview_stages.board.borrow().stage(id)
    }
    pub fn preview_stage_epoch(&self) -> u64 {
        self.preview_stages.board.borrow().epoch()
    }
    pub fn preview_stage_control(&self, id: PreviewStageId) -> Option<Ref<Control>> {
        self.preview_stages.board.borrow().control(id)
    }
    pub fn preview_stages_visible(&self) -> bool {
        self.preview_stages.visible.get()
    }
    pub fn preview_stage_manifest(&self) -> PreviewStageManifest {
        self.preview_stages.board.borrow().manifest()
    }
    pub fn preview_stage_editor(&self) -> Option<Rc<PreviewStageEditor>> {
        self.preview_stages.editor.borrow().clone()
    }
    pub fn preview_stage_view(&self) -> Option<Rc<PreviewStageView>> {
        self.preview_stages.view.borrow().clone()
    }

    pub fn add_preview_stage(
        self: &Rc<Self>,
        configuration: PreviewStageConfiguration,
    ) -> Result<PreviewStageId, String> {
        self.sync_preview_stage_sources()?;
        let result = self.preview_stages.board.borrow_mut().add(configuration);
        self.refresh_preview_stages();
        result
    }
    pub fn update_preview_stage(
        self: &Rc<Self>,
        id: PreviewStageId,
        configuration: PreviewStageConfiguration,
    ) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        let result = self
            .preview_stages
            .board
            .borrow_mut()
            .update(id, configuration);
        self.refresh_preview_stages();
        result
    }
    pub fn duplicate_preview_stage(
        self: &Rc<Self>,
        id: PreviewStageId,
    ) -> Result<PreviewStageId, String> {
        self.sync_preview_stage_sources()?;
        let result = self.preview_stages.board.borrow_mut().duplicate(id);
        self.refresh_preview_stages();
        result
    }
    pub fn remove_preview_stage(self: &Rc<Self>, id: PreviewStageId) -> Result<(), String> {
        let result = self.preview_stages.board.borrow_mut().remove(id);
        self.refresh_preview_stages();
        result
    }
    pub fn move_preview_stage(
        self: &Rc<Self>,
        id: PreviewStageId,
        index: usize,
    ) -> Result<(), String> {
        let result = self.preview_stages.board.borrow_mut().move_stage(id, index);
        self.refresh_preview_stages();
        result
    }
    pub fn set_preview_stage_paused(
        self: &Rc<Self>,
        id: PreviewStageId,
        paused: bool,
    ) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        let result = self
            .preview_stages
            .board
            .borrow_mut()
            .set_paused(id, paused);
        self.refresh_preview_stages();
        result
    }
    pub fn refresh_preview_stage(self: &Rc<Self>, id: PreviewStageId) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        let result = self.preview_stages.board.borrow_mut().refresh(id);
        self.refresh_preview_stages();
        result
    }
    pub fn refresh_all_preview_stages(self: &Rc<Self>) -> Vec<(PreviewStageId, String)> {
        if let Err(error) = self.sync_preview_stage_sources() {
            return vec![(0, error)];
        }
        let errors = self.preview_stages.board.borrow_mut().refresh_all();
        self.refresh_preview_stages();
        errors
    }
    pub fn compare_preview_stage(
        self: &Rc<Self>,
        id: PreviewStageId,
        variants: Vec<(String, PreviewEnvironment)>,
    ) -> Result<Vec<PreviewStageId>, String> {
        self.sync_preview_stage_sources()?;
        let result = self
            .preview_stages
            .board
            .borrow_mut()
            .compare_variants(id, variants);
        self.refresh_preview_stages();
        result
    }
    /// Explicit stage activation brings the correct source, definition and environment
    /// into the authoritative authoring preview before the inspector changes context.
    pub fn set_active_preview_stage(self: &Rc<Self>, id: PreviewStageId) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        let info = self
            .preview_stage(id)
            .ok_or("The preview stage no longer exists")?;
        self.focus_preview_stage_configuration(&info.configuration)?;
        self.preview_stages.board.borrow_mut().set_active(id)?;
        self.refresh(true);
        Ok(())
    }
    pub fn restore_preview_stage_manifest(
        self: &Rc<Self>,
        manifest: PreviewStageManifest,
    ) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        self.preview_stages
            .board
            .borrow_mut()
            .restore_manifest(manifest)?;
        self.refresh_preview_stages();
        Ok(())
    }
    pub fn set_preview_stages_visible(self: &Rc<Self>, visible: bool) -> Result<(), String> {
        self.cancel_canvas_navigation();
        self.cancel_canvas_guide_drag();
        if visible {
            self.ensure_preview_stage_view();
            self.sync_preview_stage_sources()?;
            if self.preview_stages.board.borrow().stages().is_empty() {
                let configuration = self.current_preview_stage_configuration("Current view")?;
                self.preview_stages.board.borrow_mut().add(configuration)?;
            }
        }
        self.preview_stages.visible.set(visible);
        self.refresh_preview_stage_visibility();
        self.refresh_preview_stages();
        Ok(())
    }

    pub(super) fn connect_preview_stages(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<Button>("StagesToggle")
            .click(move |_, event| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(
                        workspace.set_preview_stages_visible(!workspace.preview_stages_visible()),
                    );
                }
                event.set_handled(true);
            });
    }

    pub(super) fn refresh_preview_stage_visibility(&self) {
        let canvas = self.view_mode() != super::shell::WorkspaceViewMode::Xaml;
        let board = canvas && self.preview_stages_visible();
        self.root
            .get_control::<Control>("PreviewStagesHost")
            .set_is_visible(board);
        self.root
            .get_control::<Control>("ArtboardScroll")
            .set_is_visible(canvas && !board);
        self.root
            .get_control::<Control>("CanvasToolShelf")
            .set_is_visible(canvas && !board);
        self.root
            .get_control::<Button>("StagesToggle")
            .classes()
            .set("active", self.preview_stages_visible());
        for name in [
            "CanvasSizeIcon",
            "CanvasSize",
            "PreviewEnvironment",
            "CanvasNavigationCommands",
        ] {
            self.root
                .get_control::<Control>(name)
                .set_is_visible(!board);
        }
    }

    fn ensure_preview_stage_view(self: &Rc<Self>) {
        if self.preview_stages.view.borrow().is_some() {
            return;
        }
        let view = PreviewStageView::new(&self.root);
        let weak = Rc::downgrade(self);
        view.on_action(move |action, anchor| {
            weak.upgrade()
                .ok_or("The workspace was closed")?
                .handle_preview_stage_action(action, &anchor)
        });
        self.root
            .get_control::<ContentControl>("PreviewStagesHost")
            .set_content(Some(Control::boxed(view.control())));
        *self.preview_stages.view.borrow_mut() = Some(view);
    }

    fn sync_preview_stage_sources(&self) -> Result<(), String> {
        let active = self.session.borrow().document_uri().to_owned();
        let sources = self.preview_stage_sources();
        let (options, epoch) = self.preview_stage_host_options();
        self.preview_stages
            .board
            .borrow_mut()
            .sync_sources(&active, sources, options, epoch)
    }

    /// Called before the regular source/selection refresh early return. Board state
    /// has its own epochs and must remain responsive while authored XAML is unchanged.
    pub(super) fn refresh_preview_stages(self: &Rc<Self>) {
        if !self.preview_stages_visible() && self.preview_stages.board.borrow().stages().is_empty()
        {
            return;
        }
        if self.preview_stages.refreshing.replace(true) {
            return;
        }
        let result = (|| -> Result<(), String> {
            self.sync_preview_stage_sources()?;
            self.refresh_preview_stage_visibility();
            let Some(view) = self.preview_stages.view.borrow().clone() else {
                return Ok(());
            };
            let uri = self.session.borrow().document_uri().to_owned();
            let (all, active) = {
                let board = self.preview_stages.board.borrow();
                (board.stages(), board.active())
            };
            let visible = all
                .iter()
                .filter(|stage| stage.configuration.pinned || stage.configuration.source_uri == uri)
                .cloned()
                .collect::<Vec<_>>();
            let hosts = {
                let board = self.preview_stages.board.borrow();
                visible
                    .iter()
                    .filter_map(|info| board.control(info.id).map(|control| (info.id, control)))
                    .collect::<BTreeMap<_, _>>()
            };
            view.update(&visible, &hosts, active, &uri, all.len());
            let next = visible.iter().map(|info| info.id).collect::<BTreeSet<_>>();
            let newly_attached = next
                .difference(&self.preview_stages.attached.borrow())
                .copied()
                .collect::<Vec<_>>();
            *self.preview_stages.attached.borrow_mut() = next;
            // Theme resources and native state scenarios need a rooted logical tree.
            // Retry once after mounting a card; paused accepted frames stay untouched.
            let mut refreshed = false;
            for id in newly_attached {
                if let Some(info) = self.preview_stage(id) {
                    if matches!(info.status, PreviewStageStatus::Failed { .. })
                        && (info.configuration.auto_update || info.rendered_revision.is_none())
                    {
                        let _ = self.preview_stages.board.borrow_mut().refresh(id);
                        refreshed = true;
                    }
                }
            }
            if refreshed {
                let (all, active) = {
                    let board = self.preview_stages.board.borrow();
                    (board.stages(), board.active())
                };
                let visible = all
                    .iter()
                    .filter(|stage| {
                        stage.configuration.pinned || stage.configuration.source_uri == uri
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                view.update(&visible, &hosts, active, &uri, all.len());
            }
            let epoch = self.preview_stage_epoch();
            if self.preview_stages.mapped_epoch.replace(epoch) != epoch {
                view.control().update_layout();
                self.preview_stages
                    .board
                    .borrow_mut()
                    .refresh_runtime_mappings();
            }
            if self
                .preview_stages
                .editor_epoch
                .get()
                .is_some_and(|epoch| epoch != self.preview_stage_epoch())
            {
                if let Some(editor) = self.preview_stages.editor.borrow().as_ref() {
                    editor.retire();
                    editor.show_error("The sources or preview stages changed. Close and reopen this editor to apply a new stage configuration.");
                }
            }
            Ok(())
        })();
        self.preview_stages.refreshing.set(false);
        if let Err(error) = result {
            if let Some(view) = self.preview_stages.view.borrow().as_ref() {
                view.show_error(&error);
            }
        }
    }

    fn accepted_preview_stage(
        &self,
        id: PreviewStageId,
        epoch: u64,
    ) -> Result<PreviewStageInfo, String> {
        let info = self
            .preview_stage(id)
            .ok_or("This preview stage was removed; reopen the stage board")?;
        if info.epoch != epoch {
            return Err("This preview stage changed; use its current controls".into());
        }
        Ok(info)
    }

    fn handle_preview_stage_action(
        self: &Rc<Self>,
        action: PreviewStageAction,
        anchor: &Control,
    ) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        match action {
            PreviewStageAction::Add => self.show_preview_stage_editor(None, anchor),
            PreviewStageAction::Edit { stage, epoch } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.show_preview_stage_editor(Some(stage), anchor)
            }
            PreviewStageAction::Duplicate { stage, epoch } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.duplicate_preview_stage(stage).map(|_| ())
            }
            PreviewStageAction::Remove { stage, epoch } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.remove_preview_stage(stage)
            }
            PreviewStageAction::Move {
                stage,
                epoch,
                offset,
            } => {
                self.accepted_preview_stage(stage, epoch)?;
                let stages = self.preview_stages();
                let index = stages
                    .iter()
                    .position(|info| info.id == stage)
                    .ok_or("Unknown stage")?;
                let index = (index as i64 + offset as i64)
                    .clamp(0, stages.len().saturating_sub(1) as i64)
                    as usize;
                self.move_preview_stage(stage, index)
            }
            PreviewStageAction::Pin {
                stage,
                epoch,
                pinned,
            } => {
                let mut info = self.accepted_preview_stage(stage, epoch)?;
                info.configuration.pinned = pinned;
                self.update_preview_stage(stage, info.configuration)
            }
            PreviewStageAction::Mode { stage, epoch, mode } => {
                let mut info = self.accepted_preview_stage(stage, epoch)?;
                info.configuration.mode = mode;
                self.update_preview_stage(stage, info.configuration)
            }
            PreviewStageAction::Pause {
                stage,
                epoch,
                paused,
            } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.set_preview_stage_paused(stage, paused)
            }
            PreviewStageAction::Refresh { stage, epoch } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.refresh_preview_stage(stage)
            }
            PreviewStageAction::Focus { stage, epoch } => {
                self.accepted_preview_stage(stage, epoch)?;
                self.set_active_preview_stage(stage)?;
                self.set_preview_stages_visible(false)
            }
            PreviewStageAction::Select {
                stage,
                epoch,
                position,
            } => {
                let info = self.accepted_preview_stage(stage, epoch)?;
                if info.configuration.mode != PreviewStageMode::Select {
                    return Err(
                        "Switch this stage to Select before choosing authored controls".into(),
                    );
                }
                self.preview_stages
                    .board
                    .borrow_mut()
                    .refresh_runtime_mappings();
                let selection = {
                    let board = self.preview_stages.board.borrow();
                    let token = board.hit_test_selection(stage, position)?;
                    board.resolve_selection(&token)?
                };
                self.focus_preview_stage_selection(&selection)?;
                self.preview_stages.board.borrow_mut().set_active(stage)?;
                self.refresh(true);
                Ok(())
            }
            PreviewStageAction::Compare(comparison) => {
                self.add_preview_stage_comparison(comparison)
            }
            PreviewStageAction::RefreshAll => {
                let errors = self.refresh_all_preview_stages();
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors
                        .into_iter()
                        .map(|(id, error)| format!("Stage {id}: {error}"))
                        .collect::<Vec<_>>()
                        .join("\n"))
                }
            }
            PreviewStageAction::Save => {
                self.request_save_preview_stages();
                Ok(())
            }
            PreviewStageAction::Load => {
                self.request_load_preview_stages();
                Ok(())
            }
            PreviewStageAction::Back => self.set_preview_stages_visible(false),
        }
    }

    fn current_preview_stage_configuration(
        &self,
        name: &str,
    ) -> Result<PreviewStageConfiguration, String> {
        let session = self.session.borrow();
        let mut configuration = PreviewStageConfiguration::new(
            name,
            session.document_uri(),
            self.preview_environment(),
        );
        configuration.design_time_data = session.design_time_enabled();
        configuration.mode = if self.surface.interactive() {
            PreviewStageMode::Live
        } else {
            PreviewStageMode::Select
        };
        configuration.states = session
            .active_preview_states()
            .into_iter()
            .map(|(owner, group, state)| {
                Ok(crate::PreviewStageState {
                    owner: crate::PreviewSourceAnchor::capture(session.document(), owner)?,
                    group,
                    state,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if let Some(scope) = session.definition_scope() {
            configuration.target = PreviewStageTarget::Definition {
                anchor: crate::PreviewSourceAnchor::capture(session.document(), scope.node)?,
                options: scope.options.clone(),
            };
        }
        Ok(configuration)
    }

    fn add_preview_stage_comparison(
        self: &Rc<Self>,
        comparison: PreviewStageComparison,
    ) -> Result<(), String> {
        let active = self
            .preview_stages
            .board
            .borrow()
            .active()
            .ok_or("Add a stage before comparing variants")?;
        let stage = self
            .preview_stage(active)
            .ok_or("The active preview stage was removed")?;
        let base = stage.configuration.environment;
        let values = match comparison {
            PreviewStageComparison::Sizes => vec![
                (
                    "Phone".into(),
                    PreviewEnvironment {
                        width: 390.0,
                        height: 844.0,
                        ..base
                    },
                ),
                (
                    "Tablet".into(),
                    PreviewEnvironment {
                        width: 768.0,
                        height: 1024.0,
                        ..base
                    },
                ),
                (
                    "Desktop".into(),
                    PreviewEnvironment {
                        width: 1280.0,
                        height: 800.0,
                        ..base
                    },
                ),
            ],
            PreviewStageComparison::Themes => vec![
                (
                    format!("{} · Light", stage.configuration.name),
                    PreviewEnvironment {
                        theme: PreviewTheme::Light,
                        ..base
                    },
                ),
                (
                    format!("{} · Dark", stage.configuration.name),
                    PreviewEnvironment {
                        theme: PreviewTheme::Dark,
                        ..base
                    },
                ),
            ],
            PreviewStageComparison::Orientations => vec![
                (
                    format!("{} · Portrait", stage.configuration.name),
                    PreviewEnvironment {
                        width: base.width.min(base.height),
                        height: base.width.max(base.height),
                        ..base
                    },
                ),
                (
                    format!("{} · Landscape", stage.configuration.name),
                    PreviewEnvironment {
                        width: base.width.max(base.height),
                        height: base.width.min(base.height),
                        ..base
                    },
                ),
            ],
            PreviewStageComparison::Directions => vec![
                (
                    format!("{} · LTR", stage.configuration.name),
                    PreviewEnvironment {
                        flow_direction: PreviewFlowDirection::LeftToRight,
                        ..base
                    },
                ),
                (
                    format!("{} · RTL", stage.configuration.name),
                    PreviewEnvironment {
                        flow_direction: PreviewFlowDirection::RightToLeft,
                        ..base
                    },
                ),
            ],
        };
        self.compare_preview_stage(active, values).map(|_| ())
    }

    pub fn show_preview_stage_editor(
        self: &Rc<Self>,
        id: Option<PreviewStageId>,
        anchor: &Control,
    ) -> Result<(), String> {
        self.sync_preview_stage_sources()?;
        let initial = if let Some(id) = id {
            self.preview_stage(id)
                .ok_or("The preview stage was removed")?
                .configuration
        } else {
            self.current_preview_stage_configuration("New stage")?
        };
        if let Some(editor) = self.preview_stages.editor.borrow().as_ref() {
            editor.retire();
        }
        if let Some(flyout) = self.preview_stages.flyout.borrow().as_ref() {
            flyout.hide();
        }
        let editor = PreviewStageEditor::new(initial, self.preview_stage_sources());
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout
            .flyout_presenter_classes()
            .add("designer-stage-editor-flyout");
        flyout.set_content(Some(Control::boxed(editor.control())));
        let epoch = self.preview_stage_epoch();
        let weak = Rc::downgrade(self);
        editor.on_apply(move |configuration| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            workspace.sync_preview_stage_sources()?;
            if workspace.preview_stage_epoch() != epoch {
                return Err("The sources or stage board changed; reopen this stage editor".into());
            }
            if let Some(id) = id {
                workspace.update_preview_stage(id, configuration)
            } else {
                workspace.add_preview_stage(configuration).map(|_| ())
            }
        });
        let close = flyout.downgrade();
        editor.on_close(move || {
            if let Some(flyout) = close.upgrade() {
                flyout.hide();
            }
        });
        let weak = Rc::downgrade(&editor);
        flyout.closed(move || {
            if let Some(editor) = weak.upgrade() {
                editor.retire();
            }
        });
        *self.preview_stages.editor.borrow_mut() = Some(editor);
        self.preview_stages.editor_epoch.set(Some(epoch));
        *self.preview_stages.flyout.borrow_mut() = Some(flyout.clone());
        flyout.show_at(anchor);
        Ok(())
    }
}
