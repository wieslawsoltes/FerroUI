use crate::animation::{PlayState, TimeSpan};
use crate::reactive::IObservable;

/// A source of animation time: an observable that produces the time elapsed
/// since the clock started, once per frame.
///
/// Every timing decision in the animation system is derived from the values
/// a clock produces; nothing reads a system timer.
pub trait IClock: IObservable<TimeSpan> + 'static {
    /// The playback state of the clock.
    fn play_state(&self) -> PlayState;

    /// Sets the playback state of the clock.
    fn set_play_state(&self, value: PlayState);
}

impl PartialEq for dyn IClock {
    /// Clocks are compared by identity.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const dyn IClock, other as *const dyn IClock)
    }
}

impl std::fmt::Debug for dyn IClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IClock({:?})", self.play_state())
    }
}
