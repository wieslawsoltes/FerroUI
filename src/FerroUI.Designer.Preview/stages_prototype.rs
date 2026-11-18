use ferroui_base::Point;
use ferroui_designer::{
    PreviewPrototypeOverlay, PreviewPrototypeSession, PreviewStageBoard, PreviewStageId,
    PreviewStageStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(crate) struct Playback(Option<PreviewPrototypeSession>);

#[derive(Deserialize)]
pub(crate) struct Control {
    epoch: u64,
    #[serde(flatten)]
    action: Command,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum Command {
    Start { flow: String },
    Back,
    CloseOverlay,
    Restart,
    Stop,
    Activate { link: usize },
}

#[derive(Serialize)]
pub(crate) struct Scene {
    flow: String,
    screen: PreviewStageId,
    current: PreviewStageId,
    overlays: Vec<PreviewPrototypeOverlay>,
    can_go_back: bool,
}

impl Playback {
    pub(crate) fn stop(&mut self) {
        self.0.take();
    }

    pub(crate) fn scene(&self) -> Option<Scene> {
        let player = self.0.as_ref()?;
        Some(Scene {
            flow: player.flow().into(),
            screen: player.screen(),
            current: player.current(),
            overlays: player.overlays().to_vec(),
            can_go_back: player.can_go_back(),
        })
    }

    pub(crate) fn control(
        &mut self,
        board: &PreviewStageBoard,
        request: Control,
    ) -> Result<Option<Scene>, String> {
        if request.epoch != board.epoch() {
            return Err("The stage set changed before playback".into());
        }
        match request.action {
            Command::Start { flow } => {
                let graph = board.prototype();
                graph.validate(
                    &board
                        .stages()
                        .iter()
                        .map(|stage| stage.id)
                        .collect::<Vec<_>>(),
                )?;
                let player = PreviewPrototypeSession::start(&graph, &flow)?;
                if graph.reachable(player.screen()).into_iter().any(|id| {
                    !board.stage(id).is_some_and(|stage| {
                        matches!(stage.status, PreviewStageStatus::Ready { .. })
                    })
                }) {
                    return Err("Refresh every screen in the flow before playing it".into());
                }
                self.0 = Some(player);
            }
            Command::Stop => self.stop(),
            action => {
                let player = self.0.as_mut().ok_or("Start a prototype flow first")?;
                match action {
                    Command::Back => {
                        player.back()?;
                    }
                    Command::CloseOverlay => {
                        player.close_overlay()?;
                    }
                    Command::Restart => {
                        player.restart();
                    }
                    Command::Activate { link } => {
                        let graph = board.prototype();
                        let trigger = graph
                            .links
                            .get(link)
                            .filter(|link| link.source == player.current())
                            .ok_or("Choose an interaction on the current screen")?;
                        board.prototype_hotspot_bounds(trigger.source, &trigger.trigger)?;
                        player.activate_link(link)?;
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(self.scene())
    }

    pub(crate) fn interact(
        &mut self,
        board: &PreviewStageBoard,
        stage: PreviewStageId,
        point: Point,
    ) -> Result<bool, String> {
        let Some(player) = &mut self.0 else {
            return Ok(false);
        };
        if player.current() != stage {
            return Err("Interact with the current prototype screen".into());
        }
        for link in board
            .prototype()
            .links
            .iter()
            .filter(|link| link.source == stage)
        {
            if board
                .prototype_hotspot_bounds(stage, &link.trigger)
                .is_ok_and(|bounds| bounds.contains(point))
            {
                player.activate(link)?;
                return Ok(true);
            }
        }
        Ok(true)
    }
}
