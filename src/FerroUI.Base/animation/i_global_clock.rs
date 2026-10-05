use crate::animation::IClock;

/// The clock that every animation without an explicit clock runs on.
///
/// It is registered with the service locator as `dyn IGlobalClock` and is
/// pulsed once per frame by the media context, on the UI thread, with the
/// time elapsed since the media context started.
pub trait IGlobalClock: IClock {}
