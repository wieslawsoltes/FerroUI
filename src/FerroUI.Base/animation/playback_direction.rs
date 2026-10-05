/// Determines the playback direction of an animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlaybackDirection {
    /// The animation is played normally.
    #[default]
    Normal = 0,
    /// The animation is played in reverse direction.
    Reverse = 1,
    /// The animation is played forwards first, then backwards.
    Alternate = 2,
    /// The animation is played backwards first, then forwards.
    AlternateReverse = 3,
}
