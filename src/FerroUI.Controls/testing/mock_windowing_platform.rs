use super::mock_window_impl::MockWindowImpl;
use crate::platform::{IPopupImpl, ITopLevelImpl, ITrayIconImpl, IWindowBaseImpl, IWindowImpl, IWindowingPlatform};
use std::rc::Rc;

type WindowImplFactory = Rc<dyn Fn() -> Rc<dyn IWindowImpl>>;
type PopupImplFactory = Rc<dyn Fn(&dyn IWindowBaseImpl) -> Option<Rc<dyn IPopupImpl>>>;
type TrayIconImplFactory = Rc<dyn Fn() -> Option<Rc<dyn ITrayIconImpl>>>;

/// A windowing platform whose windows and popups are [`MockWindowImpl`]s.
#[derive(Default)]
pub struct MockWindowingPlatform {
    window_impl: Option<WindowImplFactory>,
    popup_impl: Option<PopupImplFactory>,
    tray_icon_impl: Option<TrayIconImplFactory>,
}

impl MockWindowingPlatform {
    /// A platform that creates the default window and popup mocks.
    pub fn new() -> Rc<MockWindowingPlatform> {
        Rc::new(MockWindowingPlatform::default())
    }

    /// A platform with the given factories; `None` stands for the default
    /// mocks (windows, popups) or for nothing (tray icons).
    pub fn with(
        window_impl: Option<WindowImplFactory>,
        popup_impl: Option<PopupImplFactory>,
        tray_icon_impl: Option<TrayIconImplFactory>,
    ) -> Rc<MockWindowingPlatform> {
        Rc::new(MockWindowingPlatform { window_impl, popup_impl, tray_icon_impl })
    }

    /// A platform whose windows are created by `window_impl`.
    pub fn with_window_impl(window_impl: impl Fn() -> Rc<dyn IWindowImpl> + 'static) -> Rc<MockWindowingPlatform> {
        Self::with(Some(Rc::new(window_impl)), None, None)
    }

    /// Creates the default window mock with an 800x600 client area.
    pub fn create_window_mock() -> Rc<MockWindowImpl> {
        MockWindowImpl::window(800.0, 600.0)
    }

    /// Creates the default window mock with the given client area.
    pub fn create_window_mock_with_size(initial_width: f64, initial_height: f64) -> Rc<MockWindowImpl> {
        MockWindowImpl::window(initial_width, initial_height)
    }

    /// Creates the default popup mock, positioned relative to `parent`.
    pub fn create_popup_mock(parent: Rc<dyn ITopLevelImpl>) -> Rc<MockWindowImpl> {
        MockWindowImpl::popup(parent)
    }
}

impl IWindowingPlatform for MockWindowingPlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        if let Some(window_impl) = &self.window_impl {
            window_impl()
        } else {
            let mock = Self::create_window_mock();

            if let Some(popup_impl) = self.popup_impl.clone() {
                mock.setup_create_popup(move |mock| popup_impl(mock));
            }

            mock
        }
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        self.create_embeddable_window()
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("The mock windowing platform does not create embeddable windows.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        self.tray_icon_impl.as_ref().and_then(|tray_icon_impl| tray_icon_impl())
    }

    fn get_windows_z_order(&self, _windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        z_order.fill(0);
    }
}
