//! The icon of a window: the data of an icon file (or of an image), from
//! which an icon of the system is made for each size that is asked for.

use ferroui_base::PixelSize;

/// The size of an icon of `base_size` pixels at 96 DPI on a display with
/// the scale factor.
pub(crate) fn get_scaled_size(base_size: i32, factor: f64) -> PixelSize {
    let scaled = (f64::from(base_size) * factor).ceil() as i32;
    PixelSize::new(scaled, scaled)
}

/// The size of a taskbar icon at 96 DPI: 24 pixels since Windows 10, 32
/// before.
pub(crate) fn taskbar_icon_size(is_windows10_or_later: bool) -> i32 {
    if is_windows10_or_later {
        24
    } else {
        32
    }
}

#[cfg(windows)]
pub use imp::IconImpl;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::win32_icon::Win32Icon;
    use crate::platform_constants::PlatformConstants;
    use crate::win32_platform::Win32Platform;
    use ferroui_controls::platform::IWindowIconImpl;
    use std::cell::RefCell;
    use std::io;
    use std::rc::{Rc, Weak};
    use std::sync::Arc;

    thread_local! {
        // The icons of the thread that are alive. The reference casts the
        // icon a window is given to this class; the contract of an icon has
        // no way to ask for its type, so a window finds the icon it was
        // given here, by its address.
        static ICONS: RefCell<Vec<Weak<IconImpl>>> = const { RefCell::new(Vec::new()) };
    }

    /// The icon of a window.
    pub struct IconImpl {
        small_icon: Win32Icon,
        big_icon: Win32Icon,
    }

    impl IconImpl {
        /// An icon from the data of a stream, for the small and for the
        /// big icon.
        pub fn new(icon: &mut dyn io::Read) -> io::Result<Rc<IconImpl>> {
            // The reference holds one icon object for both; an icon of the
            // port owns its handle, so the two are made from the same data.
            let icon_data = Self::read_icon_data(icon)?;
            Ok(Self::register(IconImpl {
                small_icon: Win32Icon::from_data(icon_data.clone(), PixelSize::default())?,
                big_icon: Win32Icon::from_data(icon_data, PixelSize::default())?,
            }))
        }

        /// An icon from the data of two streams.
        pub fn with_small_and_big(small_icon: &mut dyn io::Read, big_icon: &mut dyn io::Read) -> io::Result<Rc<IconImpl>> {
            Ok(Self::register(IconImpl {
                small_icon: Win32Icon::from_data(Self::read_icon_data(small_icon)?, PixelSize::default())?,
                big_icon: Win32Icon::from_data(Self::read_icon_data(big_icon)?, PixelSize::default())?,
            }))
        }

        fn register(icon: IconImpl) -> Rc<IconImpl> {
            let icon = Rc::new(icon);
            ICONS.with(|icons| {
                let mut icons = icons.borrow_mut();
                icons.retain(|known| known.strong_count() > 0);
                icons.push(Rc::downgrade(&icon));
            });
            icon
        }

        /// The icon of this backend behind an icon of the contract: the
        /// object itself when the backend created it, and otherwise an icon
        /// made from what the icon saves (the reference fails for an icon
        /// of another class).
        pub(crate) fn from_window_icon(icon: &Rc<dyn IWindowIconImpl>) -> io::Result<Rc<IconImpl>> {
            let address = Rc::as_ptr(icon).cast::<()>();
            let known = ICONS.with(|icons| {
                icons.borrow().iter().find(|known| known.as_ptr().cast::<()>() == address).and_then(Weak::upgrade)
            });
            if let Some(known) = known {
                return Ok(known);
            }
            let mut data = Vec::new();
            icon.save(&mut data)?;
            Self::new(&mut io::Cursor::new(data))
        }

        fn read_icon_data(stream: &mut dyn io::Read) -> io::Result<Arc<[u8]>> {
            let mut icon_data = Vec::new();
            stream.read_to_end(&mut icon_data)?;
            Ok(icon_data.into())
        }

        fn taskbar_icon_size() -> i32 {
            taskbar_icon_size(Win32Platform::windows_version() >= PlatformConstants::WINDOWS10)
        }

        // GetSystemMetrics returns values scaled for the primary monitor, as of the time at which the process started.
        // This is no good for a per-monitor DPI aware application. GetSystemMetricsForDpi would solve the problem,
        // but is only available in Windows 10 version 1607 and later. So instead, we just hard-code the 96dpi icon sizes.

        pub(crate) fn load_small_icon(&self, scale_factor: f64) -> io::Result<Win32Icon> {
            Win32Icon::from_icon(&self.small_icon, get_scaled_size(16, scale_factor))
        }

        pub(crate) fn load_big_icon(&self, scale_factor: f64) -> io::Result<Win32Icon> {
            let taskbar_icon_size = Self::taskbar_icon_size();
            let target_size = get_scaled_size(taskbar_icon_size, scale_factor);
            let mut icon = Win32Icon::from_icon(&self.big_icon, target_size)?;

            // The exact size of a taskbar icon in Windows 10 and later is 24px @ 96dpi. But if an ICO file doesn't have
            // that size, 16px can be selected instead. If this happens, fall back to a 32 pixel icon. Windows will downscale it.
            if taskbar_icon_size == 24 && icon.size().width < target_size.width {
                icon.dispose();
                icon = Win32Icon::from_icon(&self.big_icon, get_scaled_size(32, scale_factor))?;
            }

            Ok(icon)
        }

        /// Releases the two icons.
        pub fn dispose(&self) {
            self.small_icon.dispose();
            self.big_icon.dispose();
        }
    }

    impl IWindowIconImpl for IconImpl {
        fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
            self.big_icon.copy_to(output_stream)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_icon_size_is_scaled_up_to_whole_pixels() {
        assert_eq!(get_scaled_size(16, 1.0), PixelSize::new(16, 16));
        assert_eq!(get_scaled_size(16, 1.25), PixelSize::new(20, 20));
        assert_eq!(get_scaled_size(24, 1.5), PixelSize::new(36, 36));
        assert_eq!(get_scaled_size(24, 1.1), PixelSize::new(27, 27));
        assert_eq!(get_scaled_size(32, 2.0), PixelSize::new(64, 64));
    }

    #[test]
    fn the_taskbar_icon_is_smaller_since_windows_10() {
        assert_eq!(taskbar_icon_size(true), 24);
        assert_eq!(taskbar_icon_size(false), 32);
    }
}
