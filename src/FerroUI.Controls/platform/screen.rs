use super::IPlatformHandle;
use ferroui_base::PixelRect;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Describes the orientation of a screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScreenOrientation {
    /// No screen orientation is specified.
    #[default]
    None = 0,

    /// Specifies that the monitor is oriented in landscape mode where the
    /// width of the screen viewing area is greater than the height.
    Landscape = 1,

    /// Specifies that the monitor rotated 90 degrees in the clockwise
    /// direction to orient the screen in portrait mode where the height of
    /// the screen viewing area is greater than the width.
    Portrait = 2,

    /// Specifies that the monitor rotated another 90 degrees in the
    /// clockwise direction (to equal 180 degrees) to orient the screen in
    /// landscape mode where the width of the screen viewing area is greater
    /// than the height. This landscape mode is flipped 180 degrees from the
    /// landscape mode.
    LandscapeFlipped = 4,

    /// Specifies that the monitor rotated another 90 degrees in the
    /// clockwise direction (to equal 270 degrees) to orient the screen in
    /// portrait mode where the height of the screen viewing area is greater
    /// than the width. This portrait mode is flipped 180 degrees from the
    /// portrait mode.
    PortraitFlipped = 8,
}

/// Represents a single display screen.
///
/// Screens are shared (`Rc<Screen>`) and updated in place by the platform
/// backend that owns them; the setters are meant for backends only.
///
/// Two screens are equal when both have a platform handle and the handles
/// are equal; a screen without a platform handle is only equal to itself.
pub struct Screen {
    display_name: RefCell<Option<String>>,
    current_orientation: Cell<ScreenOrientation>,
    scaling: Cell<f64>,
    bounds: Cell<PixelRect>,
    working_area: Cell<PixelRect>,
    is_primary: Cell<bool>,
    platform_handle: Option<Rc<dyn IPlatformHandle>>,
}

impl Screen {
    /// Creates a screen with a scaling of 1 and otherwise empty properties.
    pub fn new(platform_handle: Option<Rc<dyn IPlatformHandle>>) -> Self {
        Self {
            display_name: RefCell::new(None),
            current_orientation: Cell::new(ScreenOrientation::None),
            scaling: Cell::new(1.0),
            bounds: Cell::new(PixelRect::default()),
            working_area: Cell::new(PixelRect::default()),
            is_primary: Cell::new(false),
            platform_handle,
        }
    }

    /// Gets the device name associated with a display.
    pub fn display_name(&self) -> Option<String> {
        self.display_name.borrow().clone()
    }

    /// Sets the device name associated with a display.
    pub fn set_display_name(&self, value: Option<String>) {
        *self.display_name.borrow_mut() = value;
    }

    /// Gets the current orientation of a screen.
    pub fn current_orientation(&self) -> ScreenOrientation {
        self.current_orientation.get()
    }

    /// Sets the current orientation of a screen.
    pub fn set_current_orientation(&self, value: ScreenOrientation) {
        self.current_orientation.set(value);
    }

    /// Gets the scaling factor applied to the screen by the operating
    /// system.
    ///
    /// Multiply this value by 100 to get a percentage. Both X and Y scaling
    /// factors are assumed uniform.
    pub fn scaling(&self) -> f64 {
        self.scaling.get()
    }

    /// Sets the scaling factor applied to the screen by the operating
    /// system.
    pub fn set_scaling(&self, value: f64) {
        self.scaling.set(value);
    }

    /// Gets the overall pixel-size and position of the screen.
    ///
    /// This generally is the raw pixel counts in both the X and Y
    /// direction.
    pub fn bounds(&self) -> PixelRect {
        self.bounds.get()
    }

    /// Sets the overall pixel-size and position of the screen.
    pub fn set_bounds(&self, value: PixelRect) {
        self.bounds.set(value);
    }

    /// Gets the actual working-area pixel-size of the screen.
    ///
    /// This area may be smaller than [`bounds`](Self::bounds) to account
    /// for notches and other block-out areas such as taskbars etc.
    pub fn working_area(&self) -> PixelRect {
        self.working_area.get()
    }

    /// Sets the actual working-area pixel-size of the screen.
    pub fn set_working_area(&self, value: PixelRect) {
        self.working_area.set(value);
    }

    /// Gets a value indicating whether the screen is the primary one.
    pub fn is_primary(&self) -> bool {
        self.is_primary.get()
    }

    /// Sets a value indicating whether the screen is the primary one.
    pub fn set_is_primary(&self, value: bool) {
        self.is_primary.set(value);
    }

    /// Tries to get the platform handle for the screen.
    ///
    /// Returns a handle describing the screen, or `None` if the handle
    /// could not be retrieved.
    pub fn try_get_platform_handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        self.platform_handle.clone()
    }

    /// When a screen is removed, all of its properties are at least
    /// emptied.
    pub fn on_removed(&self) {
        self.set_display_name(None);
        self.bounds.set(PixelRect::default());
        self.working_area.set(PixelRect::default());
        self.scaling.set(0.0);
        self.current_orientation.set(ScreenOrientation::None);
    }
}

impl PartialEq for Screen {
    fn eq(&self, other: &Screen) -> bool {
        match (&self.platform_handle, &other.platform_handle) {
            (Some(handle), Some(other_handle)) => handle.equals(other_handle.as_ref()),
            _ => std::ptr::eq(self, other),
        }
    }
}

impl Eq for Screen {}

impl Hash for Screen {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.platform_handle {
            Some(handle) => {
                handle.handle().hash(state);
                handle.handle_descriptor().hash(state);
            }
            None => std::ptr::hash(self, state),
        }
    }
}

impl fmt::Display for Screen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Screen { ")?;

        // Only printing properties that are supposed to be immutable:
        write!(f, "DisplayName = {}", self.display_name.borrow().as_deref().unwrap_or(""))?;
        if let Some(platform_handle) = &self.platform_handle {
            write!(f, ", {}: {}", platform_handle.handle_descriptor().unwrap_or(""), platform_handle.handle())?;
        }

        f.write_str(" } ")
    }
}

impl fmt::Debug for Screen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Screen")
            .field("display_name", &self.display_name.borrow())
            .field("current_orientation", &self.current_orientation.get())
            .field("scaling", &self.scaling.get())
            .field("bounds", &self.bounds.get())
            .field("working_area", &self.working_area.get())
            .field("is_primary", &self.is_primary.get())
            .field("platform_handle", &self.platform_handle.as_ref().map(|h| (h.handle(), h.handle_descriptor())))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformHandle;

    fn screen(handle: isize) -> Screen {
        Screen::new(Some(Rc::new(PlatformHandle::new(handle, Some("TestHandle")))))
    }

    #[test]
    fn new_screen_has_a_scaling_of_one() {
        let screen = Screen::new(None);
        assert_eq!(screen.scaling(), 1.0);
        assert_eq!(screen.display_name(), None);
        assert_eq!(screen.current_orientation(), ScreenOrientation::None);
        assert_eq!(screen.bounds(), PixelRect::default());
        assert_eq!(screen.working_area(), PixelRect::default());
        assert!(!screen.is_primary());
        assert!(screen.try_get_platform_handle().is_none());
    }

    #[test]
    fn display_writes_name_and_platform_handle() {
        let s = screen(3);
        s.set_display_name(Some("Main".to_string()));
        assert_eq!(s.to_string(), "Screen { DisplayName = Main, TestHandle: 3 } ");
        assert_eq!(Screen::new(None).to_string(), "Screen { DisplayName =  } ");
    }

    #[test]
    fn screens_compare_by_platform_handle() {
        let a = screen(1);
        assert!(a == screen(1));
        assert!(a != screen(2));

        let without_handle = Screen::new(None);
        assert!(without_handle == without_handle);
        assert!(without_handle != Screen::new(None));
        assert!(a != without_handle);
    }

    #[test]
    fn on_removed_empties_the_properties() {
        let s = screen(1);
        s.set_display_name(Some("Main".to_string()));
        s.set_bounds(PixelRect::new(0, 0, 100, 100));
        s.set_working_area(PixelRect::new(0, 10, 100, 90));
        s.set_scaling(2.0);
        s.set_current_orientation(ScreenOrientation::Landscape);
        s.set_is_primary(true);

        s.on_removed();

        assert_eq!(s.display_name(), None);
        assert_eq!(s.bounds(), PixelRect::default());
        assert_eq!(s.working_area(), PixelRect::default());
        assert_eq!(s.scaling(), 0.0);
        assert_eq!(s.current_orientation(), ScreenOrientation::None);
        assert!(s.is_primary());
    }

    #[test]
    fn orientation_values_match_the_reference_values() {
        assert_eq!(ScreenOrientation::None as i32, 0);
        assert_eq!(ScreenOrientation::Landscape as i32, 1);
        assert_eq!(ScreenOrientation::Portrait as i32, 2);
        assert_eq!(ScreenOrientation::LandscapeFlipped as i32, 4);
        assert_eq!(ScreenOrientation::PortraitFlipped as i32, 8);
    }
}
