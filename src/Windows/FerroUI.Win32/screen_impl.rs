//! The screens of the system.

use crate::interop::unmanaged_methods::{
    enum_display_monitors, get_system_metrics, monitor_from_point, monitor_from_rect, monitor_from_window, SystemMetric,
    MONITOR, POINT, RECT,
};
use crate::win_screen::WinScreen;
use ferroui_base::{PixelPoint, PixelRect};
use ferroui_controls::platform::{ITopLevelImpl, Screen, ScreensBase, ScreensBaseImpl, ScreensBaseImplExt};
use std::rc::Rc;

/// The screens implementation of the Windows backend.
pub struct ScreenImpl {
    base: ScreensBase<isize, WinScreen>,
}

impl ScreenImpl {
    /// Creates the screens of the system.
    pub fn new() -> Rc<ScreenImpl> {
        Rc::new(ScreenImpl { base: ScreensBase::new() })
    }

    /// The handles of all monitors of the desktop.
    pub fn get_all_display_monitor_handlers() -> Vec<isize> {
        enum_display_monitors()
    }

    /// The screen of a monitor handle, if it is a monitor of the desktop.
    pub fn screen_from_h_monitor(&self, hmonitor: isize) -> Option<Rc<WinScreen>> {
        self.try_get_screen(&hmonitor)
    }

    /// The screen a window is on; `flags` is a [`MONITOR`] value that says
    /// what to answer for a window that is on no monitor.
    pub fn screen_from_hwnd(&self, hwnd: isize, flags: u32) -> Option<Rc<WinScreen>> {
        let monitor = monitor_from_window(hwnd, flags);

        self.screen_from_h_monitor(monitor)
    }

    fn to_screen(screen: Option<Rc<WinScreen>>) -> Option<Rc<Screen>> {
        screen.map(|screen| (*screen).as_ref().screen().clone())
    }
}

impl ScreensBaseImpl for ScreenImpl {
    type Key = isize;
    type Screen = WinScreen;

    fn screens_base(&self) -> &ScreensBase<isize, WinScreen> {
        &self.base
    }

    fn get_screen_count(&self) -> i32 {
        get_system_metrics(SystemMetric::SM_CMONITORS)
    }

    fn get_all_screen_keys(&self) -> Vec<isize> {
        Self::get_all_display_monitor_handlers()
    }

    fn create_screen_from_key(&self, key: &isize) -> Rc<WinScreen> {
        Rc::new(WinScreen::new(*key))
    }

    fn screen_changed(&self, screen: &Rc<WinScreen>) {
        screen.refresh();
    }

    fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        let handle = top_level.handle()?.handle();
        Self::to_screen(self.screen_from_hwnd(handle, MONITOR::MONITOR_DEFAULTTONULL))
    }

    fn screen_from_point_core(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        let monitor = monitor_from_point(POINT { x: point.x, y: point.y }, MONITOR::MONITOR_DEFAULTTONULL);

        Self::to_screen(self.screen_from_h_monitor(monitor))
    }

    fn screen_from_rect_core(&self, rect: PixelRect) -> Option<Rc<Screen>> {
        let r = RECT::from_pixel_rect(rect);
        let monitor = monitor_from_rect(&r, MONITOR::MONITOR_DEFAULTTONULL);

        Self::to_screen(self.screen_from_h_monitor(monitor))
    }
}
