#[path = "stages_prototype_native.rs"]
mod native;

use crate::protocol::Image;
use ferroui_base::{Point, Ref};
use ferroui_controls::Control as NativeControl;
use ferroui_designer::{
    PreviewPrototypeInput, PreviewPrototypeOverlay, PreviewPrototypePlayerAction,
    PreviewPrototypeSession, PreviewStageBoard, PreviewStageFocus, PreviewStageId,
    PreviewStageKeyboardInput, PreviewStagePointerOrigin, PreviewStagePointerPhase,
};
use native::NativePlayer;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub(crate) struct Playback {
    player: Option<NativePlayer>,
    frame: u64,
}

#[derive(Deserialize)]
pub(crate) struct Control {
    pub(crate) epoch: u64,
    #[serde(default, rename = "expectedFrame")]
    pub(crate) expected_frame: Option<u64>,
    #[serde(flatten)]
    pub(crate) action: Command,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum Command {
    Start { flow: String },
    Back,
    CloseOverlay,
    DismissOverlay,
    Restart,
    Stop,
    Activate { link: usize },
}

#[derive(Serialize)]
pub(crate) struct Scene {
    flow: String,
    pub(crate) screen: PreviewStageId,
    pub(crate) current: PreviewStageId,
    pub(crate) overlays: Vec<PreviewPrototypeOverlay>,
    can_go_back: bool,
    frame: u64,
    links: Vec<usize>,
    diagnostics: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct FrameRequest {
    frame: u64,
}

#[derive(Deserialize)]
pub(crate) struct InputRequest {
    frame: u64,
    #[serde(default)]
    automation: bool,
    input: PreviewPrototypeInput,
}

#[derive(Serialize)]
pub(crate) struct InputResult {
    scene: Option<Scene>,
    focus: Option<PreviewStageFocus>,
}

impl Playback {
    pub(crate) fn stop(&mut self) {
        self.player.take();
    }

    pub(crate) fn scene(&self) -> Option<Scene> {
        let player = self.player.as_ref()?;
        let session = &player.session;
        Some(Scene {
            flow: session.flow().into(),
            screen: session.screen(),
            current: session.current(),
            overlays: session.overlays().to_vec(),
            can_go_back: session.can_go_back(),
            frame: player.frame,
            links: player.links.keys().copied().collect(),
            diagnostics: player.diagnostics.clone(),
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
        if let Some(frame) = request.expected_frame {
            self.check_frame(frame)?;
        }
        match request.action {
            Command::Start { flow } => {
                let session = PreviewPrototypeSession::start(&board.prototype(), &flow)?;
                let frame = self.next_frame();
                let player = NativePlayer::start(board, session, frame)?;
                self.player = Some(player);
            }
            action => self.apply_action(action)?,
        }
        Ok(self.scene())
    }

    fn apply_action(&mut self, action: Command) -> Result<(), String> {
        if matches!(action, Command::Stop) {
            self.stop();
            return Ok(());
        }
        let frame = self.next_frame();
        let player = self.player.as_mut().ok_or("Start a prototype flow first")?;
        let mut next = player.session.clone();
        match action {
            Command::Back => {
                next.back()?;
            }
            Command::CloseOverlay => {
                next.close_overlay()?;
            }
            Command::DismissOverlay => {
                if !next
                    .overlays()
                    .last()
                    .is_some_and(|overlay| overlay.options.dismiss_on_outside)
                {
                    return Err("This overlay requires an explicit close action".into());
                }
                next.close_overlay()?;
            }
            Command::Restart => {
                next.restart();
            }
            Command::Activate { link } => {
                player.validate_link(link)?;
                next.activate_link(link)?;
            }
            Command::Start { .. } | Command::Stop => unreachable!(),
        }
        player.navigate(next, frame)
    }

    fn next_frame(&mut self) -> u64 {
        self.frame = self
            .frame
            .checked_add(1)
            .expect("Prototype frame counter exhausted");
        self.frame
    }

    fn check_frame(&self, frame: u64) -> Result<&NativePlayer, String> {
        self.player
            .as_ref()
            .filter(|player| player.frame == frame)
            .ok_or_else(|| "The prototype changed. Use its current frame.".into())
    }

    pub(crate) fn input(&mut self, request: InputRequest) -> Result<InputResult, String> {
        let player = self.check_frame(request.frame)?;
        let (mut focus, settle) = match request.input {
            PreviewPrototypeInput::Keyboard { input } => (player.keyboard(input)?, true),
            PreviewPrototypeInput::Pointer { input } => {
                let origin = if request.automation {
                    PreviewStagePointerOrigin::Automation
                } else {
                    PreviewStagePointerOrigin::Editor
                };
                player.pointer(input, origin)?;
                (
                    None,
                    matches!(
                        input.phase,
                        PreviewStagePointerPhase::Up | PreviewStagePointerPhase::Cancel
                    ),
                )
            }
        };
        if self.flush_action()? {
            if let Some(player) = &self.player {
                focus = player.keyboard(PreviewStageKeyboardInput::Focus)?;
            }
        } else if settle {
            self.player.as_mut().unwrap().present()?;
        }
        Ok(InputResult {
            scene: self.scene(),
            focus,
        })
    }

    pub(crate) fn flush_action(&mut self) -> Result<bool, String> {
        let Some(action) = self.player.as_ref().and_then(NativePlayer::take_action) else {
            return Ok(false);
        };
        let command = match action {
            PreviewPrototypePlayerAction::Activate { link } => Command::Activate { link },
            PreviewPrototypePlayerAction::Back => Command::Back,
            PreviewPrototypePlayerAction::CloseOverlay => Command::CloseOverlay,
            PreviewPrototypePlayerAction::DismissOverlay => Command::DismissOverlay,
            PreviewPrototypePlayerAction::Restart => Command::Restart,
            PreviewPrototypePlayerAction::Exit => Command::Stop,
        };
        self.apply_action(command)?;
        Ok(true)
    }

    pub(crate) fn image(&mut self, request: FrameRequest) -> Result<Image, String> {
        self.check_frame(request.frame)?.image()
    }

    pub(crate) fn active_control(&self) -> Result<Ref<NativeControl>, String> {
        self.player
            .as_ref()
            .ok_or("Start a prototype flow first")?
            .active_control()
    }

    pub(crate) fn active_image(&mut self) -> Result<Image, String> {
        self.player
            .as_mut()
            .ok_or("Start a prototype flow first")?
            .active_image()
    }

    pub(crate) fn artboard_image(&self) -> Result<Image, String> {
        self.player
            .as_ref()
            .ok_or("Start a prototype flow first")?
            .image()
    }

    pub(crate) fn active_focus(&self) -> Result<Option<PreviewStageFocus>, String> {
        self.player
            .as_ref()
            .ok_or("Start a prototype flow first")?
            .focus()
    }

    pub(crate) fn press_key(&mut self, key: String) -> Result<(), String> {
        let frame = self
            .player
            .as_ref()
            .ok_or("Start a prototype flow first")?
            .frame;
        self.input(InputRequest {
            frame,
            automation: true,
            input: PreviewPrototypeInput::Keyboard {
                input: PreviewStageKeyboardInput::Key { key },
            },
        })?;
        Ok(())
    }

    pub(crate) fn cancel_editor_contacts(&mut self) {
        if let Some(player) = &mut self.player {
            player.cancel(PreviewStagePointerOrigin::Editor);
        }
    }

    /// Compatibility with clients that still present the individual stage images.
    pub(crate) fn interact(&mut self, stage: PreviewStageId, point: Point) -> Result<bool, String> {
        let Some(player) = &self.player else {
            return Ok(false);
        };
        if player.session.current() != stage {
            return Err("Interact with the active prototype layer".into());
        }
        if let Some((&link, _)) = player
            .links
            .iter()
            .rev()
            .find(|(_, bounds)| bounds.contains(point))
        {
            self.apply_action(Command::Activate { link })?;
        }
        Ok(true)
    }
}
