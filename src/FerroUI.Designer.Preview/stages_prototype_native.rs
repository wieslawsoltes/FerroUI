//! The IDE and scenario runner mount the same modal native artboard as Studio.
use crate::protocol::Image;
use crate::stages::StageWindow;
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_base::{threading::Dispatcher, Rect, Ref};
use ferroui_controls::Control;
use ferroui_designer::preview_prototype_layout as layout;
use ferroui_designer::{
    capture_preview_stage_png, PreviewPrototypeInput, PreviewPrototypePlayerAction, PreviewPrototypePlayerView,
    PreviewPrototypeSession, PreviewStageBoard, PreviewStageFocus, PreviewStageId,
    PreviewStageKeyboardInput, PreviewStagePointerInput, PreviewStagePointerOrigin,
    PreviewStageStatus,
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub(super) struct NativePlayer {
    _window: StageWindow,
    view: Rc<PreviewPrototypePlayerView>,
    board: PreviewStageBoard,
    stages: BTreeMap<PreviewStageId, PreviewStageId>,
    pending: Rc<Cell<Option<(PreviewPrototypePlayerAction, u64)>>>,
    pub(super) session: PreviewPrototypeSession,
    pub(super) frame: u64,
    pub(super) links: BTreeMap<usize, Rect>,
    pub(super) diagnostics: Vec<String>,
}

impl NativePlayer {
    pub(super) fn start(
        authoring: &PreviewStageBoard,
        session: PreviewPrototypeSession,
        frame: u64,
        reduced_motion: bool,
    ) -> Result<Self, String> {
        let (board, stages) = authoring.prototype_sources(&session)?.realize()?;
        let view = PreviewPrototypePlayerView::new_embedded();
        view.set_reduced_motion(reduced_motion);
        let pending = Rc::new(Cell::new(None));
        let requested = pending.clone();
        view.on_action(move |action, frame, _| {
            requested.set(Some((action, frame)));
            Ok(())
        });
        let (size, layers) = layout::layers(&board, &stages, &session)?;
        view.prepare_layers(size, &layers, session.flow(), &[], false, frame)?;
        let window = StageWindow::new(view.control());
        let mut player = Self {
            _window: window,
            view,
            board,
            stages,
            pending,
            session,
            frame,
            links: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        player.present()?;
        Ok(player)
    }

    pub(super) fn navigate(
        &mut self,
        next: PreviewPrototypeSession,
        frame: u64,
    ) -> Result<(), String> {
        if self.session == next {
            return Ok(());
        }
        layout::layers(&self.board, &self.stages, &next)?;
        self.cancel(PreviewStagePointerOrigin::Editor);
        self.cancel(PreviewStagePointerOrigin::Automation);
        let motion = self.view.capture_motion(next.motion());
        let previous = std::mem::replace(&mut self.session, next);
        let previous_frame = std::mem::replace(&mut self.frame, frame);
        if let Err(error) = self.present() {
            self.session = previous;
            self.frame = previous_frame;
            self.present()?;
            return Err(error);
        }
        match motion {
            Ok(capture) => self.view.play_motion(capture),
            Err(error) => self.diagnostics.push(format!("Transition skipped. {error}")),
        }
        Ok(())
    }

    pub(super) fn validate_link(&mut self, index: usize) -> Result<(), String> {
        self.view.advance_motion();
        if self.view.motion_running() {
            return Err("Wait for the prototype transition to finish".into());
        }
        if !self.links.contains_key(&index) {
            return Err("Choose an available interaction on the active prototype layer".into());
        }
        let (size, layers) = layout::layers(&self.board, &self.stages, &self.session)?;
        let native = self.native_id();
        let current = self.session.current();
        let report = layout::hotspots(
            &mut self.board,
            native,
            current,
            self.session.graph(),
            size,
            layers.last().unwrap().bounds,
        )?;
        if self.links.get(&index) != report.bounds.get(&index) {
            return Err("The native hotspot moved. Wait for the current prototype image.".into());
        }
        Ok(())
    }

    pub(super) fn present(&mut self) -> Result<(), String> {
        for (index, original) in self.session.active_stages().into_iter().enumerate() {
            self.board
                .set_prototype_overlay_backing(self.stages[&original], index > 0)?;
        }
        let title = self.session.flow();
        let can_back = self.session.can_go_back();
        let current = self.session.current();
        let native = self.native_id();
        let mut retried = BTreeSet::new();
        let mut prior = self
            .links
            .iter()
            .filter_map(|(&link, &bounds)| {
                self.session
                    .graph()
                    .links
                    .get(link)
                    .filter(|value| value.source == current)
                    .map(|value| ferroui_designer::PreviewPrototypeHotspot {
                        link,
                        name: value.name.clone(),
                        bounds,
                    })
            })
            .collect::<Vec<_>>();
        prior.sort_by_key(|hotspot| {
            (
                self.session.graph().links[hotspot.link].trigger.path.len(),
                hotspot.link,
            )
        });
        for _ in 0..3 {
            let (size, layers) = layout::layers(&self.board, &self.stages, &self.session)?;
            let signature = layout::signature(&layers);
            self.view
                .prepare_layers(size, &layers, title, &prior, can_back, self.frame)?;
            self.layout();
            for original in self.session.active_stages() {
                let id = self.stages[&original];
                let stage = self
                    .board
                    .stage(id)
                    .ok_or("The native prototype stage was removed")?;
                if matches!(stage.status, PreviewStageStatus::Failed { .. }) && retried.insert(id) {
                    self.board.refresh(id)?;
                } else if !matches!(stage.status, PreviewStageStatus::Ready { .. }) {
                    return Err(format!(
                        "The native stage '{}' is not ready: {:?}",
                        stage.configuration.name, stage.status
                    ));
                }
            }
            self.layout();
            let (_, measured) = layout::layers(&self.board, &self.stages, &self.session)?;
            if signature != layout::signature(&measured) {
                prior.clear();
                continue;
            }
            let top = measured.last().unwrap().bounds;
            let report = layout::hotspots(
                &mut self.board,
                native,
                current,
                self.session.graph(),
                size,
                top,
            )?;
            self.view.set_hotspot_focus_targets(&report.focus_targets);
            self.view.update_layers(
                size,
                &measured,
                title,
                &report.hotspots,
                can_back,
                self.frame,
            )?;
            self.view.focus_active_plane();
            self.layout();
            let (_, accepted) = layout::layers(&self.board, &self.stages, &self.session)?;
            let final_report = layout::hotspots(
                &mut self.board,
                native,
                current,
                self.session.graph(),
                size,
                top,
            )?;
            if signature == layout::signature(&accepted)
                && report.bounds == final_report.bounds
                && report.focus_targets == final_report.focus_targets
            {
                self.links = final_report.bounds;
                self.diagnostics = final_report.diagnostics;
                return Ok(());
            }
            prior = final_report.hotspots;
        }
        Err("The native prototype layout did not settle. Inspect its content sizing.".into())
    }

    fn layout(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.view.control().update_layout();
    }

    fn native_id(&self) -> PreviewStageId {
        self.stages[&self.session.current()]
    }

    pub(super) fn keyboard(
        &self,
        input: PreviewStageKeyboardInput,
    ) -> Result<Option<PreviewStageFocus>, String> {
        self.view.focus_active_plane();
        self.view
            .keyboard_target(&self.board, self.native_id())?
            .input(input)
    }

    pub(super) fn focus(&self) -> Result<Option<PreviewStageFocus>, String> {
        Ok(self
            .view
            .keyboard_target(&self.board, self.native_id())?
            .current_focus())
    }

    pub(super) fn pointer(
        &self,
        input: PreviewStagePointerInput,
        origin: PreviewStagePointerOrigin,
    ) -> Result<(), String> {
        self.view
            .pointer_target(&self.board, self.native_id(), origin)?
            .input(input)
    }

    pub(super) fn cancel(&mut self, origin: PreviewStagePointerOrigin) {
        self.board.cancel_pointer_input(origin);
        Dispatcher::ui_thread().run_jobs(None);
    }

    pub(super) fn take_action(&self) -> Option<PreviewPrototypePlayerAction> {
        self.pending
            .take()
            .filter(|(_, frame)| *frame == self.frame)
            .map(|(action, _)| action)
    }

    pub(super) fn image(&self) -> Result<Image, String> {
        self.view.advance_motion();
        self.layout();
        capture(&self.view.artboard())
    }

    pub(super) fn animating(&self) -> bool {
        self.view.motion_running()
    }

    pub(super) fn advance_motion(&self) {
        self.view.advance_motion();
    }

    pub(super) fn set_reduced_motion(&self, reduced: bool) {
        self.view.set_reduced_motion(reduced);
    }

    pub(super) fn check_input(&self, input: &PreviewPrototypeInput) -> Result<(), String> {
        self.view.check_motion_input(input)
    }

    pub(super) fn active_control(&self) -> Result<Ref<Control>, String> {
        self.board
            .control(self.native_id())
            .ok_or_else(|| "The native stage was removed".into())
    }

    pub(super) fn active_image(&mut self) -> Result<Image, String> {
        let id = self.native_id();
        let overlay = !self.session.overlays().is_empty();
        self.board.set_prototype_overlay_backing(id, false)?;
        let image = self.active_control().and_then(|control| capture(&control));
        self.board.set_prototype_overlay_backing(id, overlay)?;
        image
    }
}

impl Drop for NativePlayer {
    fn drop(&mut self) {
        self.view.clear();
    }
}

fn capture(control: &Ref<Control>) -> Result<Image, String> {
    let image = capture_preview_stage_png(control, Default::default())?;
    Ok(Image {
        png: STANDARD.encode(image.png),
        width: image.pixel_width,
        height: image.pixel_height,
    })
}
