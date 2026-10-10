//! A screen of the system, identified by its monitor handle.

use crate::interop::unmanaged_methods::{
    display_config_source_name, display_config_target_name, enum_current_display_settings, get_dpi_for_monitor,
    get_monitor_info, has_get_dpi_for_monitor, query_active_display_paths,
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

/// The name of the display of a device among the active paths of the
/// display configuration: the friendly name of the monitor of the first
/// path whose source is the device. The walk ends at a path whose source
/// or whose monitor the system does not name; the name is then the device
/// name, as it is when no path has the device as its source.
pub(crate) fn display_name_from_paths<P>(
    device_name: &str,
    paths: &[P],
    source_name: impl Fn(&P) -> Option<String>,
    target_name: impl Fn(&P) -> Option<String>,
) -> Option<String> {
    for path in paths {
        let Some(source) = source_name(path) else { break };

        if source != device_name {
            continue;
        }

        let Some(target) = target_name(path) else { break };

        return Some(target);
    }

    // Fallback to MONITORINFOEX - \\DISPLAY1.
    Some(device_name.to_string())
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

    /// The name of the display: the friendly name of the monitor from the
    /// display configuration of the system, or the device name of the
    /// monitor.
    fn get_display_name(&self, device_name: &str) -> Option<String> {
        if crate::win32_platform::Win32Platform::windows_version() >= crate::platform_constants::PlatformConstants::WINDOWS7 {
            let paths = query_active_display_paths()?;

            return display_name_from_paths(device_name, &paths, display_config_source_name, display_config_target_name);
        }

        // Fallback to MONITORINFOEX - \\DISPLAY1.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(pairs: &[(Option<&str>, Option<&str>)], device: &str) -> Option<String> {
        let paths: Vec<usize> = (0..pairs.len()).collect();
        display_name_from_paths(
            device,
            &paths,
            |&index| pairs[index].0.map(str::to_string),
            |&index| pairs[index].1.map(str::to_string),
        )
    }

    #[test]
    fn the_name_is_the_monitor_of_the_path_whose_source_is_the_device() {
        let pairs = [(Some(r"\\.\DISPLAY1"), Some("First")), (Some(r"\\.\DISPLAY2"), Some("Second"))];
        assert_eq!(names(&pairs, r"\\.\DISPLAY2").as_deref(), Some("Second"));
        assert_eq!(names(&pairs, r"\\.\DISPLAY1").as_deref(), Some("First"));
        // A monitor without a friendly name has the empty name.
        assert_eq!(names(&[(Some(r"\\.\DISPLAY1"), Some(""))], r"\\.\DISPLAY1").as_deref(), Some(""));
    }

    #[test]
    fn the_name_is_the_device_name_when_no_path_has_the_device_or_the_walk_ends() {
        let pairs = [(Some(r"\\.\DISPLAY1"), Some("First"))];
        assert_eq!(names(&pairs, r"\\.\DISPLAY3").as_deref(), Some(r"\\.\DISPLAY3"));
        assert_eq!(names(&[], r"\\.\DISPLAY1").as_deref(), Some(r"\\.\DISPLAY1"));
        // A source the system does not name ends the walk before the path
        // of the device is reached.
        let pairs = [(None, Some("First")), (Some(r"\\.\DISPLAY2"), Some("Second"))];
        assert_eq!(names(&pairs, r"\\.\DISPLAY2").as_deref(), Some(r"\\.\DISPLAY2"));
        // So does a monitor the system does not name.
        let pairs = [(Some(r"\\.\DISPLAY2"), None), (Some(r"\\.\DISPLAY2"), Some("Second"))];
        assert_eq!(names(&pairs, r"\\.\DISPLAY2").as_deref(), Some(r"\\.\DISPLAY2"));
    }

    /// Against the system: the active paths of the display configuration
    /// are read, and every source the system names is a GDI device.
    #[test]
    fn the_system_names_the_sources_of_its_active_paths() {
        let Some(paths) = query_active_display_paths() else {
            // A session without a display configuration (a service).
            println!("the system has no display configuration to query");
            return;
        };
        for path in &paths {
            let source = display_config_source_name(path);
            let target = display_config_target_name(path);
            println!("path {path:?}: source {source:?}, monitor {target:?}");
            if let Some(source) = source {
                assert!(source.starts_with(r"\\.\DISPLAY"), "{source:?}");
            }
        }
    }
}
