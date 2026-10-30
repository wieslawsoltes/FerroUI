//! WinUI's `VisualStateChangedEventArgs` (`Microsoft.UI.Xaml.VisualStateChangedEventArgs`,
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::VisualState;
use crate::Control;
use ferroui_base::Ref;

/// The arguments of `CurrentStateChanging` and `CurrentStateChanged` of a
/// [`VisualStateGroup`](super::VisualStateGroup).
#[derive(Clone, Debug, Default)]
pub struct VisualStateChangedEventArgs {
    old_state: Option<Ref<VisualState>>,
    new_state: Option<Ref<VisualState>>,
    control: Option<Ref<Control>>,
}

impl VisualStateChangedEventArgs {
    pub fn new(
        old_state: Option<Ref<VisualState>>,
        new_state: Option<Ref<VisualState>>,
        control: Option<Ref<Control>>,
    ) -> Self {
        Self { old_state, new_state, control }
    }

    /// The state the group leaves (`None` for no state).
    pub fn old_state(&self) -> Option<Ref<VisualState>> {
        self.old_state.clone()
    }

    /// The state the group enters (`None` for no state).
    pub fn new_state(&self) -> Option<Ref<VisualState>> {
        self.new_state.clone()
    }

    /// The control whose states change.
    pub fn control(&self) -> Option<Ref<Control>> {
        self.control.clone()
    }
}
