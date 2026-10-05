/// This enum offers different ways of evaluating the IsOffscreen AutomationProperty
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IsOffscreenBehavior {
    /// The AutomationProperty IsOffscreen is calculated based on IsEffectivelyVisible.
    Default,
    /// The AutomationProperty IsOffscreen is false.
    Onscreen,
    /// The AutomationProperty IsOffscreen if true.
    Offscreen,
    /// The AutomationProperty IsOffscreen is calculated based on clip regions.
    FromClip,
}
