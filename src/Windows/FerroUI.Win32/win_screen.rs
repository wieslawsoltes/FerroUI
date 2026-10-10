//! A screen of the system, identified by its monitor handle.

use crate::interop::unmanaged_methods::{
    enum_current_display_settings, get_dpi_for_monitor, get_monitor_info, has_get_dpi_for_monitor,
    screen_dpi_from_device_caps, MONITOR_DPI_TYPE,
};
use crate::win32_type_extensions::Win32TypeExtensions;
use ferroui_controls::platform::{PlatformHandle, PlatformScreen, ScreenOrientation};
use std::cell::Cell;
use std::rc::Rc;

/// A screen of the Windows backend.
pub struct WinScreen {
    base: PlatformScreen,
    h_monitor: isize,
    frequency: Cell<i32>,
}

impl AsRef<PlatformScreen> for WinScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

/// The orientation of a display from its `DMDO_*` value.
pub(crate) fn orientation_from_display_orientation(orientation: u32) -> ScreenOrientation {
    match orientation {
        0 => ScreenOrientation::Landscape,
        1 => ScreenOrientation::Portrait,
        2 => ScreenOrientation::LandscapeFlipped,
        3 => ScreenOrientation::PortraitFlipped,
        _ => ScreenOrientation::None,
    }
}

impl WinScreen {
    /// Creates the screen of a monitor handle.
    pub fn new(h_monitor: isize) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(PlatformHandle::new(h_monitor, Some("HMonitor")))),
            h_monitor,
            frequency: Cell::new(0),
        }
    }

    /// The refresh rate of the screen, in hertz.
    pub(crate) fn frequency(&self) -> i32 {
        self.frequency.get()
    }

    /// Reads the state of the screen from the system.
    pub fn refresh(&self) {
        let info = get_monitor_info(self.h_monitor).unwrap_or_default();

        self.base.set_is_primary(info.dw_flags == 1);
        self.base.set_bounds(info.rc_monitor.to_pixel_rect());
        self.base.set_working_area(info.rc_work.to_pixel_rect());
        self.base.set_scaling(self.get_scaling());
        if self.base.display_name().is_none() {
            self.base.set_display_name(self.get_display_name(&info.sz_device));
        }

        let (frequency, orientation) = enum_current_display_settings(&info.sz_device).unwrap_or((0, u32::MAX));

        self.frequency.set(frequency as i32);
        self.base.set_current_orientation(orientation_from_display_orientation(orientation));
    }

    /// The name of the display: the device name of the monitor.
    ///
    /// The reference first asks the display configuration of the system for
    /// the friendly name of the monitor (`QueryDisplayConfig`) and falls
    /// back to the device name; that query is stage 2 of this backend
    /// (docs/porting/win32-platform.md), so the name is the fallback of the
    /// reference: `\\.\DISPLAY1`.
    fn get_display_name(&self, device_name: &str) -> Option<String> {
        Some(device_name.to_string())
    }

    fn get_scaling(&self) -> f64 {
        let dpi = if has_get_dpi_for_monitor() {
            get_dpi_for_monitor(self.h_monitor, MONITOR_DPI_TYPE::MDT_EFFECTIVE_DPI).map_or(0.0, |(x, _)| f64::from(x))
        } else {
            screen_dpi_from_device_caps()
        };

        dpi / 96.0
    }
}
