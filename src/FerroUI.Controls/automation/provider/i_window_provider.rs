use crate::automation::{AutomationError, peers::AutomationPeer};
use ferroui_base::Ref;

/// How a window is displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowVisualState {
    Normal,
    Maximized,
    Minimized,
}

/// Whether a window is available for user interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowInteractionState {
    Running,
    Closing,
    ReadyForUserInteraction,
    BlockedByModalWindow,
    NotResponding,
}

/// Allows assistive technology to inspect and close a window or an embedded dialog.
pub trait IWindowProvider {
    /// The peer that provides this contract.
    fn peer(&self) -> Ref<AutomationPeer>;

    /// Requests that the window close.
    fn close(&self) -> Result<(), AutomationError>;

    /// Requests a visual state. Returns `false` when the window leaves its state unchanged.
    fn set_visual_state(&self, _state: WindowVisualState) -> Result<bool, AutomationError> {
        Ok(false)
    }

    /// Checks input readiness. A provider on the UI thread should return promptly.
    fn wait_for_input_idle(&self, _timeout_milliseconds: i32) -> bool {
        true
    }

    /// Whether the window can be maximized.
    fn maximizable(&self) -> bool {
        false
    }

    /// Whether the window can be minimized.
    fn minimizable(&self) -> bool {
        false
    }

    /// Whether the window is modal.
    fn is_modal(&self) -> Result<bool, AutomationError>;

    /// Whether the window is topmost.
    fn is_topmost(&self) -> Result<bool, AutomationError>;

    /// The current visual state.
    fn visual_state(&self) -> WindowVisualState {
        WindowVisualState::Normal
    }

    /// The current interaction state.
    fn interaction_state(&self) -> Result<WindowInteractionState, AutomationError>;
}

impl PartialEq for dyn IWindowProvider {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
