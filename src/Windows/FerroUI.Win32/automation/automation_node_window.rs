//! Dispatch the UI Automation Window pattern to the peer on the UI thread.

use super::automation_node::AutomationNode;
use super::interop::{IWindowProviderImpl, WindowInteractionState, WindowVisualState};
use ferroui_controls::automation::provider::{
    IWindowProvider, WindowInteractionState as Interaction, WindowVisualState as Visual,
};
use ferroui_microcom::HResult;
use std::rc::Rc;

const S_FALSE: HResult = HResult(1);

impl IWindowProviderImpl for AutomationNode {
    fn set_visual_state(&self, state: WindowVisualState) -> Result<(), HResult> {
        let state = match state {
            WindowVisualState::Normal => Visual::Normal,
            WindowVisualState::Maximized => Visual::Maximized,
            WindowVisualState::Minimized => Visual::Minimized,
            _ => return Err(HResult::INVALIDARG),
        };
        let changed = self.invoke_sync_provider(move |provider: &Rc<dyn IWindowProvider>| {
            Self::enabled(provider.set_visual_state(state))
        })?;
        // S_FALSE is the successful COM result for a visual state that did not change.
        if changed { Ok(()) } else { Err(S_FALSE) }
    }

    fn close(&self) -> Result<(), HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Self::enabled(provider.close())
        })
    }

    fn wait_for_input_idle(&self, milliseconds: i32) -> Result<i32, HResult> {
        self.invoke_sync_provider(move |provider: &Rc<dyn IWindowProvider>| {
            Ok(i32::from(provider.wait_for_input_idle(milliseconds)))
        })
    }

    fn get_maximizable(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Ok(i32::from(provider.maximizable()))
        })
    }

    fn get_minimizable(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Ok(i32::from(provider.minimizable()))
        })
    }

    fn get_is_modal(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Self::enabled(provider.is_modal()).map(i32::from)
        })
    }

    fn get_visual_state(&self) -> Result<WindowVisualState, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Ok(match provider.visual_state() {
                Visual::Normal => WindowVisualState::Normal,
                Visual::Maximized => WindowVisualState::Maximized,
                Visual::Minimized => WindowVisualState::Minimized,
            })
        })
    }

    fn get_interaction_state(&self) -> Result<WindowInteractionState, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Ok(match Self::enabled(provider.interaction_state())? {
                Interaction::Running => WindowInteractionState::Running,
                Interaction::Closing => WindowInteractionState::Closing,
                Interaction::ReadyForUserInteraction => {
                    WindowInteractionState::ReadyForUserInteraction
                }
                Interaction::BlockedByModalWindow => WindowInteractionState::BlockedByModalWindow,
                Interaction::NotResponding => WindowInteractionState::NotResponding,
            })
        })
    }

    fn get_is_topmost(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|provider: &Rc<dyn IWindowProvider>| {
            Self::enabled(provider.is_topmost()).map(i32::from)
        })
    }
}
