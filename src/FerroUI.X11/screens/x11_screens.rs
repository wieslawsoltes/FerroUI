//! The screens of the server (the port of `X11Screens.cs`).

use super::x11_screen_providers::{FallbackScreensImpl, IX11RawScreenInfoProvider, Randr15ScreensImpl, X11Screen};
use crate::event::Event;
use crate::x11_info::Version;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::Atom;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::platform::{ScreensBase, ScreensBaseImpl, ScreensBaseImplExt};
use std::rc::Rc;

/// The screens implementation of the X11 backend: one screen for every
/// monitor of RandR 1.5 or newer, and the root window as the one screen of
/// a server without it. The key of a screen is the atom of the name of its
/// monitor.
pub struct X11Screens {
    base: ScreensBase<Atom, X11Screen>,
    impl_: Rc<dyn IX11RawScreenInfoProvider>,
    /// Raised when the screens changed, after the handler of the base
    /// class was called.
    ///
    /// A representation difference: in the reference several parties add
    /// handlers to the one `Changed` delegate of the base class. The base
    /// class of this framework has a single slot for it, which the
    /// controls layer takes; the other parties of this backend subscribe
    /// here. See [`notify_changed`](Self::notify_changed).
    pub changed_event: Event,
}

impl X11Screens {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<X11Screens> {
        let info = platform.info();
        let impl_: Rc<dyn IX11RawScreenInfoProvider> =
            if info.randr_version().is_some_and(|version| version >= Version::new(1, 5)) {
                Randr15ScreensImpl::new(platform)
            } else {
                FallbackScreensImpl::new(platform)
            };
        let this = Rc::new(X11Screens { base: ScreensBase::new(), impl_, changed_event: Event::new() });
        let weak = Rc::downgrade(&this);
        this.impl_.subscribe_changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.notify_changed();
            }
        }));
        this
    }

    /// What the reference does with `OnChanged()` of the base class: the
    /// cached screens are invalidated and the changed notification of the
    /// base class is scheduled; right after it, at the same priority, the
    /// handlers of [`changed_event`](Self::changed_event) are scheduled.
    ///
    /// The base class notifies its handler only when it has one or when a
    /// screen was asked for before; the handlers of the event are called
    /// for every change.
    pub fn notify_changed(self: &Rc<Self>) {
        self.on_changed();

        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.changed_event.raise();
                }
            },
            DispatcherPriority::INPUT,
        );
    }

    /// The highest refresh rate of the screens, and at least the default
    /// rate; the default rate when the provider does not know refresh
    /// rates.
    pub fn max_refresh_rate(&self) -> i32 {
        effective_max_refresh_rate(
            self.impl_.as_provider_with_refresh_rate().map(|refresh_provider| refresh_provider.max_refresh_rate()),
        )
    }
}

/// The refresh rate to render at for the rate a provider answers (`None`
/// for a provider that does not know refresh rates): never below the
/// default rate.
fn effective_max_refresh_rate(provider_max_refresh_rate: Option<i32>) -> i32 {
    match provider_max_refresh_rate {
        Some(max_refresh_rate) => FerroX11Platform::DEFAULT_FPS.max(max_refresh_rate),
        None => FerroX11Platform::DEFAULT_FPS,
    }
}

impl ScreensBaseImpl for X11Screens {
    type Key = Atom;
    type Screen = X11Screen;

    fn screens_base(&self) -> &ScreensBase<Atom, X11Screen> {
        &self.base
    }

    fn get_screen_count(&self) -> i32 {
        self.impl_.screen_keys().len() as i32
    }

    fn get_all_screen_keys(&self) -> Vec<Atom> {
        self.impl_.screen_keys()
    }

    fn create_screen_from_key(&self, key: &Atom) -> Rc<X11Screen> {
        Rc::new(self.impl_.create_screen_from_key(*key))
    }

    fn screen_changed(&self, screen: &Rc<X11Screen>) {
        let handle = screen.try_get_platform_handle().map(|handle| handle.handle());
        if let Some(handle) = handle {
            screen.refresh(&self.impl_.get_monitor_info_by_key(handle as Atom));
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_refresh_rate_is_never_below_the_default_rate() {
        assert_eq!(effective_max_refresh_rate(None), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(effective_max_refresh_rate(Some(144)), 144);
        assert_eq!(effective_max_refresh_rate(Some(30)), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(effective_max_refresh_rate(Some(0)), FerroX11Platform::DEFAULT_FPS);
        assert_eq!(effective_max_refresh_rate(Some(FerroX11Platform::DEFAULT_FPS)), FerroX11Platform::DEFAULT_FPS);
    }
}
