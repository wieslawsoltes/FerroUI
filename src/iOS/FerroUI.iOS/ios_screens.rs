//! The screens of the device.

use ferroui_base::PixelRect;
use ferroui_controls::platform::ScreenOrientation;

/// The values of `UIDeviceOrientation`.
pub mod device_orientation {
    pub const UNKNOWN: isize = 0;
    pub const PORTRAIT: isize = 1;
    pub const PORTRAIT_UPSIDE_DOWN: isize = 2;
    pub const LANDSCAPE_LEFT: isize = 3;
    pub const LANDSCAPE_RIGHT: isize = 4;
    pub const FACE_UP: isize = 5;
    pub const FACE_DOWN: isize = 6;
}

/// The orientation of a screen for the `UIDeviceOrientation` of its device
/// and its native size in pixels.
pub fn screen_orientation(ui_orientation: isize, native_width: f64, native_height: f64) -> ScreenOrientation {
    match ui_orientation {
        device_orientation::PORTRAIT => ScreenOrientation::Portrait,
        device_orientation::PORTRAIT_UPSIDE_DOWN => ScreenOrientation::PortraitFlipped,
        device_orientation::LANDSCAPE_LEFT => ScreenOrientation::Landscape,
        device_orientation::LANDSCAPE_RIGHT => ScreenOrientation::LandscapeFlipped,
        device_orientation::FACE_UP | device_orientation::FACE_DOWN => {
            if native_width > native_height {
                ScreenOrientation::Landscape
            } else {
                ScreenOrientation::Portrait
            }
        }
        _ => ScreenOrientation::None,
    }
}

/// The bounds of a screen in pixels, from its native bounds (`x`, `y`,
/// `width`, `height`) and the size of its bounds in points.
///
/// The native bounds are "the bounding rectangle of the physical screen,
/// measured in pixels", so they are cast to integers, and "this value does
/// not change as the device rotates", so they are rotated to match the
/// other platforms. As a reference, the bounds in points are always
/// rotated.
pub fn screen_bounds(native_bounds: (f64, f64, f64, f64), scaled_width: f64, scaled_height: f64) -> PixelRect {
    let (x, y, width, height) = native_bounds;
    if scaled_width > scaled_height && width < height {
        PixelRect::new(x as i32, y as i32, height as i32, width as i32)
    } else {
        PixelRect::new(x as i32, y as i32, width as i32, height as i32)
    }
}

#[cfg(target_os = "ios")]
pub use uikit::{IosScreen, IosScreens};

// The reference reads the screens through the interfaces of UIKit that
// predate scenes (the list of screens, the main screen, their notifications).
#[cfg(target_os = "ios")]
#[allow(deprecated)]
mod uikit {
    use super::{device_orientation, screen_bounds, screen_orientation};
    use crate::ferro_view::TopLevelImpl;
    use block2::RcBlock;
    use ferroui_base::{PixelPoint, PixelRect};
    use ferroui_controls::platform::{
        ITopLevelImpl, PlatformHandle, PlatformScreen, Screen, ScreensBase, ScreensBaseImpl, ScreensBaseImplExt,
    };
    use objc2::rc::Retained;
    use objc2::MainThreadMarker;
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSNotificationName};
    use objc2_ui_kit::{
        UIDevice, UIDeviceOrientation, UIDeviceOrientationDidChangeNotification, UIScreen,
        UIScreenDidConnectNotification, UIScreenDidDisconnectNotification, UIScreenModeDidChangeNotification,
    };
    use std::hash::{Hash, Hasher};
    use std::ptr::NonNull;
    use std::rc::{Rc, Weak};

    const _: () = {
        // The constants of this file are those of UIKit.
        assert!(device_orientation::UNKNOWN == UIDeviceOrientation::Unknown.0);
        assert!(device_orientation::PORTRAIT == UIDeviceOrientation::Portrait.0);
        assert!(device_orientation::PORTRAIT_UPSIDE_DOWN == UIDeviceOrientation::PortraitUpsideDown.0);
        assert!(device_orientation::LANDSCAPE_LEFT == UIDeviceOrientation::LandscapeLeft.0);
        assert!(device_orientation::LANDSCAPE_RIGHT == UIDeviceOrientation::LandscapeRight.0);
        assert!(device_orientation::FACE_UP == UIDeviceOrientation::FaceUp.0);
        assert!(device_orientation::FACE_DOWN == UIDeviceOrientation::FaceDown.0);
    };

    fn main_thread() -> MainThreadMarker {
        match MainThreadMarker::new() {
            Some(mtm) => mtm,
            None => panic!("The screens of the device are read on the main thread."),
        }
    }

    /// A screen object of UIKit as the key of a screen: two keys are equal
    /// when they are the same object.
    #[derive(Clone)]
    pub struct ScreenKey(Retained<UIScreen>);

    impl PartialEq for ScreenKey {
        fn eq(&self, other: &Self) -> bool {
            Retained::as_ptr(&self.0) == Retained::as_ptr(&other.0)
        }
    }

    impl Eq for ScreenKey {}

    impl Hash for ScreenKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            Retained::as_ptr(&self.0).hash(state);
        }
    }

    /// A screen of the device.
    pub struct IosScreen {
        base: PlatformScreen,
        screen: Retained<UIScreen>,
    }

    impl IosScreen {
        fn new(screen: Retained<UIScreen>) -> Self {
            let handle = PlatformHandle::new(Retained::as_ptr(&screen) as isize, Some("UIScreen"));
            Self { base: PlatformScreen::new(Rc::new(handle)), screen }
        }

        /// Reads the properties of the screen again.
        pub fn refresh(&self) {
            let mtm = main_thread();
            let is_primary = Retained::as_ptr(&self.screen) == Retained::as_ptr(&UIScreen::mainScreen(mtm));
            self.base.set_is_primary(is_primary);
            self.base.set_scaling(self.screen.nativeScale());
            self.base.set_display_name(is_primary.then(|| "MainScreen".to_string()));

            let native_bounds = self.screen.nativeBounds();
            let scaled_bounds = self.screen.bounds();

            let ui_orientation = if is_primary {
                UIDevice::currentDevice(mtm).orientation()
            } else {
                UIDeviceOrientation::LandscapeLeft
            };
            self.base.set_current_orientation(screen_orientation(
                ui_orientation.0,
                native_bounds.size.width,
                native_bounds.size.height,
            ));

            let bounds: PixelRect = screen_bounds(
                (native_bounds.origin.x, native_bounds.origin.y, native_bounds.size.width, native_bounds.size.height),
                scaled_bounds.size.width,
                scaled_bounds.size.height,
            );
            self.base.set_bounds(bounds);
            self.base.set_working_area(bounds);
        }
    }

    impl AsRef<PlatformScreen> for IosScreen {
        fn as_ref(&self) -> &PlatformScreen {
            &self.base
        }
    }

    /// The screens of the device.
    pub struct IosScreens {
        base: ScreensBase<ScreenKey, IosScreen>,
    }

    impl IosScreens {
        /// Creates the screens and follows their changes.
        pub fn new() -> Rc<Self> {
            let this = Rc::new(Self { base: ScreensBase::new() });

            // SAFETY: the four names are constants of UIKit.
            let names: [&NSNotificationName; 4] = unsafe {
                [
                    UIScreenDidConnectNotification,
                    UIScreenDidDisconnectNotification,
                    UIScreenModeDidChangeNotification,
                    UIDeviceOrientationDidChangeNotification,
                ]
            };
            for name in names {
                Self::observe(name, Rc::downgrade(&this));
            }

            this
        }

        fn observe(name: &NSNotificationName, this: Weak<IosScreens>) {
            let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
                if let Some(this) = this.upgrade() {
                    ScreensBaseImplExt::on_changed(&this);
                }
            });
            // SAFETY: the block takes the one argument of a notification
            // block and ignores it. No queue is given, so the block runs on
            // the thread that posts the notification, which for the
            // notifications of the screens and of the device is the main
            // thread, the thread that made the block.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(name),
                    None,
                    None,
                    &block,
                )
            };
            // The screens live as long as the process, and so does the
            // subscription.
            std::mem::forget(observer);
        }
    }

    impl ScreensBaseImpl for IosScreens {
        type Key = ScreenKey;
        type Screen = IosScreen;

        fn screens_base(&self) -> &ScreensBase<ScreenKey, IosScreen> {
            &self.base
        }

        fn get_all_screen_keys(&self) -> Vec<ScreenKey> {
            UIScreen::screens(main_thread()).iter().map(ScreenKey).collect()
        }

        fn create_screen_from_key(&self, key: &ScreenKey) -> Rc<IosScreen> {
            Rc::new(IosScreen::new(key.0.clone()))
        }

        fn screen_changed(&self, screen: &Rc<IosScreen>) {
            screen.refresh();
        }

        fn screen_from_point_core(&self, _point: PixelPoint) -> Option<Rc<Screen>> {
            None
        }

        fn screen_from_rect_core(&self, _rect: PixelRect) -> Option<Rc<Screen>> {
            None
        }

        fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
            let top_level = top_level.as_any().downcast_ref::<TopLevelImpl>()?;
            let ui_screen = top_level.view()?.window()?.screen();
            let screen = self.try_get_screen(&ScreenKey(ui_screen))?;
            Some(screen.base.screen().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn the_orientation_of_the_device_is_the_orientation_of_its_screen() {
        let orientation = |value| screen_orientation(value, 1179.0, 2556.0);
        assert_eq!(ScreenOrientation::Portrait, orientation(device_orientation::PORTRAIT));
        assert_eq!(ScreenOrientation::PortraitFlipped, orientation(device_orientation::PORTRAIT_UPSIDE_DOWN));
        assert_eq!(ScreenOrientation::Landscape, orientation(device_orientation::LANDSCAPE_LEFT));
        assert_eq!(ScreenOrientation::LandscapeFlipped, orientation(device_orientation::LANDSCAPE_RIGHT));
        assert_eq!(ScreenOrientation::None, orientation(device_orientation::UNKNOWN));
    }

    #[test]
    fn a_device_that_lies_flat_has_the_orientation_of_its_native_size() {
        assert_eq!(ScreenOrientation::Portrait, screen_orientation(device_orientation::FACE_UP, 1179.0, 2556.0));
        assert_eq!(ScreenOrientation::Portrait, screen_orientation(device_orientation::FACE_DOWN, 1179.0, 2556.0));
        assert_eq!(ScreenOrientation::Landscape, screen_orientation(device_orientation::FACE_UP, 2732.0, 2048.0));
    }

    #[test]
    fn the_native_bounds_are_rotated_with_the_bounds_in_points() {
        let native = (0.0, 0.0, 1179.0, 2556.0);
        assert_eq!(PixelRect::new(0, 0, 1179, 2556), screen_bounds(native, 393.0, 852.0));
        assert_eq!(PixelRect::new(0, 0, 2556, 1179), screen_bounds(native, 852.0, 393.0));
        // Native bounds that are wide already are left as they are.
        assert_eq!(PixelRect::new(0, 0, 2732, 2048), screen_bounds((0.0, 0.0, 2732.0, 2048.0), 1366.0, 1024.0));
    }
}
