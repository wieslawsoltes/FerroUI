/// Describes the timing of binding source updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UpdateSourceTrigger {
    /// The default update source trigger, currently `PropertyChanged`.
    #[default]
    Default = 0,
    /// Updates the binding source immediately whenever the binding target
    /// property changes.
    PropertyChanged = 1,
    /// Updates the binding source whenever the binding target element loses
    /// focus.
    LostFocus = 2,
    /// Updates the binding source only when `update_source` is called on the
    /// binding expression.
    Explicit = 3,
}
