//! A play session realizes a bounded set of isolated native stages. Screen history
//! stores original board identities; native controls are retained until Exit/Play.
use super::*;
use crate::{
    PreviewPrototypeHotspot, PreviewPrototypePlayerAction, PreviewStageMode, PreviewStageSize,
    PreviewStageStatus,
};
use ferroui_base::input::InputElement;
use std::collections::BTreeSet;

struct PrototypeHotspotFrame {
    hotspots: Vec<PreviewPrototypeHotspot>,
    focus_targets: Vec<(usize, Ref<InputElement>)>,
    bounds: BTreeMap<usize, Rect>,
    diagnostics: Vec<String>,
}

fn inspect_prototype_hotspots(
    player: &mut RunningPrototype,
    graph: &PreviewPrototype,
    size: PreviewStageSize,
    top: Rect,
) -> Result<PrototypeHotspotFrame, String> {
    let id = player.stages[&player.session.current()];
    // Only the top layer is interactive. Covered views retain their native
    // graphs without rebuilding every hidden source map on each tick.
    player.board.refresh_runtime_mapping(id)?;
    let mut hotspots = Vec::new();
    let mut focus_targets = Vec::new();
    let mut diagnostics = Vec::new();
    let mut indexed = graph
        .links
        .iter()
        .enumerate()
        .filter(|(_, link)| link.source == player.session.current())
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(index, link)| (link.trigger.path.len(), *index));
    for (index, link) in indexed {
        match layout::hotspot(&player.board, id, &link.trigger, size, top) {
            Ok(bounds) => {
                if let Ok(visual) = player.board.prototype_hotspot_visual(id, &link.trigger) {
                    if let Some(input) = visual.cast::<InputElement>() {
                        focus_targets.push((index, input));
                    }
                }
                hotspots.push(PreviewPrototypeHotspot {
                    link: index,
                    name: link.name.clone(),
                    bounds,
                });
            }
            Err(error) => diagnostics.push(format!("{}: {error}", link.name)),
        }
    }
    let bounds = hotspots
        .iter()
        .map(|hotspot| (hotspot.link, hotspot.bounds))
        .collect();
    Ok(PrototypeHotspotFrame {
        hotspots,
        focus_targets,
        bounds,
        diagnostics,
    })
}

impl DesignerWorkspace {
    pub fn preview_prototype_playback(&self) -> Option<PreviewPrototypePlaybackInfo> {
        self.prototype
            .player
            .borrow()
            .as_ref()
            .map(|player| PreviewPrototypePlaybackInfo {
                flow: player.session.flow().into(),
                screen: player.session.screen(),
                current: player.session.current(),
                overlays: player.session.overlays().to_vec(),
                history: player.session.history().to_vec(),
                epoch: self.prototype.generation.get(),
                current_source: player.source_model == self.preview_stage_model(),
                diagnostics: player.diagnostics.clone(),
            })
    }

    /// A native view in the isolated player, addressed by its original board ID.
    /// It is never the comparison card's native control.
    pub fn preview_prototype_control(&self, stage: PreviewStageId) -> Option<Ref<Control>> {
        let player = self.prototype.player.borrow();
        let player = player.as_ref()?;
        player.board.control(*player.stages.get(&stage)?)
    }

    pub fn start_preview_prototype(self: &Rc<Self>, flow_name: &str) -> Result<(), String> {
        let graph = self.preview_prototype();
        PreviewPrototypeSession::start(&graph, flow_name)?;
        self.set_preview_prototype_visible(true)?;
        let source_model = self.preview_stage_model();
        let stages = self.preview_stages();
        graph.validate(&stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        let session = PreviewPrototypeSession::start(&graph, flow_name)?;
        let reachable = graph.reachable(session.current());
        let sources = self.preview_stage_sources();
        let mut configurations = Vec::new();
        let mut originals = Vec::new();
        for info in stages.iter().filter(|info| reachable.contains(&info.id)) {
            let source = sources
                .iter()
                .find(|source| source.uri == info.configuration.source_uri)
                .ok_or("Open every screen source in the selected flow before playing it")?;
            if source.pending || !source.document.is_valid() {
                return Err("Apply or repair the flow's pending XAML before playing it".into());
            }
            for link in graph.links.iter().filter(|link| link.source == info.id) {
                link.trigger.resolve(&source.document)?;
            }
            let mut configuration = info.configuration.clone();
            configuration.mode = PreviewStageMode::Live;
            configuration.auto_update = true;
            configuration.pinned = true;
            originals.push(info.id);
            configurations.push(configuration);
        }
        let (options, options_epoch) = self.preview_stage_host_options();
        let mut board = PreviewStageBoard::new();
        board.sync_sources(
            self.session.borrow().document_uri(),
            sources,
            options,
            options_epoch,
        )?;
        let ids = board.add_configurations(configurations)?;
        let mapping = originals.into_iter().zip(ids).collect::<BTreeMap<_, _>>();

        if source_model != self.preview_stage_model() {
            return Err("The sources or stage definitions changed while creating the prototype; play it again".into());
        }
        self.retire_preview_prototype_player();
        self.ensure_preview_prototype_player_view();
        *self.prototype.player.borrow_mut() = Some(RunningPrototype {
            session,
            board,
            stages: mapping,
            source_model,
            bounds: BTreeMap::new(),
            layout: Vec::new(),
            diagnostics: Vec::new(),
        });
        self.bump_prototype_generation();
        self.refresh_preview_prototype();
        if self.prototype.player.borrow().is_none() {
            return Err("The prototype's native start screen could not be presented; inspect the overview diagnostic".into());
        }
        Ok(())
    }

    pub fn activate_preview_prototype_link(self: &Rc<Self>, index: usize) -> Result<(), String> {
        self.ensure_current_prototype_player()?;
        let graph = self.preview_prototype();
        let link = graph
            .links
            .get(index)
            .ok_or("The prototype interaction was removed")?;
        let next = {
            let player = self.prototype.player.borrow();
            let player = player
                .as_ref()
                .ok_or("Play a prototype flow before activating an interaction")?;
            if link.source != player.session.current() {
                return Err("This interaction belongs to a different prototype screen".into());
            }
            let native = *player
                .stages
                .get(&link.source)
                .ok_or("The prototype screen was removed")?;
            let (size, layers) = layout::layers(player, &player.session)?;
            if player.layout != layout::signature(&layers) {
                return Err(
                    "The presented screens moved; wait for the current native layout".into(),
                );
            }
            let bounds = layout::hotspot(
                &player.board,
                native,
                &link.trigger,
                size,
                layers.last().unwrap().bounds,
            )?;
            if player.bounds.get(&index) != Some(&bounds) {
                return Err(
                    "The hotspot moved or became unavailable; wait for the current native layout"
                        .into(),
                );
            }
            let mut next = player.session.clone();
            next.activate_link(index)?;
            next
        };
        self.accept_prototype_navigation(next)
    }

    pub fn back_preview_prototype(self: &Rc<Self>) -> Result<(), String> {
        self.ensure_current_prototype_player()?;
        let mut next = self
            .prototype
            .player
            .borrow()
            .as_ref()
            .unwrap()
            .session
            .clone();
        next.back()?;
        self.accept_prototype_navigation(next)
    }

    /// Close the top overlay without consuming screen navigation history.
    pub fn close_preview_prototype_overlay(self: &Rc<Self>) -> Result<(), String> {
        self.close_prototype_overlay(false)
    }

    fn close_prototype_overlay(self: &Rc<Self>, outside: bool) -> Result<(), String> {
        self.ensure_current_prototype_player()?;
        let mut next = {
            let player = self.prototype.player.borrow();
            let player = player.as_ref().ok_or("The prototype was closed")?;
            let overlay = player
                .session
                .overlays()
                .last()
                .ok_or("The prototype has no open overlay")?;
            if outside && !overlay.options.dismiss_on_outside {
                return Err("This overlay requires an explicit close action".into());
            }
            player.session.clone()
        };
        next.close_overlay()?;
        self.accept_prototype_navigation(next)
    }

    /// Restart the journey while retaining the play session's native control state.
    /// Exit and Play create fresh native views when a full reset is wanted.
    pub fn restart_preview_prototype(self: &Rc<Self>) -> Result<(), String> {
        self.ensure_current_prototype_player()?;
        let mut next = self
            .prototype
            .player
            .borrow()
            .as_ref()
            .unwrap()
            .session
            .clone();
        next.restart();
        self.accept_prototype_navigation(next)
    }

    pub fn stop_preview_prototype(self: &Rc<Self>) {
        self.retire_preview_prototype_player();
        self.refresh_preview_prototype();
    }

    fn accept_prototype_navigation(
        self: &Rc<Self>,
        next: PreviewPrototypeSession,
    ) -> Result<(), String> {
        let (previous, retained_base) = {
            let player = self.prototype.player.borrow();
            let player = player.as_ref().ok_or("The prototype was closed")?;
            if player.session == next {
                return Ok(());
            }
            // Reject unusable manual positions or unavailable layer identities
            // before changing accepted history, overlays or the current screen.
            layout::layers(player, &next)?;
            (
                player.session.clone(),
                player
                    .board
                    .control(player.stages[&player.session.screen()]),
            )
        };
        if self.prototype.refreshing.replace(true) {
            return Err("Wait for the current native prototype layout before navigating".into());
        }
        {
            let mut player = self.prototype.player.borrow_mut();
            let player = player.as_mut().unwrap();
            player.session = next;
            player.bounds.clear();
            player.layout.clear();
            player.diagnostics.clear();
        }
        self.bump_prototype_generation();
        let result = self.refresh_preview_prototype_player();
        if result.is_err() {
            // A fitted destination can resolve a different size only after it
            // inherits the real native root. Recover the prior retained stack
            // when that final geometry cannot be presented. Frame tokens still
            // advance because provisional native controls have been retired.
            let same_player = self
                .prototype
                .player
                .borrow()
                .as_ref()
                .is_some_and(|player| {
                    player
                        .stages
                        .get(&previous.screen())
                        .and_then(|id| player.board.control(*id))
                        == retained_base
                });
            if same_player && self.ensure_current_prototype_player().is_ok() {
                {
                    let mut player = self.prototype.player.borrow_mut();
                    let player = player.as_mut().unwrap();
                    player.session = previous;
                    player.bounds.clear();
                    player.layout.clear();
                    player.diagnostics.clear();
                }
                self.bump_prototype_generation();
                if self.refresh_preview_prototype_player().is_err() {
                    self.retire_preview_prototype_player();
                }
            } else if same_player {
                self.retire_preview_prototype_player();
            }
        }
        self.prototype.refreshing.set(false);
        if let Err(error) = &result {
            if self.prototype.player.borrow().is_none() {
                self.refresh_preview_prototype();
                if let Some(view) = self.preview_prototype_view() {
                    view.show_error(error);
                }
            } else if let Some(view) = self.preview_prototype_player_view() {
                view.show_error(error);
            }
        }
        result
    }

    fn ensure_current_prototype_player(&self) -> Result<(), String> {
        if !self.preview_stages_visible()
            || !self.prototype.visible.get()
            || self.view_mode() == super::super::shell::WorkspaceViewMode::Xaml
        {
            return Err("The prototype player is not visible".into());
        }
        let player = self.prototype.player.borrow();
        let player = player
            .as_ref()
            .ok_or("Play a prototype flow before navigating")?;
        if player.source_model != self.preview_stage_model() {
            return Err("The sources or stage definitions changed; play the flow again".into());
        }
        Ok(())
    }

    pub(super) fn retire_preview_prototype_player(&self) {
        if let Some(view) = self.preview_prototype_player_view() {
            view.clear();
        }
        if self.prototype.player.borrow_mut().take().is_some() {
            self.bump_prototype_generation();
        }
    }

    fn ensure_preview_prototype_player_view(self: &Rc<Self>) {
        if self.prototype.player_view.borrow().is_some() {
            return;
        }
        let view = PreviewPrototypePlayerView::new(&self.root);
        let weak = Rc::downgrade(self);
        view.on_action(move |action, epoch, _| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            if epoch != workspace.prototype.generation.get() {
                return Err("This prototype frame was replaced; use the current controls".into());
            }
            match action {
                PreviewPrototypePlayerAction::Activate { link } => {
                    workspace.activate_preview_prototype_link(link)
                }
                PreviewPrototypePlayerAction::Back => workspace.back_preview_prototype(),
                PreviewPrototypePlayerAction::CloseOverlay => {
                    workspace.close_preview_prototype_overlay()
                }
                PreviewPrototypePlayerAction::DismissOverlay => {
                    workspace.close_prototype_overlay(true)
                }
                PreviewPrototypePlayerAction::Restart => workspace.restart_preview_prototype(),
                PreviewPrototypePlayerAction::Exit => {
                    workspace.stop_preview_prototype();
                    Ok(())
                }
            }
        });
        *self.prototype.player_view.borrow_mut() = Some(view);
    }

    pub(super) fn refresh_preview_prototype_player(self: &Rc<Self>) -> Result<(), String> {
        // Restoring native focus can apply styles or invoke application callbacks
        // that change a fitted view's desired size. Settle that final layout too,
        // before accepting the journey or exposing its new interaction geometry.
        for _ in 0..3 {
            if self.refresh_preview_prototype_player_frame()? {
                return Ok(());
            }
        }
        Err("The native overlay layout did not settle after restoring focus; inspect its content sizing".into())
    }

    fn refresh_preview_prototype_player_frame(self: &Rc<Self>) -> Result<bool, String> {
        self.ensure_current_prototype_player()?;
        {
            let mut player = self.prototype.player.borrow_mut();
            let player = player.as_mut().ok_or("The prototype was closed")?;
            for (index, original) in player.session.active_stages().into_iter().enumerate() {
                let native = player.stages[&original];
                player
                    .board
                    .set_prototype_overlay_backing(native, index > 0)?;
            }
        }
        let view = self
            .preview_prototype_player_view()
            .ok_or("The native prototype player was closed")?;
        self.mount_prototype_control(view.control());
        let mut retried = BTreeSet::new();
        let mut presented = None;
        // A native fitted view can resolve a different size after attachment.
        // Settle the bounded visible stack, keeping every covered host mounted.
        for _ in 0..3 {
            let (size, layers, title, can_back, prior_hotspots, failed) = {
                let player = self.prototype.player.borrow();
                let player = player.as_ref().ok_or("The prototype was closed")?;
                let (size, layers) = layout::layers(player, &player.session)?;
                let active = *player.stages.get(&player.session.current()).unwrap();
                let info = player
                    .board
                    .stage(active)
                    .ok_or("The active screen was removed")?;
                let title = if player.session.overlays().is_empty() {
                    format!("{} · {}", player.session.flow(), info.configuration.name)
                } else {
                    format!(
                        "{} · {} · Overlay {}",
                        player.session.flow(),
                        info.configuration.name,
                        player.session.overlays().len()
                    )
                };
                let graph = self.preview_prototype();
                let mut prior_hotspots = Vec::new();
                if player.layout == layout::signature(&layers) {
                    prior_hotspots = player
                        .bounds
                        .iter()
                        .filter_map(|(index, bounds)| {
                            graph.links.get(*index).map(|link| PreviewPrototypeHotspot {
                                link: *index,
                                name: link.name.clone(),
                                bounds: *bounds,
                            })
                        })
                        .collect();
                    prior_hotspots.sort_by_key(|hotspot| {
                        (graph.links[hotspot.link].trigger.path.len(), hotspot.link)
                    });
                }
                let failed = player
                    .session
                    .active_stages()
                    .into_iter()
                    .map(|id| player.stages[&id])
                    .filter(|id| {
                        player.board.stage(*id).is_some_and(|stage| {
                            matches!(stage.status, PreviewStageStatus::Failed { .. })
                        })
                    })
                    .collect::<Vec<_>>();
                (
                    size,
                    layers,
                    title,
                    player.session.can_go_back(),
                    prior_hotspots,
                    failed,
                )
            };
            view.prepare_layers(
                size,
                &layers,
                &title,
                &prior_hotspots,
                can_back,
                self.prototype.generation.get(),
            )?;
            view.control().update_layout();
            // Definition factories and authored scenarios may require inherited
            // native resources from the actual player root before they can load.
            for id in failed {
                if retried.insert(id) {
                    self.prototype
                        .player
                        .borrow_mut()
                        .as_mut()
                        .ok_or("The prototype was closed during layout")?
                        .board
                        .refresh(id)?;
                }
            }
            view.control().update_layout();
            self.ensure_current_prototype_player()?;
            let current = {
                let player = self.prototype.player.borrow();
                let player = player.as_ref().ok_or("The prototype was closed")?;
                let (_, current) = layout::layers(player, &player.session)?;
                layout::signature(&current)
            };
            if current == layout::signature(&layers) {
                presented = Some((size, layers, title, can_back));
                break;
            }
        }
        let (size, layers, title, can_back) = presented
            .ok_or("The native overlay layout did not settle; inspect its content sizing")?;
        let (hotspots, focus_targets, diagnostics, changed) = {
            let graph = self.preview_prototype();
            let mut player = self.prototype.player.borrow_mut();
            let player = player
                .as_mut()
                .ok_or("The prototype was closed during layout")?;
            for original in player.session.active_stages() {
                let info = player
                    .board
                    .stage(player.stages[&original])
                    .ok_or("The native prototype screen was removed")?;
                if !matches!(info.status, PreviewStageStatus::Ready { .. }) {
                    return Err(format!(
                        "The native screen '{}' is not ready: {:?}",
                        info.configuration.name, info.status
                    ));
                }
            }
            let PrototypeHotspotFrame {
                hotspots,
                focus_targets,
                bounds,
                diagnostics,
            } = inspect_prototype_hotspots(player, &graph, size, layers.last().unwrap().bounds)?;
            let current_layout = layout::signature(&layers);
            let changed = player.bounds != bounds || player.layout != current_layout;
            player.bounds = bounds;
            player.layout = current_layout;
            player.diagnostics = diagnostics.clone();
            (hotspots, focus_targets, diagnostics, changed)
        };
        if changed {
            self.bump_prototype_generation();
        }
        view.set_hotspot_focus_targets(&focus_targets);
        view.update_layers(
            size,
            &layers,
            &title,
            &hotspots,
            can_back,
            self.prototype.generation.get(),
        )?;
        view.control().update_layout();
        view.show_error(&diagnostics.join("\n"));
        // A factory, focus or layout callback can synchronously change source.
        self.ensure_current_prototype_player()?;
        let graph = self.preview_prototype();
        let mut player = self.prototype.player.borrow_mut();
        let player = player
            .as_mut()
            .ok_or("The prototype was closed during layout")?;
        let (size, current) = layout::layers(player, &player.session)?;
        if player.layout != layout::signature(&current) {
            return Ok(false);
        }
        // Focus styles and handlers may also move, hide or replace an authored
        // hotspot without changing the frame size. Its visual identity, clipped
        // geometry and diagnostics must belong to this final native presentation.
        let accepted =
            inspect_prototype_hotspots(player, &graph, size, current.last().unwrap().bounds)?;
        Ok(player.bounds == accepted.bounds
            && player.diagnostics == accepted.diagnostics
            && focus_targets == accepted.focus_targets)
    }
}
