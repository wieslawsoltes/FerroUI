use crate::interop::screen_helper as browser_screen_helper;
use crate::interop::JsObject;
use crate::js_object_control_handle::JsObjectPlatformHandle;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::{PixelPoint, PixelRect};
use ferroui_controls::platform::{
    IScreenImpl, ITopLevelImpl, PlatformScreen, Screen, ScreenHelper, ScreenOrientation, ScreensBase,
    ScreensBaseImpl, ScreensBaseImplExt,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::rc::{Rc, Weak};

/// The orientation of a value of `ScreenOrientation` of the page.
fn to_screen_orientation(orientation: i32) -> ScreenOrientation {
    match orientation {
        1 => ScreenOrientation::Landscape,
        2 => ScreenOrientation::Portrait,
        4 => ScreenOrientation::LandscapeFlipped,
        8 => ScreenOrientation::PortraitFlipped,
        _ => ScreenOrientation::None,
    }
}

/// A rectangle of the page given as x, y, width and height; empty when the
/// page gives fewer numbers.
fn to_pixel_rect(values: &[f64]) -> PixelRect {
    match values {
        [x, y, width, height, ..] => PixelRect::new(*x as i32, *y as i32, *width as i32, *height as i32),
        _ => PixelRect::default(),
    }
}

/// Identifies a screen of the page by its object.
#[derive(Clone)]
pub struct BrowserScreenKey(JsObject);

impl PartialEq for BrowserScreenKey {
    fn eq(&self, other: &Self) -> bool {
        // The same object of the page (`===`).
        self.0 == other.0
    }
}

impl Eq for BrowserScreenKey {}

impl Hash for BrowserScreenKey {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        // An object of the page has nothing to hash; a page has a handful of screens.
    }
}

/// A screen of the page.
pub struct BrowserScreen {
    base: PlatformScreen,
    screen: JsObject,
    is_current: Cell<bool>,
}

impl BrowserScreen {
    fn new(screen: JsObject) -> Self {
        Self {
            base: PlatformScreen::new(Rc::new(JsObjectPlatformHandle::new(screen.clone()))),
            screen,
            is_current: Cell::new(false),
        }
    }

    /// Whether the window is on the screen.
    pub(crate) fn is_current(&self) -> bool {
        self.is_current.get()
    }

    /// Reads the properties of the screen from the page.
    pub fn refresh(&self) {
        self.is_current.set(browser_screen_helper::is_current(&self.screen));
        self.base.set_display_name(browser_screen_helper::get_display_name(&self.screen));
        self.base.set_scaling(browser_screen_helper::get_scaling(&self.screen));
        self.base.set_is_primary(browser_screen_helper::is_primary(&self.screen));
        self.base.set_current_orientation(to_screen_orientation(browser_screen_helper::get_current_orientation(
            &self.screen,
        )));
        self.base.set_bounds(to_pixel_rect(&browser_screen_helper::get_bounds(&self.screen)));
        self.base.set_working_area(to_pixel_rect(&browser_screen_helper::get_working_area(&self.screen)));
    }
}

impl AsRef<PlatformScreen> for BrowserScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

thread_local! {
    static INSTANCE: RefCell<Option<Rc<BrowserScreens>>> = const { RefCell::new(None) };
}

/// The screens of the page: the screen of the window, and every screen
/// once the page has been given the details of the screens.
pub struct BrowserScreens {
    base: ScreensBase<BrowserScreenKey, BrowserScreen>,
    weak_self: Weak<BrowserScreens>,
    is_extended: Rc<Cell<bool>>,
}

impl BrowserScreens {
    /// The screens of the page, created on first use.
    pub fn instance() -> Rc<BrowserScreens> {
        if let Some(instance) = INSTANCE.with(|instance| instance.borrow().clone()) {
            return instance;
        }

        let instance = Self::new();
        INSTANCE.with(|slot| *slot.borrow_mut() = Some(instance.clone()));
        instance
    }

    fn new() -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            base: ScreensBase::new(),
            weak_self: weak_self.clone(),
            is_extended: Rc::new(Cell::new(false)),
        });

        browser_screen_helper::subscribe_on_changed(&BrowserWindowingPlatform::global_this());
        browser_screen_helper::check_permissions(&BrowserWindowingPlatform::global_this());

        this
    }

    /// The page reported a change of the screens.
    pub(crate) fn on_changed(&self) {
        if let Some(this) = self.weak_self.upgrade() {
            ScreensBaseImplExt::on_changed(&this);
        }
    }
}

impl ScreensBaseImpl for BrowserScreens {
    type Key = BrowserScreenKey;
    type Screen = BrowserScreen;

    fn screens_base(&self) -> &ScreensBase<BrowserScreenKey, BrowserScreen> {
        &self.base
    }

    fn get_all_screen_keys(&self) -> Vec<BrowserScreenKey> {
        browser_screen_helper::get_all_screens(&BrowserWindowingPlatform::global_this())
            .into_iter()
            .map(BrowserScreenKey)
            .collect()
    }

    fn create_screen_from_key(&self, key: &BrowserScreenKey) -> Rc<BrowserScreen> {
        Rc::new(BrowserScreen::new(key.0.clone()))
    }

    fn screen_changed(&self, screen: &Rc<BrowserScreen>) {
        screen.refresh();
    }

    fn screen_from_top_level_core(&self, _top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        self.all_platform_screens().into_iter().find(|screen| screen.is_current()).map(|screen| screen.base.screen().clone())
    }

    fn screen_from_point_core(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        if self.is_extended.get() {
            ScreenHelper::screen_from_point(point, &IScreenImpl::all_screens(self))
        } else {
            None
        }
    }

    fn screen_from_rect_core(&self, rect: PixelRect) -> Option<Rc<Screen>> {
        if self.is_extended.get() {
            ScreenHelper::screen_from_rect(rect, &IScreenImpl::all_screens(self))
        } else {
            None
        }
    }

    fn request_screen_details_core(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        let is_extended = self.is_extended.clone();
        Box::pin(async move {
            match browser_screen_helper::request_detailed_screens(&BrowserWindowingPlatform::global_this()).await {
                Ok(result) => {
                    is_extended.set(result);
                    result
                }
                Err(error) => {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::BROWSER_PLATFORM) {
                        logger.log(None, &format!("Failed to get extended screen details: {error}"));
                    }
                    false
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_orientations_of_the_page_are_the_screen_orientations() {
        assert_eq!(ScreenOrientation::None, to_screen_orientation(0));
        assert_eq!(ScreenOrientation::Landscape, to_screen_orientation(1));
        assert_eq!(ScreenOrientation::Portrait, to_screen_orientation(2));
        assert_eq!(ScreenOrientation::LandscapeFlipped, to_screen_orientation(4));
        assert_eq!(ScreenOrientation::PortraitFlipped, to_screen_orientation(8));
        assert_eq!(ScreenOrientation::None, to_screen_orientation(3));
    }

    #[test]
    fn rectangles_are_read_as_x_y_width_height() {
        assert_eq!(PixelRect::new(10, 20, 1920, 1080), to_pixel_rect(&[10.0, 20.0, 1920.0, 1080.0]));
        assert_eq!(PixelRect::new(-1920, 0, 1920, 1080), to_pixel_rect(&[-1920.5, 0.0, 1920.0, 1080.0]));
        assert_eq!(PixelRect::default(), to_pixel_rect(&[1.0, 2.0]));
    }
}
