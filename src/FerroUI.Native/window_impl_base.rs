//! What windows and popups have in common.

use crate::helpers::*;
use crate::interop::*;
use crate::top_level_impl::{callback_property, MacOSTopLevelHandle, TopLevelEvents, TopLevelImpl, TopLevelParent};
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Size};
use ferroui_controls::platform::PlatformThemeVariant;
use ferroui_controls::{WindowEdge, WindowResizeReason};
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::rc::Rc;

/// What the concrete window types provide to the shared window code.
pub(crate) trait WindowBaseParent: TopLevelParent {
    /// The shared window state.
    fn window_base(&self) -> &WindowBaseImpl;

    /// Shows the window.
    fn show_core(&self, activate: bool, is_dialog: bool) {
        self.window_base().show(activate, is_dialog);
    }
}

/// The receiver of the window events of the native side for the concrete
/// window types.
pub(crate) trait WindowEventsParent: WindowBaseParent {
    fn on_closing(&self) -> bool;
    fn on_window_state_changed(&self, state: FrnWindowState);
    fn on_got_input_when_disabled(&self);
}

/// The state and behaviour shared by windows and popups.
pub struct WindowBaseImpl {
    base: Rc<TopLevelImpl>,
    deactivated: RefCell<Option<Rc<dyn Fn()>>>,
    activated: RefCell<Option<Rc<dyn Fn()>>>,
    position_changed: RefCell<Option<Rc<dyn Fn(PixelPoint)>>>,
}

impl WindowBaseImpl {
    pub(crate) fn new(factory: ComPtr<IFerroNativeFactory>) -> WindowBaseImpl {
        WindowBaseImpl {
            base: TopLevelImpl::new(factory),
            deactivated: RefCell::new(None),
            activated: RefCell::new(None),
            position_changed: RefCell::new(None),
        }
    }

    pub(crate) fn top_level(&self) -> &Rc<TopLevelImpl> {
        &self.base
    }

    /// The native window; `None` once it is disposed.
    pub fn native(&self) -> Option<ComPtr<IFrnWindowBase>> {
        self.base.native_window_base()
    }

    pub fn position(&self) -> PixelPoint {
        match self.native() {
            Some(native) => to_ferro_pixel_point(native.get_position().check()),
            None => PixelPoint::default(),
        }
    }

    pub fn set_position(&self, value: PixelPoint) {
        if let Some(native) = self.native() {
            native.set_position(pixel_point_to_frn_point(value)).check();
        }
    }

    callback_property!(deactivated, set_deactivated, deactivated, dyn Fn());
    callback_property!(activated, set_activated, activated, dyn Fn());
    callback_property!(position_changed, set_position_changed, position_changed, dyn Fn(PixelPoint));

    pub fn frame_size(&self) -> Option<Size> {
        let native = self.native()?;
        // The native side leaves the size at (-1, -1) when the window has
        // no frame.
        let s = native.get_frame_size().check();
        if s.width < 0.0 && s.height < 0.0 {
            None
        } else {
            Some(Size::new(s.width, s.height))
        }
    }

    pub(crate) fn init(&self, handle: Rc<MacOSTopLevelHandle>) {
        self.base.init(handle);

        let mut default_width = 0;
        let mut default_height = 0;

        // The screen with the lowest scaling that contains the window.
        let position = self.position();
        let mut screens = self.base.screens().all_screens();
        screens.sort_by(|a, b| a.scaling().partial_cmp(&b.scaling()).unwrap_or(std::cmp::Ordering::Equal));
        let monitor = screens.into_iter().find(|m| m.bounds().contains(position));

        if let Some(monitor) = monitor {
            // Emulate Windows 7+ default window size behavior.
            default_width = (monitor.working_area().width as f64 * 0.75) as i32;
            default_height = (monitor.working_area().height as f64 * 0.7) as i32;
        }

        default_width = default_width.max(300);
        default_height = default_height.max(200);

        self.resize(Size::new(default_width as f64, default_height as f64), WindowResizeReason::Layout);
    }

    pub fn activate(&self) {
        if let Some(native) = self.native() {
            native.activate().check();
        }
    }

    pub fn resize(&self, client_size: Size, reason: WindowResizeReason) {
        if let Some(native) = self.native() {
            native.resize(client_size.width, client_size.height, to_frn_resize_reason(reason)).check();
        }
    }

    pub fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>) {
        // The theme variant enum of the top-level contract and the one of
        // the settings contract are two types with the same values.
        let theme_variant = match theme_variant {
            Some(theme_variant) => theme_variant as i32,
            None => FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .map_or(PlatformThemeVariant::Light as i32, |settings| {
                    settings.get_color_values().theme_variant() as i32
                }),
        };
        if let Some(native) = self.native() {
            native.set_frame_theme_variant(FrnPlatformThemeVariant(theme_variant)).check();
        }
    }

    pub(crate) fn dispose(&self) {
        if let Some(native) = self.native() {
            native.close().check();
        }
        self.base.dispose();
    }

    pub fn show(&self, activate: bool, is_dialog: bool) {
        if let Some(native) = self.native() {
            native.show(activate, is_dialog).check();
        }
    }

    pub fn hide(&self) {
        if let Some(native) = self.native() {
            native.hide().check();
        }
    }

    pub fn begin_move_drag(&self) {
        if let Some(native) = self.native() {
            native.begin_move_drag().check();
        }
    }

    pub fn max_auto_size_hint(&self) -> Size {
        // The largest screen by width + height; the first one wins a tie.
        let mut best: Option<Size> = None;
        for screen in self.base.screens().all_screens() {
            let size = screen.bounds().size().to_size(1.0);
            if best.is_none_or(|best| size.width + size.height > best.width + best.height) {
                best = Some(size);
            }
        }
        best.unwrap_or_default()
    }

    pub fn set_topmost(&self, value: bool) {
        if let Some(native) = self.native() {
            native.set_top_most(value).check();
        }
    }

    // TODO (as in the reference implementation): not implemented on macOS.
    pub fn begin_resize_drag(&self, _edge: WindowEdge) {}

    pub fn set_min_max_size(&self, min_size: Size, max_size: Size) {
        if let Some(native) = self.native() {
            native.set_min_max_size(to_frn_size(min_size), to_frn_size(max_size)).check();
        }
    }
}

impl<P: WindowBaseParent> IFrnWindowBaseEventsImpl for TopLevelEvents<P> {
    fn activated(&self) {
        crate::callback_base::guard((), || {
            let activated = self.0.window_base().activated();
            if let Some(activated) = activated {
                activated();
            }
        })
    }

    fn deactivated(&self) {
        crate::callback_base::guard((), || {
            let deactivated = self.0.window_base().deactivated();
            if let Some(deactivated) = deactivated {
                deactivated();
            }
        })
    }

    fn position_changed(&self, position: FrnPoint) {
        crate::callback_base::guard((), || {
            let position_changed = self.0.window_base().position_changed();
            if let Some(position_changed) = position_changed {
                position_changed(to_ferro_pixel_point(position));
            }
        })
    }
}

impl<P: WindowEventsParent> IFrnWindowEventsImpl for TopLevelEvents<P> {
    fn closing(&self) -> bool {
        crate::callback_base::guard(true, || self.0.on_closing())
    }

    fn window_state_changed(&self, state: FrnWindowState) {
        crate::callback_base::guard((), || self.0.on_window_state_changed(state))
    }

    fn got_input_when_disabled(&self) {
        crate::callback_base::guard((), || self.0.on_got_input_when_disabled())
    }
}

/// Implements `IWindowBaseImpl` for a type that is a [`WindowBaseParent`].
macro_rules! impl_window_base_contract {
    ($ty:ty) => {
        impl ferroui_controls::platform::IWindowBaseImpl for $ty {
            fn frame_size(&self) -> Option<ferroui_base::Size> {
                $crate::window_impl_base::WindowBaseParent::window_base(self).frame_size()
            }

            fn show(&self, activate: bool, is_dialog: bool) {
                $crate::window_impl_base::WindowBaseParent::show_core(self, activate, is_dialog)
            }

            fn hide(&self) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).hide()
            }

            fn position(&self) -> ferroui_base::PixelPoint {
                $crate::window_impl_base::WindowBaseParent::window_base(self).position()
            }

            fn position_changed(&self) -> Option<std::rc::Rc<dyn Fn(ferroui_base::PixelPoint)>> {
                $crate::window_impl_base::WindowBaseParent::window_base(self).position_changed()
            }

            fn set_position_changed(&self, value: Option<std::rc::Rc<dyn Fn(ferroui_base::PixelPoint)>>) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).set_position_changed(value)
            }

            fn activate(&self) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).activate()
            }

            fn deactivated(&self) -> Option<std::rc::Rc<dyn Fn()>> {
                $crate::window_impl_base::WindowBaseParent::window_base(self).deactivated()
            }

            fn set_deactivated(&self, value: Option<std::rc::Rc<dyn Fn()>>) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).set_deactivated(value)
            }

            fn activated(&self) -> Option<std::rc::Rc<dyn Fn()>> {
                $crate::window_impl_base::WindowBaseParent::window_base(self).activated()
            }

            fn set_activated(&self, value: Option<std::rc::Rc<dyn Fn()>>) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).set_activated(value)
            }

            fn max_auto_size_hint(&self) -> ferroui_base::Size {
                $crate::window_impl_base::WindowBaseParent::window_base(self).max_auto_size_hint()
            }

            fn set_topmost(&self, value: bool) {
                $crate::window_impl_base::WindowBaseParent::window_base(self).set_topmost(value)
            }
        }
    };
}
pub(crate) use impl_window_base_contract;
