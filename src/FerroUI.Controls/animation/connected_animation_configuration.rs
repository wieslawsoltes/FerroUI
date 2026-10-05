use ferroui_base::animation::TimeSpan;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// Base class for connected animation configurations that control the
/// visual style and physics of the transition.
pub trait ConnectedAnimationConfiguration: Any {
    /// The configuration as [`Any`], for the class checks of
    /// [`ConnectedAnimation`](super::ConnectedAnimation).
    fn as_any(&self) -> &dyn Any;
}

/// Configurations compare by reference.
impl PartialEq for dyn ConnectedAnimationConfiguration {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// Produces a gravity-physics effect suitable for forward navigation: the
/// element arcs slightly as it travels and casts an animated shadow. This is
/// the default configuration when none is specified.
///
/// Use [`DirectConnectedAnimationConfiguration`] for back navigation and
/// [`BasicConnectedAnimationConfiguration`] for a plain transition.
pub struct GravityConnectedAnimationConfiguration {
    is_shadow_enabled: Cell<bool>,
}

impl GravityConnectedAnimationConfiguration {
    /// Creates the configuration.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { is_shadow_enabled: Cell::new(true) })
    }

    /// Gets whether a drop shadow is rendered beneath the element during the
    /// gravity arc. Defaults to `true`.
    pub fn is_shadow_enabled(&self) -> bool {
        self.is_shadow_enabled.get()
    }

    /// Sets whether a drop shadow is rendered beneath the element during the
    /// gravity arc.
    pub fn set_is_shadow_enabled(&self, value: bool) {
        self.is_shadow_enabled.set(value);
    }

    /// Sets [`is_shadow_enabled`](Self::is_shadow_enabled) and returns the
    /// configuration.
    pub fn with_is_shadow_enabled(self: Rc<Self>, value: bool) -> Rc<Self> {
        self.set_is_shadow_enabled(value);
        self
    }
}

impl ConnectedAnimationConfiguration for GravityConnectedAnimationConfiguration {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Produces a direct, linear translation suitable for back navigation. No
/// gravity arc or shadow is applied, and the default duration is shorter
/// (150 ms).
///
/// Assign this to the configuration of a
/// [`ConnectedAnimation`](super::ConnectedAnimation) before starting the
/// return animation to animate back to the source view.
pub struct DirectConnectedAnimationConfiguration {
    duration: Cell<Option<TimeSpan>>,
}

impl DirectConnectedAnimationConfiguration {
    /// Creates the configuration.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { duration: Cell::new(None) })
    }

    /// Gets the duration of the animation. When `None`, the default duration
    /// of the [`ConnectedAnimationService`](super::ConnectedAnimationService)
    /// is used.
    pub fn duration(&self) -> Option<TimeSpan> {
        self.duration.get()
    }

    /// Sets the duration of the animation.
    pub fn set_duration(&self, value: Option<TimeSpan>) {
        self.duration.set(value);
    }

    /// Sets [`duration`](Self::duration) and returns the configuration.
    pub fn with_duration(self: Rc<Self>, value: Option<TimeSpan>) -> Rc<Self> {
        self.set_duration(value);
        self
    }
}

impl ConnectedAnimationConfiguration for DirectConnectedAnimationConfiguration {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Produces a simple ease-in-out transition between the source and
/// destination elements with no gravity arc or shadow. The duration is the
/// default duration of the
/// [`ConnectedAnimationService`](super::ConnectedAnimationService).
pub struct BasicConnectedAnimationConfiguration {}

impl BasicConnectedAnimationConfiguration {
    /// Creates the configuration.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {})
    }
}

impl ConnectedAnimationConfiguration for BasicConnectedAnimationConfiguration {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
