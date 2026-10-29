//! A play session realizes a bounded set of isolated native stages. Screen history
//! stores original board identities; native controls are retained until Exit/Play.
use super::*;
use crate::{
    PreviewPrototypeHotspot, PreviewPrototypePlayerAction, PreviewStageMode, PreviewStageStatus,
};

impl DesignerWorkspace {
    pub fn preview_prototype_playback(&self) -> Option<PreviewPrototypePlaybackInfo> {
        self.prototype
            .player
            .borrow()
            .as_ref()
            .map(|player| PreviewPrototypePlaybackInfo {
                flow: player.session.flow().into(),
                current: player.session.current(),
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
            let bounds = player
                .board
                .prototype_hotspot_bounds(native, &link.trigger)?;
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
        {
            let mut player = self.prototype.player.borrow_mut();
            let player = player.as_mut().ok_or("The prototype was closed")?;
            player
                .stages
                .get(&next.current())
                .ok_or("The destination is outside the current flow")?;
            if player.session == next {
                return Ok(());
            }
            player.session = next;
            player.bounds.clear();
            player.diagnostics.clear();
        }
        self.bump_prototype_generation();
        self.refresh_preview_prototype();
        if self.prototype.player.borrow().is_none() {
            return Err(
                "The native destination could not be presented; inspect the overview diagnostic"
                    .into(),
            );
        }
        Ok(())
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
        self.ensure_current_prototype_player()?;
        let view = self
            .preview_prototype_player_view()
            .ok_or("The native prototype player was closed")?;
        let (id, control, info, title, can_back) = {
            let player = self.prototype.player.borrow();
            let player = player.as_ref().unwrap();
            let id = *player
                .stages
                .get(&player.session.current())
                .ok_or("The prototype screen was removed")?;
            let info = player
                .board
                .stage(id)
                .ok_or("The native prototype screen was removed")?;
            (
                id,
                player
                    .board
                    .control(id)
                    .ok_or("The native prototype view is unavailable")?,
                info.clone(),
                format!("{} · {}", player.session.flow(), info.configuration.name),
                player.session.can_go_back(),
            )
        };
        self.mount_prototype_control(view.control());
        // Mount before querying desired-size/source geometry. A definition/state
        // factory may need inherited native resources from the actual logical root.
        let prior_hotspots = {
            let player = self.prototype.player.borrow();
            let player = player.as_ref().unwrap();
            let graph = self.preview_prototype();
            let mut hotspots = player
                .bounds
                .iter()
                .filter_map(|(index, bounds)| {
                    graph.links.get(*index).map(|link| PreviewPrototypeHotspot {
                        link: *index,
                        name: link.name.clone(),
                        bounds: *bounds,
                    })
                })
                .collect::<Vec<_>>();
            hotspots.sort_by_key(|hotspot| {
                (graph.links[hotspot.link].trigger.path.len(), hotspot.link)
            });
            hotspots
        };
        view.update(
            control.clone(),
            info.layout_size,
            &title,
            &prior_hotspots,
            can_back,
            self.prototype.generation.get(),
        );
        view.control().update_layout();
        if matches!(info.status, PreviewStageStatus::Failed { .. }) {
            self.prototype
                .player
                .borrow_mut()
                .as_mut()
                .unwrap()
                .board
                .refresh(id)?;
            view.control().update_layout();
        }
        let (size, hotspots, diagnostics, changed) = {
            let graph = self.preview_prototype();
            let mut player = self.prototype.player.borrow_mut();
            let player = player
                .as_mut()
                .ok_or("The prototype was closed during layout")?;
            player.board.refresh_runtime_mapping(id)?;
            let info = player
                .board
                .stage(id)
                .ok_or("The native prototype screen was removed")?;
            if !matches!(info.status, PreviewStageStatus::Ready { .. }) {
                return Err(format!(
                    "The native screen '{}' is not ready: {:?}",
                    info.configuration.name, info.status
                ));
            }
            let mut hotspots = Vec::new();
            let mut diagnostics = Vec::new();
            let mut indexed = graph
                .links
                .iter()
                .enumerate()
                .filter(|(_, link)| link.source == player.session.current())
                .collect::<Vec<_>>();
            // Deeper authored controls are last in the native overlay z-order.
            indexed.sort_by_key(|(index, link)| (link.trigger.path.len(), *index));
            for (index, link) in indexed {
                match player.board.prototype_hotspot_bounds(id, &link.trigger) {
                    Ok(bounds) => hotspots.push(PreviewPrototypeHotspot {
                        link: index,
                        name: link.name.clone(),
                        bounds,
                    }),
                    Err(error) => diagnostics.push(format!("{}: {error}", link.name)),
                }
            }
            let bounds = hotspots
                .iter()
                .map(|hotspot| (hotspot.link, hotspot.bounds))
                .collect::<BTreeMap<_, _>>();
            let changed = player.bounds != bounds;
            player.bounds = bounds;
            player.diagnostics = diagnostics.clone();
            (info.layout_size, hotspots, diagnostics, changed)
        };
        if changed {
            self.bump_prototype_generation();
        }
        view.update(
            control,
            size,
            &title,
            &hotspots,
            can_back,
            self.prototype.generation.get(),
        );
        view.control().update_layout();
        view.show_error(&diagnostics.join("\n"));
        // A factory/layout callback can synchronously change the host's source.
        self.ensure_current_prototype_player()?;
        Ok(())
    }
}
