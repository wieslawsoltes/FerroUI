use crate::CompositionAnimationFactory;
use ferroui_base::media::{Stretch, StretchDirection};
use std::time::Duration;

#[derive(Clone, Copy)]
pub(crate) struct PlaybackSettings {
    pub stretch: Stretch,
    pub stretch_direction: StretchDirection,
    pub repeat_count: i32,
    pub playback_rate: f64,
}

pub(crate) enum VisualCommand {
    Replace(Option<CompositionAnimationFactory>, bool),
    Start,
    Stop,
    Pause,
    Resume,
    Seek(Duration),
    Update,
    Redraw,
}

pub(crate) struct VisualPayload {
    pub command: VisualCommand,
    pub generation: u64,
    pub revision: u64,
    pub settings: PlaybackSettings,
}
