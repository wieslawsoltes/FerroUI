//! `BindingOptions` (`Binding/BindingOptions.cs`).

/// The options of the binding of change sets to an observable collection (`BindingOptions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindingOptions {
    /// The number of changes above which the collection is loaded again and a reset is raised
    /// (`ResetThreshold`).
    pub reset_threshold: usize,
    /// Whether an update replaces the item in place, rather than removing and adding it
    /// (`UseReplaceForUpdates`).
    pub use_replace_for_updates: bool,
    /// Whether the first change set loads the collection with a reset (`ResetOnFirstTimeLoad`).
    pub reset_on_first_time_load: bool,
}

impl BindingOptions {
    /// `DefaultResetThreshold`.
    pub const DEFAULT_RESET_THRESHOLD: usize = 25;
    /// `DefaultUseReplaceForUpdates`.
    pub const DEFAULT_USE_REPLACE_FOR_UPDATES: bool = true;
    /// `DefaultResetOnFirstTimeLoad`.
    pub const DEFAULT_RESET_ON_FIRST_TIME_LOAD: bool = true;

    /// The options with the threshold and upstream's defaults for the others
    /// (`BindingOptions(int ResetThreshold)`).
    pub const fn new(reset_threshold: usize) -> Self {
        Self { reset_threshold, use_replace_for_updates: Self::DEFAULT_USE_REPLACE_FOR_UPDATES, reset_on_first_time_load: Self::DEFAULT_RESET_ON_FIRST_TIME_LOAD }
    }

    /// Every part (`BindingOptions(int, bool, bool)`).
    pub const fn with_all(reset_threshold: usize, use_replace_for_updates: bool, reset_on_first_time_load: bool) -> Self {
        Self { reset_threshold, use_replace_for_updates, reset_on_first_time_load }
    }

    /// The options that never reset the collection (`NeverFireReset(bool useReplaceForUpdates =
    /// true)`).
    pub const fn never_fire_reset() -> Self {
        Self::never_fire_reset_with(Self::DEFAULT_RESET_ON_FIRST_TIME_LOAD)
    }

    /// [`never_fire_reset`](Self::never_fire_reset) with the choice of replacements
    /// (`NeverFireReset(bool)`).
    pub const fn never_fire_reset_with(use_replace_for_updates: bool) -> Self {
        Self { reset_threshold: usize::MAX, use_replace_for_updates, reset_on_first_time_load: false }
    }

    /// A copy with another threshold (`with { ResetThreshold = .. }`).
    pub const fn with_reset_threshold(self, reset_threshold: usize) -> Self {
        Self { reset_threshold, ..self }
    }

    /// A copy with another choice of replacements (`with { UseReplaceForUpdates = .. }`).
    pub const fn with_use_replace_for_updates(self, use_replace_for_updates: bool) -> Self {
        Self { use_replace_for_updates, ..self }
    }

    /// A copy with another choice of the first load (`with { ResetOnFirstTimeLoad = .. }`).
    pub const fn with_reset_on_first_time_load(self, reset_on_first_time_load: bool) -> Self {
        Self { reset_on_first_time_load, ..self }
    }
}

impl Default for BindingOptions {
    /// `new BindingOptions(DefaultResetThreshold)`.
    fn default() -> Self {
        Self::new(Self::DEFAULT_RESET_THRESHOLD)
    }
}
