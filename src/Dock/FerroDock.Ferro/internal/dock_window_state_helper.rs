// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{DockWindowState, MdiWindowState};
use ferroui_controls::WindowState;
/// Original native/floating/managed window state conversions.
pub(crate) struct DockWindowStateHelper;
impl DockWindowStateHelper {
    pub(crate) fn to_dock_window_state(state: WindowState) -> DockWindowState {
        match state {
            WindowState::Minimized => DockWindowState::Minimized,
            WindowState::Maximized => DockWindowState::Maximized,
            WindowState::FullScreen => DockWindowState::FullScreen,
            _ => DockWindowState::Normal,
        }
    }
    pub(crate) fn to_ferro_window_state(state: DockWindowState) -> WindowState {
        match state {
            DockWindowState::Minimized => WindowState::Minimized,
            DockWindowState::Maximized => WindowState::Maximized,
            DockWindowState::FullScreen => WindowState::FullScreen,
            _ => WindowState::Normal,
        }
    }
    pub(crate) fn to_dock_window_state_from_mdi(state: MdiWindowState) -> DockWindowState {
        match state {
            MdiWindowState::Minimized => DockWindowState::Minimized,
            MdiWindowState::Maximized => DockWindowState::Maximized,
            _ => DockWindowState::Normal,
        }
    }
    pub(crate) fn to_mdi_window_state(state: DockWindowState) -> MdiWindowState {
        match state {
            DockWindowState::Minimized => MdiWindowState::Minimized,
            DockWindowState::Maximized => MdiWindowState::Maximized,
            _ => MdiWindowState::Normal,
        }
    }
}
