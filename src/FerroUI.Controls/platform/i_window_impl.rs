use super::{IWindowBaseImpl, IWindowIconImpl, PlatformAllowedWindowActions, PlatformRequestedDrawnDecoration};
use crate::{WindowCloseReason, WindowDecorations, WindowEdge, WindowResizeReason, WindowState};
use ferroui_base::input::PointerPressedEventArgs;
use ferroui_base::{PixelPoint, Size, Thickness};
use std::rc::Rc;

/// Defines a platform-specific window implementation.
pub trait IWindowImpl: IWindowBaseImpl {
    /// Gets the minimized/maximized state of the window.
    fn window_state(&self) -> WindowState;

    /// Sets the minimized/maximized state of the window.
    fn set_window_state(&self, value: WindowState);

    /// Indicates if the [`window_state`](Self::window_state) getter is
    /// reliable on this platform.
    fn window_state_getter_is_usable(&self) -> bool;

    /// Gets the method called when the minimized/maximized state of the
    /// window changes.
    fn window_state_changed(&self) -> Option<Rc<dyn Fn(WindowState)>>;

    /// Sets a method called when the minimized/maximized state of the
    /// window changes.
    fn set_window_state_changed(&self, value: Option<Rc<dyn Fn(WindowState)>>);

    /// Sets the title of the window.
    fn set_title(&self, title: Option<&str>);

    /// Sets the parent of the window.
    fn set_parent(&self, parent: Option<Rc<dyn IWindowImpl>>);

    /// Disables the window for example when a modal dialog is open.
    fn set_enabled(&self, enable: bool);

    /// Gets the method called when a disabled window receives input. Can be
    /// used to activate child windows.
    fn got_input_when_disabled(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when a disabled window receives input.
    fn set_got_input_when_disabled(&self, value: Option<Rc<dyn Fn()>>);

    /// Requests the specified window decorations (title bar, borders, etc).
    fn set_window_decorations(&self, enabled: WindowDecorations);

    /// Sets the icon of this window.
    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>);

    /// Enables or disables the taskbar icon.
    fn show_taskbar_icon(&self, value: bool);

    /// Enables or disables resizing of the window.
    fn can_resize(&self, value: bool);

    /// Enables or disables minimizing the window.
    fn set_can_minimize(&self, value: bool);

    /// Enables or disables maximizing the window.
    fn set_can_maximize(&self, value: bool);

    /// Gets the method called before the underlying implementation is
    /// destroyed. Return `true` to prevent the underlying implementation
    /// from closing.
    fn closing(&self) -> Option<Rc<dyn Fn(WindowCloseReason) -> bool>>;

    /// Sets a method called before the underlying implementation is
    /// destroyed.
    fn set_closing(&self, value: Option<Rc<dyn Fn(WindowCloseReason) -> bool>>);

    /// Gets a value to indicate if the platform was able to extend client
    /// area to non-client area.
    fn is_client_area_extended_to_decorations(&self) -> bool;

    /// Gets the method called when the "extend client area to decorations"
    /// state changes.
    fn extend_client_area_to_decorations_changed(&self) -> Option<Rc<dyn Fn(bool)>>;

    /// Sets a method called when the "extend client area to decorations"
    /// state changes.
    fn set_extend_client_area_to_decorations_changed(&self, value: Option<Rc<dyn Fn(bool)>>);

    /// Gets a flag that indicates if managed decorations i.e. caption
    /// buttons are required. This property is used when the client area is
    /// extended to the decorations.
    fn needs_managed_decorations(&self) -> bool;

    /// Gets the decorations the platform asks the toolkit to draw.
    fn requested_drawn_decorations(&self) -> PlatformRequestedDrawnDecoration;

    /// Gets the method called when the requested drawn decorations change.
    fn drawn_decorations_request_changed(&self) -> Option<Rc<dyn Fn()>> {
        None
    }

    /// Sets a method called when the requested drawn decorations change.
    fn set_drawn_decorations_request_changed(&self, _value: Option<Rc<dyn Fn()>>) {}

    /// Gets a thickness that describes the amount each side of the
    /// non-client area extends into the client area.
    fn extended_margins(&self) -> Thickness;

    /// Gets a thickness that describes the margin around the window that is
    /// offscreen.
    fn off_screen_margin(&self) -> Thickness;

    /// Starts moving a window with left button being held. Should be called
    /// from a left mouse button press event handler.
    fn begin_move_drag(&self, e: &PointerPressedEventArgs);

    /// Starts resizing a window. This function is used if an application
    /// has window resizing controls. Should be called from a left mouse
    /// button press event handler.
    fn begin_resize_drag(&self, edge: WindowEdge, e: &PointerPressedEventArgs);

    /// Sets the client size of the top level.
    ///
    /// `client_size` is the new client size; `reason` is the reason for
    /// the resize (the reference default is
    /// [`WindowResizeReason::Application`]).
    fn resize(&self, client_size: Size, reason: WindowResizeReason);

    /// Sets the position of the window, in device pixels.
    fn move_(&self, point: PixelPoint);

    /// Sets the minimum and maximum size of the window, in
    /// device-independent pixels.
    fn set_min_max_size(&self, min_size: Size, max_size: Size);

    /// Sets if the client area should extend into the non-client area.
    fn set_extend_client_area_to_decorations_hint(&self, extend_into_client_area_hint: bool);

    /// Sets the height of the title bar if the client area is extended;
    /// -1 stands for the system default.
    fn set_extend_client_area_title_bar_height_hint(&self, title_bar_height: f64);

    /// Gets the window actions the platform currently allows.
    fn allowed_window_actions(&self) -> PlatformAllowedWindowActions {
        PlatformAllowedWindowActions::ALL
    }

    /// Gets the method called when the allowed window actions change.
    fn allowed_window_actions_changed(&self) -> Option<Rc<dyn Fn(PlatformAllowedWindowActions)>> {
        None
    }

    /// Sets a method called when the allowed window actions change.
    fn set_allowed_window_actions_changed(&self, _value: Option<Rc<dyn Fn(PlatformAllowedWindowActions)>>) {}

    /// Sets the extents of the shadow the toolkit draws around the window.
    fn set_shadow_extents(&self, _extents: Thickness) {}
}
