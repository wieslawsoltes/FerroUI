use crate::platform::{IScreenImpl, ITopLevelImpl, IWindowBaseImpl, Screen, ScreenHelper};
use ferroui_base::{PixelPoint, PixelRect};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Creates a screen with the given properties.
pub fn mock_screen(scaling: f64, bounds: PixelRect, working_area: PixelRect, is_primary: bool) -> Rc<Screen> {
    let screen = Screen::new(None);
    screen.set_scaling(scaling);
    screen.set_bounds(bounds);
    screen.set_working_area(working_area);
    screen.set_is_primary(is_primary);
    Rc::new(screen)
}

/// A screens implementation over a fixed list of screens.
pub struct MockScreenImpl {
    screens: RefCell<Vec<Rc<Screen>>>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    resolves_screens: Cell<bool>,
}

impl MockScreenImpl {
    pub fn new(screens: Vec<Rc<Screen>>) -> Rc<MockScreenImpl> {
        Rc::new(MockScreenImpl { screens: RefCell::new(screens), changed: RefCell::new(None), resolves_screens: Cell::new(false) })
    }

    /// Makes the `screen_from_*` queries answer from the list of screens.
    /// By default they find nothing, like the queries of a mock that were
    /// not set up.
    pub fn set_resolves_screens(&self, value: bool) {
        self.resolves_screens.set(value);
    }

    /// Replaces the screens and raises the changed notification.
    pub fn set_screens(&self, screens: Vec<Rc<Screen>>) {
        *self.screens.borrow_mut() = screens;
        let changed = self.changed.borrow().clone();
        if let Some(changed) = changed {
            changed();
        }
    }
}

impl IScreenImpl for MockScreenImpl {
    fn screen_count(&self) -> i32 {
        self.screens.borrow().len() as i32
    }

    fn all_screens(&self) -> Vec<Rc<Screen>> {
        self.screens.borrow().clone()
    }

    fn changed(&self) -> Option<Rc<dyn Fn()>> {
        self.changed.borrow().clone()
    }

    fn set_changed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.changed.borrow_mut() = value;
    }

    fn screen_from_window(&self, window: &dyn IWindowBaseImpl) -> Option<Rc<Screen>> {
        if !self.resolves_screens.get() {
            return None;
        }
        ScreenHelper::screen_from_window(window, &self.all_screens())
    }

    fn screen_from_top_level(&self, _top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        None
    }

    fn screen_from_point(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        if !self.resolves_screens.get() {
            return None;
        }
        ScreenHelper::screen_from_point(point, &self.all_screens())
    }

    fn screen_from_rect(&self, rect: PixelRect) -> Option<Rc<Screen>> {
        if !self.resolves_screens.get() {
            return None;
        }
        ScreenHelper::screen_from_rect(rect, &self.all_screens())
    }

    fn request_screen_details(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        Box::pin(std::future::ready(true))
    }
}
