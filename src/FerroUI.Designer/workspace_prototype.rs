//! Prototype definitions belong to the preview board. The editor and player have
//! independent presentation state and never activate an authored document tab.
use super::*;
use crate::{
    PreviewPrototype, PreviewPrototypeElementChoice, PreviewPrototypeFlowEditor,
    PreviewPrototypeLinkEditor, PreviewPrototypeOverlay, PreviewPrototypePlayerView,
    PreviewPrototypeSession, PreviewPrototypeView, PreviewPrototypeViewAction, PreviewSourceAnchor,
    PreviewStageBoard, PreviewStageId,
};
use ferroui_base::Rect;
use ferroui_controls::Flyout;
use serde::Serialize;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[path = "workspace_prototype_layout.rs"]
mod layout;
#[path = "workspace_prototype_player.rs"]
mod player;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PreviewPrototypePlaybackInfo {
    pub flow: String,
    /// The screen under the modal overlay stack. History contains screens only.
    pub screen: PreviewStageId,
    pub current: PreviewStageId,
    pub overlays: Vec<PreviewPrototypeOverlay>,
    pub history: Vec<PreviewStageId>,
    pub epoch: u64,
    pub current_source: bool,
    pub diagnostics: Vec<String>,
}

pub(super) struct RunningPrototype {
    session: PreviewPrototypeSession,
    board: PreviewStageBoard,
    stages: BTreeMap<PreviewStageId, PreviewStageId>,
    source_model: String,
    bounds: BTreeMap<usize, Rect>,
    layout: Vec<(PreviewStageId, Rect)>,
    diagnostics: Vec<String>,
}

enum PrototypeEditor {
    Flow(Rc<PreviewPrototypeFlowEditor>),
    Link(Rc<PreviewPrototypeLinkEditor>),
}
impl PrototypeEditor {
    fn retire(&self, message: &str) {
        match self {
            Self::Flow(editor) => {
                editor.retire();
                editor.show_error(message);
            }
            Self::Link(editor) => {
                editor.retire();
                editor.show_error(message);
            }
        }
    }
}

pub(super) struct PrototypeUi {
    visible: Cell<bool>,
    refreshing: Cell<bool>,
    generation: Cell<u64>,
    view: RefCell<Option<Rc<PreviewPrototypeView>>>,
    view_model: RefCell<Option<String>>,
    player_view: RefCell<Option<Rc<PreviewPrototypePlayerView>>>,
    player: RefCell<Option<RunningPrototype>>,
    editor: RefCell<Option<PrototypeEditor>>,
    editor_model: RefCell<Option<String>>,
    flyout: RefCell<Option<Ref<Flyout>>>,
}
impl Default for PrototypeUi {
    fn default() -> Self {
        Self {
            visible: Cell::new(false),
            refreshing: Cell::new(false),
            generation: Cell::new(0),
            view: RefCell::new(None),
            view_model: RefCell::new(None),
            player_view: RefCell::new(None),
            player: RefCell::new(None),
            editor: RefCell::new(None),
            editor_model: RefCell::new(None),
            flyout: RefCell::new(None),
        }
    }
}

impl DesignerWorkspace {
    pub fn preview_prototype(&self) -> PreviewPrototype {
        self.preview_stages.board.borrow().prototype()
    }

    /// Includes the accepted source/board token and the independent playback epoch.
    /// Reading this token never creates controls, updates layout or advances input.
    pub fn preview_prototype_model(&self) -> String {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        self.preview_stage_model().hash(&mut hash);
        self.prototype.generation.get().hash(&mut hash);
        self.prototype.visible.get().hash(&mut hash);
        format!("{:016x}", hash.finish())
    }

    pub fn preview_prototype_view(&self) -> Option<Rc<PreviewPrototypeView>> {
        self.prototype.view.borrow().clone()
    }
    pub fn preview_prototype_player_view(&self) -> Option<Rc<PreviewPrototypePlayerView>> {
        self.prototype.player_view.borrow().clone()
    }
    pub fn preview_prototype_flow_editor(&self) -> Option<Rc<PreviewPrototypeFlowEditor>> {
        match self.prototype.editor.borrow().as_ref() {
            Some(PrototypeEditor::Flow(editor)) => Some(editor.clone()),
            _ => None,
        }
    }
    pub fn preview_prototype_link_editor(&self) -> Option<Rc<PreviewPrototypeLinkEditor>> {
        match self.prototype.editor.borrow().as_ref() {
            Some(PrototypeEditor::Link(editor)) => Some(editor.clone()),
            _ => None,
        }
    }
    pub fn preview_prototype_visible(&self) -> bool {
        self.prototype.visible.get()
    }

    /// Validate a detached graph against the currently accepted authored buffers.
    /// This operation is observational: it does not run factories or synchronize a
    /// pending draft. Runtime geometry/instance uniqueness is checked by playback.
    fn validate_preview_prototype(&self, graph: &PreviewPrototype) -> Result<(), String> {
        let stages = self.preview_stages();
        graph.validate(&stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        let sources = self.preview_stage_sources();
        for link in &graph.links {
            let stage = stages.iter().find(|stage| stage.id == link.source).unwrap();
            let source = sources
                .iter()
                .find(|source| source.uri == stage.configuration.source_uri)
                .ok_or("Open the interaction's source document before editing its flow")?;
            if source.pending {
                return Err(
                    "Apply or repair pending XAML before editing prototype interactions".into(),
                );
            }
            link.trigger.resolve(&source.document)?;
        }
        Ok(())
    }

    pub fn set_preview_prototype(self: &Rc<Self>, graph: PreviewPrototype) -> Result<(), String> {
        self.validate_preview_prototype(&graph)?;
        if graph == self.preview_prototype() {
            return Ok(());
        }
        self.preview_stages
            .board
            .borrow_mut()
            .set_prototype(graph)?;
        self.retire_preview_prototype_player();
        self.refresh_preview_prototype();
        Ok(())
    }

    /// Authored, unambiguous source choices from accepted native stage trees.
    /// The returned anchors contain no runtime inspector or session-local node IDs.
    pub fn preview_prototype_elements(&self) -> Vec<PreviewPrototypeElementChoice> {
        let sources = self.preview_stage_sources();
        let mut choices = Vec::new();
        for stage in self.preview_stages() {
            if !self.preview_stage_source_is_current(stage.id) {
                continue;
            }
            let Some(source) = sources
                .iter()
                .find(|source| source.uri == stage.configuration.source_uri)
            else {
                continue;
            };
            let Ok(rows) = self.preview_stage_runtime_tree(stage.id, TreeKind::Visual) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            for id in rows.into_iter().filter_map(|row| row.source_id) {
                if !seen.insert(id) {
                    continue;
                }
                let Some(node) = source.document.ast().node(id) else {
                    continue;
                };
                let Ok(anchor) = PreviewSourceAnchor::capture(&source.document, id) else {
                    continue;
                };
                let label = match node.xaml_name() {
                    Some(name) => format!("{} · {name}", node.local_name()),
                    None => format!(
                        "{} · source line {}",
                        node.local_name(),
                        source.document.source()
                            [..node.span.start.min(source.document.source().len())]
                            .bytes()
                            .filter(|byte| *byte == b'\n')
                            .count()
                            + 1
                    ),
                };
                choices.push(PreviewPrototypeElementChoice {
                    stage: stage.id,
                    label,
                    anchor,
                });
                if choices.len() >= 4096 {
                    return choices;
                }
            }
        }
        choices
    }

    pub fn set_preview_prototype_visible(self: &Rc<Self>, visible: bool) -> Result<(), String> {
        if !visible {
            self.close_preview_prototype();
            self.refresh_preview_stages();
            return Ok(());
        }
        self.set_preview_stages_visible(true)?;
        self.ensure_preview_prototype_view();
        self.prototype.visible.set(true);
        self.refresh_preview_prototype();
        Ok(())
    }

    pub(super) fn close_preview_prototype(&self) {
        self.retire_preview_prototype_player();
        self.prototype.visible.set(false);
        if let Some(flyout) = self.prototype.flyout.borrow_mut().take() {
            flyout.hide();
        }
        if let Some(editor) = self.prototype.editor.borrow_mut().take() {
            editor.retire("The prototype editor was closed");
        }
        self.prototype.editor_model.borrow_mut().take();
        if let Some(view) = self.preview_stage_view() {
            self.mount_prototype_control(view.control());
        }
    }

    fn bump_prototype_generation(&self) {
        self.prototype.generation.set(
            self.prototype
                .generation
                .get()
                .checked_add(1)
                .expect("Prototype epoch exhausted"),
        );
    }

    fn mount_prototype_control(&self, control: Ref<Control>) {
        let host = self.root.get_control::<ContentControl>("PreviewStagesHost");
        let current = host
            .content()
            .and_then(|value| Ref::<Control>::from_xaml_object(&value));
        if current.as_ref() != Some(&control) {
            host.set_content(Some(Control::boxed(control)));
        }
    }

    fn ensure_preview_prototype_view(self: &Rc<Self>) {
        if self.prototype.view.borrow().is_some() {
            return;
        }
        let view = PreviewPrototypeView::new(&self.root);
        let weak = Rc::downgrade(self);
        view.on_action(move |action, epoch, anchor| {
            weak.upgrade()
                .ok_or("The workspace was closed")?
                .handle_prototype_action(action, epoch, &anchor)
        });
        *self.prototype.view.borrow_mut() = Some(view);
    }

    pub(super) fn refresh_preview_prototype(self: &Rc<Self>) {
        if self.prototype.refreshing.replace(true) {
            return;
        }
        let model = self.preview_stage_model();
        if self
            .prototype
            .editor_model
            .borrow()
            .as_ref()
            .is_some_and(|accepted| accepted != &model)
        {
            if let Some(editor) = self.prototype.editor.borrow().as_ref() {
                editor.retire("The sources or preview stages changed. Reopen this prototype editor to use current values.");
            }
        }
        let stale = self
            .prototype
            .player
            .borrow()
            .as_ref()
            .is_some_and(|player| player.source_model != model);
        if stale || self.view_mode() == super::shell::WorkspaceViewMode::Xaml {
            self.retire_preview_prototype_player();
        }
        if self.prototype.visible.get() {
            self.ensure_preview_prototype_view();
            if let Some(view) = self.preview_prototype_view() {
                if self.prototype.view_model.borrow().as_ref() != Some(&model) {
                    view.update(
                        &self.preview_prototype(),
                        &self.preview_stages(),
                        self.preview_stage_epoch(),
                    );
                    *self.prototype.view_model.borrow_mut() = Some(model);
                }
                if self.prototype.player.borrow().is_some() {
                    if let Err(error) = self.refresh_preview_prototype_player() {
                        self.retire_preview_prototype_player();
                        self.mount_prototype_control(view.control());
                        view.show_error(&error);
                    }
                } else {
                    self.mount_prototype_control(view.control());
                    if stale {
                        view.show_error("The sources or stage definitions changed. Play the flow again to create current native views.");
                    }
                }
            }
        }
        self.prototype.refreshing.set(false);
    }

    fn handle_prototype_action(
        self: &Rc<Self>,
        action: PreviewPrototypeViewAction,
        epoch: u64,
        anchor: &Control,
    ) -> Result<(), String> {
        if epoch != self.preview_stage_epoch() {
            return Err("The prototype graph changed; use the current overview".into());
        }
        let mut graph = self.preview_prototype();
        match action {
            PreviewPrototypeViewAction::Back => self.set_preview_prototype_visible(false),
            PreviewPrototypeViewAction::NewFlow => {
                self.show_preview_prototype_flow_editor(None, anchor)
            }
            PreviewPrototypeViewAction::EditFlow { index } => {
                self.show_preview_prototype_flow_editor(Some(index), anchor)
            }
            PreviewPrototypeViewAction::RemoveFlow { index } => {
                if index >= graph.flows.len() {
                    return Err("The flow was removed".into());
                }
                graph.flows.remove(index);
                self.set_preview_prototype(graph)
            }
            PreviewPrototypeViewAction::Play { flow } => {
                let flow = graph.flows.get(flow).ok_or("The flow was removed")?;
                self.start_preview_prototype(&flow.name)
            }
            PreviewPrototypeViewAction::AddLink { source } => {
                self.show_preview_prototype_link_editor(None, source, anchor)
            }
            PreviewPrototypeViewAction::EditLink { index } => {
                self.show_preview_prototype_link_editor(Some(index), None, anchor)
            }
            PreviewPrototypeViewAction::RemoveLink { index } => {
                if index >= graph.links.len() {
                    return Err("The interaction was removed".into());
                }
                graph.links.remove(index);
                self.set_preview_prototype(graph)
            }
            PreviewPrototypeViewAction::FocusStage { stage } => {
                self.set_preview_prototype_visible(false)?;
                self.set_active_preview_stage(stage)
            }
        }
    }

    fn close_prototype_editor(&self) {
        if let Some(flyout) = self.prototype.flyout.borrow_mut().take() {
            flyout.hide();
        }
        if let Some(editor) = self.prototype.editor.borrow_mut().take() {
            editor.retire("The editor was closed");
        }
        self.prototype.editor_model.borrow_mut().take();
    }

    pub fn show_preview_prototype_flow_editor(
        self: &Rc<Self>,
        index: Option<usize>,
        anchor: &Control,
    ) -> Result<(), String> {
        let graph = self.preview_prototype();
        let initial = index
            .map(|index| {
                graph
                    .flows
                    .get(index)
                    .cloned()
                    .ok_or("The flow was removed")
            })
            .transpose()?;
        let stages = self.preview_stages();
        if stages.is_empty() {
            return Err("Add a preview stage before creating a flow".into());
        }
        self.close_prototype_editor();
        let editor = PreviewPrototypeFlowEditor::new(initial, stages);
        let model = self.preview_stage_model();
        let weak = Rc::downgrade(self);
        let expected = model.clone();
        editor.on_apply(move |flow| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            if workspace.preview_stage_model() != expected {
                return Err("The sources or prototype changed; reopen the flow editor".into());
            }
            let mut next = workspace.preview_prototype();
            match index {
                Some(index) => next.flows[index] = flow,
                None => next.flows.push(flow),
            }
            workspace.set_preview_prototype(next)
        });
        let weak = Rc::downgrade(self);
        editor.on_close(move || {
            if let Some(workspace) = weak.upgrade() {
                workspace.close_prototype_editor();
            }
        });
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout
            .flyout_presenter_classes()
            .add("designer-stage-editor-flyout");
        flyout.set_content(Some(Control::boxed(editor.control())));
        let weak_editor = Rc::downgrade(&editor);
        flyout.closed(move || {
            if let Some(editor) = weak_editor.upgrade() {
                editor.retire();
            }
        });
        *self.prototype.editor.borrow_mut() = Some(PrototypeEditor::Flow(editor));
        *self.prototype.editor_model.borrow_mut() = Some(model);
        *self.prototype.flyout.borrow_mut() = Some(flyout.clone());
        flyout.show_at(anchor);
        Ok(())
    }

    pub fn show_preview_prototype_link_editor(
        self: &Rc<Self>,
        index: Option<usize>,
        source: Option<PreviewStageId>,
        anchor: &Control,
    ) -> Result<(), String> {
        let graph = self.preview_prototype();
        let initial = index
            .map(|index| {
                graph
                    .links
                    .get(index)
                    .cloned()
                    .ok_or("The interaction was removed")
            })
            .transpose()?;
        if let Some(source) = source {
            self.preview_stage(source)
                .ok_or("The source stage was removed")?;
        }
        let stages = self.preview_stages();
        if stages.is_empty() {
            return Err("Add a preview stage before creating an interaction".into());
        }
        let elements = self.preview_prototype_elements();
        if elements.is_empty() {
            return Err(
                "Refresh a valid stage with named authored controls before adding an interaction"
                    .into(),
            );
        }
        self.close_prototype_editor();
        let editor = PreviewPrototypeLinkEditor::new(initial, source, stages, elements);
        let model = self.preview_stage_model();
        let weak = Rc::downgrade(self);
        let expected = model.clone();
        editor.on_apply(move |link| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            if workspace.preview_stage_model() != expected {
                return Err(
                    "The sources or prototype changed; reopen the interaction editor".into(),
                );
            }
            let mut next = workspace.preview_prototype();
            match index {
                Some(index) => next.links[index] = link,
                None => next.links.push(link),
            }
            workspace.set_preview_prototype(next)
        });
        let weak = Rc::downgrade(self);
        editor.on_close(move || {
            if let Some(workspace) = weak.upgrade() {
                workspace.close_prototype_editor();
            }
        });
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout
            .flyout_presenter_classes()
            .add("designer-stage-editor-flyout");
        flyout.set_content(Some(Control::boxed(editor.control())));
        let weak_editor = Rc::downgrade(&editor);
        flyout.closed(move || {
            if let Some(editor) = weak_editor.upgrade() {
                editor.retire();
            }
        });
        *self.prototype.editor.borrow_mut() = Some(PrototypeEditor::Link(editor));
        *self.prototype.editor_model.borrow_mut() = Some(model);
        *self.prototype.flyout.borrow_mut() = Some(flyout.clone());
        flyout.show_at(anchor);
        Ok(())
    }
}

#[cfg(test)]
#[path = "workspace_prototype_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "workspace_prototype_overlay_tests.rs"]
mod overlay_tests;
