//! Optional UI-thread timings at the native layout and declarative view boundaries.
use crate::{Ref, Visual};
use std::{any::Any, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    Measure,
    Arrange,
    Render,
    Reconcile,
}

impl Phase {
    pub const ALL: [Self; 4] = [Self::Measure, Self::Arrange, Self::Render, Self::Reconcile];
}

#[cfg(feature = "ui-profiling")]
#[path = "ui_profile_recording.rs"]
mod recording;
#[cfg(feature = "ui-profiling")]
pub use recording::{Capture, Metric, Row, Snapshot, ProfileSpan};

#[cfg(not(feature = "ui-profiling"))]
pub struct ProfileSpan;

#[cfg(not(feature = "ui-profiling"))]
impl ProfileSpan {
    #[inline]
    pub fn set_visual(&self, _: Option<Ref<Visual>>) {}
}

/// Records an actual layout operation after its cached result has been ruled out.
#[cfg(feature = "ui-profiling")]
pub(crate) fn layout(control: &Visual, phase: Phase) -> Option<ProfileSpan> {
    recording::layout(control, phase)
}

/// Times an instance of a declarative component. Resolve its current output and
/// host only during a capture. The host identifies the window before mounting.
#[inline]
pub fn component<T: Any>(
    instance: &Rc<T>,
    name: &'static str,
    phase: Phase,
    views: impl FnOnce() -> (Option<Ref<Visual>>, Option<Ref<Visual>>),
) -> Option<ProfileSpan> {
    #[cfg(feature = "ui-profiling")]
    {
        recording::component(instance, name, phase, views)
    }
    #[cfg(not(feature = "ui-profiling"))]
    {
        let _ = (instance, name, phase, views);
        None
    }
}
