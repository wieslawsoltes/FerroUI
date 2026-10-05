use crate::platform::{IScreenImpl, Screen};
use crate::{TopLevel, WindowBase};
use ferroui_base::utilities::HandlerList;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{PixelPoint, PixelRect, Visual};
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};

/// Represents all screens available on a device.
pub struct Screens {
    this: Weak<Screens>,
    i_screen_impl: Rc<dyn IScreenImpl>,
    changed_handlers: HandlerList<dyn Fn()>,
}

impl Screens {
    /// Creates the screens object over a platform screens implementation.
    pub fn new(i_screen_impl: Rc<dyn IScreenImpl>) -> Rc<Screens> {
        Rc::new_cyclic(|this| Screens { this: this.clone(), i_screen_impl, changed_handlers: HandlerList::new() })
    }

    /// The number of screens available on the device.
    pub fn screen_count(&self) -> i32 {
        self.i_screen_impl.screen_count()
    }

    /// All screens available on the device.
    pub fn all(&self) -> Vec<Rc<Screen>> {
        self.i_screen_impl.all_screens()
    }

    /// The primary screen on the device.
    pub fn primary(&self) -> Option<Rc<Screen>> {
        self.all().into_iter().find(|x| x.is_primary())
    }

    /// Fired when screens are added or removed, or when a screen's
    /// properties change. Disposing the returned handle unsubscribes.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        if self.changed_handlers.is_empty() {
            let weak = self.this.clone();
            self.i_screen_impl.set_changed(Some(Rc::new(move || {
                if let Some(screens) = weak.upgrade() {
                    screens.impl_changed();
                }
            })));
        }
        let token = self.changed_handlers.add(Rc::new(handler));

        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(screens) = weak.upgrade() {
                screens.changed_handlers.remove(token);
                if screens.changed_handlers.is_empty() {
                    screens.i_screen_impl.set_changed(None);
                }
            }
        })
    }

    /// Retrieves the screen that contains the largest area of the specified
    /// bounds.
    pub fn screen_from_bounds(&self, bounds: PixelRect) -> Option<Rc<Screen>> {
        self.i_screen_impl.screen_from_rect(bounds)
    }

    /// Retrieves the screen for the specified window.
    ///
    /// # Panics
    /// Panics when the window platform implementation was already disposed.
    pub fn screen_from_window(&self, window: &WindowBase) -> Option<Rc<Screen>> {
        let Some(platform_impl) = window.platform_impl() else {
            panic!("Window platform implementation was already disposed.");
        };
        let window_impl = platform_impl
            .as_window_base_impl()
            .expect("the platform implementation of a window is a window base implementation");
        self.i_screen_impl.screen_from_window(window_impl)
    }

    /// Retrieves the screen for the specified top-level.
    ///
    /// # Panics
    /// Panics when the platform implementation was already disposed.
    pub fn screen_from_top_level(&self, top_level: &TopLevel) -> Option<Rc<Screen>> {
        let Some(platform_impl) = top_level.platform_impl() else {
            panic!("Window platform implementation was already disposed.");
        };
        self.i_screen_impl.screen_from_top_level(&*platform_impl)
    }

    /// Retrieves the screen that contains the specified point.
    pub fn screen_from_point(&self, point: PixelPoint) -> Option<Rc<Screen>> {
        self.i_screen_impl.screen_from_point(point)
    }

    /// Retrieves the screen that contains the specified visual.
    ///
    /// # Panics
    /// Panics when the visual does not belong to a visual tree.
    pub fn screen_from_visual(&self, visual: &Visual) -> Option<Rc<Screen>> {
        let Some(top_level) = TopLevel::get_top_level(Some(visual)) else {
            panic!("Control does not belong to a visual tree.");
        };

        if top_level.is::<WindowBase>() {
            let tl = visual.point_to_screen(visual.bounds().top_left());
            let br = visual.point_to_screen(visual.bounds().bottom_right());

            // Attempt to get screen from the physical position on any screen first. Fallback to the screen hosting top level.
            self.screen_from_bounds(PixelRect::from_points(tl, br)).or_else(|| self.screen_from_top_level(&top_level))
        } else {
            self.screen_from_top_level(&top_level)
        }
    }

    /// Asks the underlying system for permission to get more detailed
    /// information about screens.
    ///
    /// On the browser platform the user has to grant permission; on other
    /// platforms the details are always available.
    pub fn request_screen_details(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        self.i_screen_impl.request_screen_details()
    }

    fn impl_changed(&self) {
        for (_, handler) in self.changed_handlers.snapshot().iter() {
            handler();
        }
    }
}
