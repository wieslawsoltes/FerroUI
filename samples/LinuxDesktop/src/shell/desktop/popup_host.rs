//! Select a popup host that can actually cover clients on the active backend.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PopupHost {
    InScene,
    Native,
    OpaqueNative,
}

impl PopupHost {
    pub(super) fn current() -> Self {
        use ferroui_base::{FerroLocator, LocatorExtensions};
        if use_context::<super::super::ManagedDesktopWindow>().is_some() {
            let platform = FerroLocator::current().get_service::<ferroui_x11::FerroX11Platform>();
            if platform.is_some_and(|x| !x.globals().is_composition_enabled()) {
                Self::OpaqueNative
            } else {
                Self::Native
            }
        } else if use_context::<DesktopBackdrop>().is_some() {
            Self::InScene
        } else {
            // A Wayland panel is a narrow layer surface. Its native popup
            // can extend beyond the edge reservation.
            Self::Native
        }
    }
    pub(super) fn configure(self, popup: &ferroui_controls::primitives::Popup, radius: f64) {
        popup.set_should_use_overlay_layer(self == Self::InScene);
        if self == Self::OpaqueNative {
            super::super::x11_outline::track_popup(popup, radius);
        }
    }
}

pub(super) fn configure_menu(menu: &ferroui_controls::MenuFlyout, radius: f64) {
    let host = PopupHost::current();
    host.configure(&menu.popup(), radius);
    if host == PopupHost::OpaqueNative {
        let weak = menu.to_ref().downgrade();
        let surface = colors().surface;
        menu.opening(move |_| {
            if let Some(presenter) = weak.upgrade().and_then(|menu| {
                menu.popup()
                    .child()?
                    .downcast::<ferroui_controls::MenuFlyoutPresenter>()
                    .ok()
            }) {
                presenter.set_margin(Default::default());
                presenter.set_corner_radius(Default::default());
                presenter.set_background(Some(opaque_background(surface)));
            }
        })
        .detach();
    }
}

pub(super) fn opaque_background(color: &str) -> Rc<dyn ferroui_base::media::IBrush> {
    use ferroui_base::media::{Color, immutable::ImmutableSolidColorBrush};
    Rc::new(ImmutableSolidColorBrush::new(
        Color::parse(color).expect("desktop surface color"),
    ))
}
