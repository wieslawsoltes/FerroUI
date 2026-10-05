//! The screens of the system.

use crate::frn_string::frn_string_to_string;
use crate::helpers::*;
use crate::interop::*;
use crate::top_level_impl::top_level_of;
use ferroui_controls::platform::{
    ITopLevelImpl, PlatformHandle, PlatformScreen, Screen, ScreenOrientation, ScreensBase, ScreensBaseImpl,
    ScreensBaseImplExt,
};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The orientation of a native screen; `None` for a value the toolkit does
/// not know.
fn to_screen_orientation(orientation: FrnScreenOrientation) -> Option<ScreenOrientation> {
    Some(match orientation.0 {
        0 => ScreenOrientation::None,
        1 => ScreenOrientation::Landscape,
        2 => ScreenOrientation::Portrait,
        3 => ScreenOrientation::LandscapeFlipped,
        4 => ScreenOrientation::PortraitFlipped,
        _ => return None,
    })
}

/// A screen identified by its `CGDirectDisplayID`.
pub struct NativeScreen {
    base: PlatformScreen,
    display_id: u32,
}

impl NativeScreen {
    fn new(display_id: u32) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(PlatformHandle::new(display_id as isize, Some("CGDirectDisplayID")))),
            display_id,
        }
    }

    fn refresh(&self, native: &IFrnScreens) {
        let mut localized_name: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: `localized_name` is a valid out pointer; the native side
        // stores null or a string with one reference owned by the caller.
        let screen = unsafe { native.get_screen(self.display_id, &mut localized_name) }.check();

        self.base.set_is_primary(screen.is_primary);
        self.base.set_scaling(screen.scaling as f64);
        self.base.set_bounds(to_ferro_pixel_rect(screen.bounds));
        self.base.set_working_area(to_ferro_pixel_rect(screen.working_area));
        self.base.set_current_orientation(match to_screen_orientation(screen.orientation) {
            Some(orientation) => orientation,
            None => panic!("Specified argument was out of the range of valid values: {:?}", screen.orientation),
        });

        // SAFETY: see above; the reference is released when `name` drops.
        let name = unsafe { ComPtr::from_raw(localized_name as *mut IFrnString) };
        self.base.set_display_name(name.and_then(|name| frn_string_to_string(&name)));
    }
}

impl AsRef<PlatformScreen> for NativeScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

/// The screens implementation of the macOS backend.
pub struct ScreenImpl {
    base: ScreensBase<u32, NativeScreen>,
    native: RefCell<Option<ComPtr<IFrnScreens>>>,
}

struct ScreenEvents(Weak<ScreenImpl>);

impl IFrnScreenEventsImpl for ScreenEvents {
    fn on_changed(&self) -> Result<(), HResult> {
        crate::callback_base::guard(Ok(()), || {
            if let Some(screen_impl) = self.0.upgrade() {
                screen_impl.on_changed();
            }
            Ok(())
        })
    }
}

impl ScreenImpl {
    /// Creates the screens; `factory` creates the native screens object for
    /// the events callback it is given.
    pub fn new(factory: impl FnOnce(&IFrnScreenEvents) -> Option<ComPtr<IFrnScreens>>) -> Rc<ScreenImpl> {
        Rc::new_cyclic(|weak_self| {
            let events = IFrnScreenEvents::from_impl(ScreenEvents(weak_self.clone()));
            ScreenImpl { base: ScreensBase::new(), native: RefCell::new(factory(&events)) }
        })
    }

    #[track_caller]
    fn native(&self) -> ComPtr<IFrnScreens> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: the native screens"),
        }
    }

    /// Releases the native screens object.
    pub fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

impl ScreensBaseImpl for ScreenImpl {
    type Key = u32;
    type Screen = NativeScreen;

    fn screens_base(&self) -> &ScreensBase<u32, NativeScreen> {
        &self.base
    }

    fn get_screen_count(&self) -> i32 {
        // SAFETY: a null pointer asks for the count only.
        unsafe { self.native().get_screen_ids(std::ptr::null_mut()) }.check()
    }

    fn get_all_screen_keys(&self) -> Vec<u32> {
        let native = self.native();
        // SAFETY: a null pointer asks for the count only.
        let screen_count = unsafe { native.get_screen_ids(std::ptr::null_mut()) }.check();
        let mut display_ids = vec![0u32; screen_count.max(0) as usize];
        if !display_ids.is_empty() {
            // SAFETY: the buffer holds as many ids as the native side just
            // reported; it writes at most that many.
            unsafe { native.get_screen_ids(display_ids.as_mut_ptr()) }.check();
        }

        display_ids
    }

    fn create_screen_from_key(&self, key: &u32) -> Rc<NativeScreen> {
        Rc::new(NativeScreen::new(*key))
    }

    fn screen_changed(&self, screen: &Rc<NativeScreen>) {
        screen.refresh(&self.native());
    }

    fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        let Some(top_level) = top_level_of(top_level) else {
            panic!("The top-level belongs to a different windowing platform.");
        };
        let display_id = top_level.native()?.get_current_display_id().check();
        let screen = self.try_get_screen(&display_id)?;
        Some((*screen).as_ref().screen().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientations() {
        assert_eq!(to_screen_orientation(FrnScreenOrientation::UnknownOrientation), Some(ScreenOrientation::None));
        assert_eq!(to_screen_orientation(FrnScreenOrientation::Landscape), Some(ScreenOrientation::Landscape));
        assert_eq!(to_screen_orientation(FrnScreenOrientation::Portrait), Some(ScreenOrientation::Portrait));
        assert_eq!(
            to_screen_orientation(FrnScreenOrientation::LandscapeFlipped),
            Some(ScreenOrientation::LandscapeFlipped)
        );
        assert_eq!(
            to_screen_orientation(FrnScreenOrientation::PortraitFlipped),
            Some(ScreenOrientation::PortraitFlipped)
        );
        assert_eq!(to_screen_orientation(FrnScreenOrientation(5)), None);
    }
}
