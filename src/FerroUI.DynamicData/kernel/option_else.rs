//! `OptionElse` (`Kernel/OptionElse.cs`).

/// What `if_has_value` returns: runs the action of [`else_`](Self::else_) when the option was
/// empty (`OptionElse`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OptionElse {
    should_run_action: bool,
}

impl OptionElse {
    /// The result of an empty option, whose `else_` runs its action (`new OptionElse()`).
    pub(crate) fn run() -> Self {
        Self { should_run_action: true }
    }

    /// The result of an option with a value, whose `else_` does nothing (`OptionElse.NoAction`).
    pub(crate) fn no_action() -> Self {
        Self { should_run_action: false }
    }

    /// Runs `action` when the option was empty (`Else(Action)`; `else` is a keyword of Rust).
    pub fn else_(&self, action: impl FnOnce()) {
        if self.should_run_action {
            action();
        }
    }
}
