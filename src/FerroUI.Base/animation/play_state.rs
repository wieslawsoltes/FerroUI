/// Determines the playback state of an animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlayState {
    /// The animation is running.
    #[default]
    Run = 0,
    /// The animation is paused.
    Pause = 1,
    /// The animation is stopped.
    Stop = 2,
}
