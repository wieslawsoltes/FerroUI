/// Determines whether an animation pauses when its target is not effectively
/// visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlaybackBehavior {
    /// The system decides based on context: animations started from styles
    /// pause when the control is not effectively visible, while manually
    /// started animations and animations that animate visibility run
    /// regardless.
    #[default]
    Auto = 0,
    /// The animation always plays regardless of the control's effective
    /// visibility.
    Always = 1,
    /// The animation pauses when the control is not effectively visible.
    OnlyIfVisible = 2,
}
