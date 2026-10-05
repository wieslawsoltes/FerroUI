use super::{ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::cell::Cell;
use std::rc::Rc;

thread_local! {
    static DESIGNER_MODE: Cell<bool> = const { Cell::new(false) };
}

/// Creates the platform implementations of windows and other top-levels
/// through the registered [`IWindowingPlatform`].
pub struct PlatformManager;

impl PlatformManager {
    /// Enters the designer mode, in which windows are created as
    /// embeddable windows and no tray icons are created. Disposing the
    /// returned value leaves the mode.
    pub fn designer_mode() -> Rc<dyn IDisposable> {
        DESIGNER_MODE.set(true);
        Disposable::create(|| DESIGNER_MODE.set(false))
    }

    pub fn set_designer_scaling_factor(_factor: f64) {}

    /// Creates a tray icon, if a windowing platform is registered and it
    /// supports them.
    pub fn create_tray_icon() -> Option<Rc<dyn ITrayIconImpl>> {
        if DESIGNER_MODE.get() {
            None
        } else {
            FerroLocator::current().get_service::<dyn IWindowingPlatform>()?.create_tray_icon()
        }
    }

    /// Creates a window.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn create_window() -> Rc<dyn IWindowImpl> {
        let platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();

        if DESIGNER_MODE.get() {
            platform.create_embeddable_window()
        } else {
            platform.create_window()
        }
    }

    /// Creates a window that can be embedded into a foreign window.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn create_embeddable_window() -> Rc<dyn IWindowImpl> {
        let platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();
        platform.create_embeddable_window()
    }

    /// Creates a top-level that can be embedded into a foreign window.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn create_embeddable_top_level() -> Rc<dyn ITopLevelImpl> {
        let platform = FerroLocator::current().get_required_service::<dyn IWindowingPlatform>();
        platform.create_embeddable_top_level()
    }
}
