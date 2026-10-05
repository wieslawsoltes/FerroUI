use std::fmt;

/// Defines the priorities with which jobs can be invoked on a
/// [`Dispatcher`](super::Dispatcher).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DispatcherPriority {
    value: i32,
}

impl DispatcherPriority {
    const fn new(value: i32) -> Self {
        Self { value }
    }

    /// The numeric value of the priority.
    #[inline]
    pub const fn value(self) -> i32 {
        self.value
    }

    /// The lowest foreground dispatcher priority.
    pub const DEFAULT: Self = Self::new(0);

    pub(crate) const MINIMUM_FOREGROUND_PRIORITY: Self = Self::DEFAULT;

    /// The job will be processed with the same priority as input.
    pub const INPUT: Self = Self::new(Self::DEFAULT.value - 1);

    /// The job will be processed after other non-idle operations have completed.
    pub const BACKGROUND: Self = Self::new(Self::INPUT.value - 1);

    /// The job will be processed after background operations have completed.
    pub const CONTEXT_IDLE: Self = Self::new(Self::BACKGROUND.value - 1);

    /// The job will be processed when the application is idle.
    pub const APPLICATION_IDLE: Self = Self::new(Self::CONTEXT_IDLE.value - 1);

    /// The job will be processed when the system is idle.
    pub const SYSTEM_IDLE: Self = Self::new(Self::APPLICATION_IDLE.value - 1);

    /// Minimum possible priority that's actually dispatched, default value.
    pub(crate) const MINIMUM_ACTIVE_VALUE: Self = Self::new(Self::SYSTEM_IDLE.value);

    /// A dispatcher priority for jobs that shouldn't be executed yet.
    pub const INACTIVE: Self = Self::new(Self::MINIMUM_ACTIVE_VALUE.value - 1);

    /// Minimum valid priority.
    pub(crate) const MIN_VALUE: Self = Self::new(Self::INACTIVE.value);

    /// Used internally in dispatcher code.
    pub const INVALID: Self = Self::new(Self::MINIMUM_ACTIVE_VALUE.value - 2);

    /// The job will be processed after layout and render but before input.
    pub const LOADED: Self = Self::new(Self::DEFAULT.value + 1);

    /// A special priority for platforms with a UI render timer or for forced
    /// full rasterization requests.
    pub const UI_THREAD_RENDER: Self = Self::new(Self::LOADED.value + 1);

    /// A special priority to synchronize native control host positions,
    /// IME, etc. Renderer results have to be reported before any user code
    /// and most of the system input callbacks.
    pub const AFTER_RENDER: Self = Self::new(Self::UI_THREAD_RENDER.value + 1);

    /// The job will be processed with the same priority as render.
    pub const RENDER: Self = Self::new(Self::AFTER_RENDER.value + 1);

    /// A special platform hook for jobs to be executed before the normal
    /// render cycle.
    pub const BEFORE_RENDER: Self = Self::new(Self::RENDER.value + 1);

    /// A special priority for platforms that resize the render target in
    /// asynchronous-commit mode.
    pub const ASYNC_RENDER_TARGET_RESIZE: Self = Self::new(Self::BEFORE_RENDER.value + 1);

    /// The job will be processed with the same priority as data binding.
    #[deprecated(note = "WPF compatibility")]
    pub const DATA_BIND: Self = Self::new(Self::ASYNC_RENDER_TARGET_RESIZE.value + 1);

    /// The job will be processed with normal priority.
    pub const NORMAL: Self = Self::new(Self::ASYNC_RENDER_TARGET_RESIZE.value + 2);

    /// The job will be processed before other asynchronous operations.
    pub const SEND: Self = Self::new(Self::NORMAL.value + 1);

    /// Maximum possible priority.
    pub const MAX_VALUE: Self = Self::SEND;

    /// Creates a priority from its numeric value.
    ///
    /// # Panics
    /// Panics when `value` is outside the valid range; use
    /// [`TryFrom<i32>`] for untrusted input.
    pub fn from_value(value: i32) -> Self {
        match Self::try_from(value) {
            Ok(priority) => priority,
            Err(_) => panic!("value: specified argument was out of the range of valid values"),
        }
    }

    /// Verifies that `priority` can be used to queue work.
    ///
    /// # Panics
    /// Panics when the priority is outside `INACTIVE..=MAX_VALUE`.
    pub fn validate(priority: DispatcherPriority, parameter_name: &str) {
        if priority < Self::INACTIVE || priority > Self::MAX_VALUE {
            panic!("Invalid DispatcherPriority value (parameter '{parameter_name}')");
        }
    }
}

impl Default for DispatcherPriority {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The error returned when converting an out-of-range number to a
/// [`DispatcherPriority`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatcherPriorityOutOfRangeError(pub i32);

impl fmt::Display for DispatcherPriorityOutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is not a valid dispatcher priority", self.0)
    }
}

impl std::error::Error for DispatcherPriorityOutOfRangeError {}

impl TryFrom<i32> for DispatcherPriority {
    type Error = DispatcherPriorityOutOfRangeError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if !(Self::MIN_VALUE.value..=Self::MAX_VALUE.value).contains(&value) {
            Err(DispatcherPriorityOutOfRangeError(value))
        } else {
            Ok(Self::new(value))
        }
    }
}

impl From<DispatcherPriority> for i32 {
    fn from(priority: DispatcherPriority) -> i32 {
        priority.value
    }
}

#[allow(deprecated)]
impl fmt::Display for DispatcherPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match *self {
            Self::INVALID => "Invalid",
            Self::INACTIVE => "Inactive",
            Self::SYSTEM_IDLE => "SystemIdle",
            Self::CONTEXT_IDLE => "ContextIdle",
            Self::APPLICATION_IDLE => "ApplicationIdle",
            Self::BACKGROUND => "Background",
            Self::INPUT => "Input",
            Self::DEFAULT => "Default",
            Self::LOADED => "Loaded",
            Self::UI_THREAD_RENDER => "UiThreadRender",
            Self::AFTER_RENDER => "AfterRender",
            Self::RENDER => "Render",
            Self::BEFORE_RENDER => "BeforeRender",
            Self::ASYNC_RENDER_TARGET_RESIZE => "AsyncRenderTargetResize",
            Self::DATA_BIND => "DataBind",
            Self::NORMAL => "Normal",
            Self::SEND => "Send",
            _ => return write!(f, "{}", self.value),
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn values_match_the_documented_ordering() {
        assert_eq!(DispatcherPriority::INVALID.value(), -7);
        assert_eq!(DispatcherPriority::INACTIVE.value(), -6);
        assert_eq!(DispatcherPriority::SYSTEM_IDLE.value(), -5);
        assert_eq!(DispatcherPriority::APPLICATION_IDLE.value(), -4);
        assert_eq!(DispatcherPriority::CONTEXT_IDLE.value(), -3);
        assert_eq!(DispatcherPriority::BACKGROUND.value(), -2);
        assert_eq!(DispatcherPriority::INPUT.value(), -1);
        assert_eq!(DispatcherPriority::DEFAULT.value(), 0);
        assert_eq!(DispatcherPriority::LOADED.value(), 1);
        assert_eq!(DispatcherPriority::UI_THREAD_RENDER.value(), 2);
        assert_eq!(DispatcherPriority::AFTER_RENDER.value(), 3);
        assert_eq!(DispatcherPriority::RENDER.value(), 4);
        assert_eq!(DispatcherPriority::BEFORE_RENDER.value(), 5);
        assert_eq!(DispatcherPriority::ASYNC_RENDER_TARGET_RESIZE.value(), 6);
        assert_eq!(DispatcherPriority::DATA_BIND.value(), 7);
        assert_eq!(DispatcherPriority::NORMAL.value(), 8);
        assert_eq!(DispatcherPriority::SEND.value(), 9);
        assert_eq!(DispatcherPriority::default(), DispatcherPriority::DEFAULT);
        assert!(DispatcherPriority::RENDER > DispatcherPriority::INPUT);
    }

    #[test]
    fn from_value_validates_the_range() {
        assert_eq!(DispatcherPriority::try_from(4), Ok(DispatcherPriority::RENDER));
        assert!(DispatcherPriority::try_from(-7).is_err());
        assert!(DispatcherPriority::try_from(10).is_err());
        assert_eq!(i32::from(DispatcherPriority::SEND), 9);
    }

    #[test]
    fn display_uses_the_priority_name() {
        assert_eq!(DispatcherPriority::RENDER.to_string(), "Render");
        assert_eq!(DispatcherPriority::INVALID.to_string(), "Invalid");
        assert_eq!(DispatcherPriority::UI_THREAD_RENDER.to_string(), "UiThreadRender");
    }

    #[test]
    #[should_panic(expected = "Invalid DispatcherPriority value")]
    fn validate_rejects_invalid() {
        DispatcherPriority::validate(DispatcherPriority::INVALID, "priority");
    }
}
