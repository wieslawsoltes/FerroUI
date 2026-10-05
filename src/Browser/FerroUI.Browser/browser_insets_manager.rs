use crate::interop::dom_helper;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::media::Color;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Thickness;
use ferroui_controls::platform::{IInsetsManager, InsetsManagerBase, SafeAreaChangedArgs};
use std::rc::Rc;

/// The page reads; replaced by a fake in the tests.
pub(crate) trait IInsetsPage {
    fn is_fullscreen(&self) -> bool;
    fn set_fullscreen(&self, is_fullscreen: bool);
    fn get_safe_area_padding(&self) -> Vec<f64>;
}

struct InsetsPage;

impl IInsetsPage for InsetsPage {
    fn is_fullscreen(&self) -> bool {
        dom_helper::is_fullscreen(&BrowserWindowingPlatform::global_this())
    }

    fn set_fullscreen(&self, is_fullscreen: bool) {
        dom_helper::set_fullscreen(&BrowserWindowingPlatform::global_this(), is_fullscreen);
    }

    fn get_safe_area_padding(&self) -> Vec<f64> {
        dom_helper::get_safe_area_padding(&BrowserWindowingPlatform::global_this())
    }
}

/// The insets of the page: the safe area of the device (the CSS
/// `safe-area-inset-*` environment variables) and the full screen, which
/// hides the system bars.
pub struct BrowserInsetsManager {
    base: InsetsManagerBase,
    page: Box<dyn IInsetsPage>,
}

impl Default for BrowserInsetsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserInsetsManager {
    /// Creates the insets manager of a view.
    pub fn new() -> Self {
        Self::with_page(Box::new(InsetsPage))
    }

    pub(crate) fn with_page(page: Box<dyn IInsetsPage>) -> Self {
        Self { base: InsetsManagerBase::new(), page }
    }

    /// Raises the safe area changed event with the current safe area.
    pub fn notify_safe_area_padding_changed(&self) {
        self.base.on_safe_area_changed(SafeAreaChangedArgs::new(self.safe_area_padding()));
    }
}

impl IInsetsManager for BrowserInsetsManager {
    // As the original, which answers whether the page is in full screen: the inverse of what the
    // setter takes, which hides the bars by requesting full screen.
    fn is_system_bar_visible(&self) -> Option<bool> {
        Some(self.page.is_fullscreen())
    }

    fn set_is_system_bar_visible(&self, value: Option<bool>) {
        self.page.set_fullscreen(!value.unwrap_or(true));
    }

    fn display_edge_to_edge_preference(&self) -> bool {
        self.base.display_edge_to_edge_preference()
    }

    fn set_display_edge_to_edge_preference(&self, value: bool) {
        self.base.set_display_edge_to_edge_preference(value);
    }

    fn displays_edge_to_edge(&self) -> bool {
        self.base.displays_edge_to_edge()
    }

    fn safe_area_padding(&self) -> Thickness {
        match self.page.get_safe_area_padding()[..] {
            [left, top, right, bottom, ..] => Thickness::new(left, top, right, bottom),
            _ => Thickness::default(),
        }
    }

    fn system_bar_color(&self) -> Option<Color> {
        self.base.system_bar_color()
    }

    fn set_system_bar_color(&self, value: Option<Color>) {
        self.base.set_system_bar_color(value);
    }

    fn safe_area_changed(&self, handler: Rc<dyn Fn(&SafeAreaChangedArgs)>) -> Rc<dyn IDisposable> {
        self.base.safe_area_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct PageState {
        fullscreen: Cell<bool>,
        requests: RefCell<Vec<bool>>,
        padding: RefCell<Vec<f64>>,
    }

    struct FakePage(Rc<PageState>);

    impl IInsetsPage for FakePage {
        fn is_fullscreen(&self) -> bool {
            self.0.fullscreen.get()
        }

        fn set_fullscreen(&self, is_fullscreen: bool) {
            self.0.requests.borrow_mut().push(is_fullscreen);
        }

        fn get_safe_area_padding(&self) -> Vec<f64> {
            self.0.padding.borrow().clone()
        }
    }

    fn manager() -> (BrowserInsetsManager, Rc<PageState>) {
        let state = Rc::new(PageState::default());
        (BrowserInsetsManager::with_page(Box::new(FakePage(state.clone()))), state)
    }

    #[test]
    fn the_system_bar_visibility_is_the_full_screen_of_the_page() {
        let (manager, state) = manager();
        assert_eq!(Some(false), manager.is_system_bar_visible());
        state.fullscreen.set(true);
        assert_eq!(Some(true), manager.is_system_bar_visible());
    }

    #[test]
    fn hiding_the_system_bars_requests_the_full_screen() {
        let (manager, state) = manager();
        manager.set_is_system_bar_visible(Some(false));
        manager.set_is_system_bar_visible(Some(true));
        manager.set_is_system_bar_visible(None);
        assert_eq!(vec![true, false, false], *state.requests.borrow());
    }

    #[test]
    fn the_safe_area_is_read_as_left_top_right_bottom() {
        let (manager, state) = manager();
        *state.padding.borrow_mut() = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(Thickness::new(1.0, 2.0, 3.0, 4.0), manager.safe_area_padding());

        state.padding.borrow_mut().clear();
        assert_eq!(Thickness::default(), manager.safe_area_padding());
    }

    #[test]
    fn the_edge_to_edge_preference_and_the_bar_color_are_kept() {
        let (manager, _) = manager();
        assert!(!manager.display_edge_to_edge_preference());
        manager.set_display_edge_to_edge_preference(true);
        assert!(manager.display_edge_to_edge_preference());
        assert!(manager.displays_edge_to_edge());

        assert_eq!(None, manager.system_bar_color());
        manager.set_system_bar_color(Some(Color::from_rgb(1, 2, 3)));
        assert_eq!(Some(Color::from_rgb(1, 2, 3)), manager.system_bar_color());
    }

    #[test]
    fn a_change_of_the_safe_area_is_reported_with_the_current_padding() {
        let _scope = Dispatcher::unit_test_scope();
        let (manager, state) = manager();
        *state.padding.borrow_mut() = vec![0.0, 10.0, 0.0, 20.0];
        let reported = Rc::new(RefCell::new(Vec::new()));
        let _subscription = manager.safe_area_changed({
            let reported = reported.clone();
            Rc::new(move |args: &SafeAreaChangedArgs| reported.borrow_mut().push(args.safe_area_padding()))
        });

        manager.notify_safe_area_padding_changed();

        assert_eq!(vec![Thickness::new(0.0, 10.0, 0.0, 20.0)], *reported.borrow());
    }
}
