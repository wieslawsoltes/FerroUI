//! Clip native popup and toast windows to rounded bounds on non-composited
//! X11 desktops. A compositing manager uses the ordinary alpha channel.
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::{primitives::Popup, TopLevel};
use ferroui_x11::xlib::{self, XID};
use ferroui_x11::FerroX11Platform;
use std::{cell::RefCell, rc::Rc};

pub(super) fn track_popup(popup: &Popup, radius: f64) {
    // The popup owns this handler. Releasing its host drops all corresponding
    // root/scale/manager subscriptions, even if the menu is opened repeatedly.
    let current = RefCell::new(None);
    popup.popup_host_changed(move |host| {
        let root = host.and_then(|h| h.as_popup_root());
        current.replace(root.and_then(|w| track(&w.upcast::<TopLevel>(), radius)));
    }).detach();
}

pub(super) fn track(window: &Ref<TopLevel>, radius: f64) -> Option<Subscription> {
    let platform = FerroLocator::current().get_service::<FerroX11Platform>()?;
    if !platform.info().has_x_fixes() {
        return None;
    }
    let handle = window.try_get_platform_handle()?;
    let xid = handle.handle() as XID;
    let display = platform.info().display();
    let weak = window.downgrade();
    let globals = platform.globals().clone();
    let apply = Rc::new(move || {
        let Some(window) = weak.upgrade() else { return };
        if globals.is_composition_enabled() {
            xlib::x_fixes_set_bounding_shape(display, xid, None);
            return;
        }
        let scale = window.render_scaling();
        let size = window.client_size();
        let (w, h) = ((size.width * scale).round(), (size.height * scale).round());
        if !(1. ..=i16::MAX as f64).contains(&w) || !(1. ..=i16::MAX as f64).contains(&h) {
            return;
        }
        let mut shape = rounded_region(w as i16, h as i16, (radius * scale).round() as i16);
        xlib::x_fixes_set_bounding_shape(display, xid, Some(&mut shape));
    });
    apply();
    let resize = apply.clone();
    let resized = window.property_changed(move |event| {
        if event.property() == TopLevel::client_size_property().as_property() {
            resize();
        }
    });
    let scaling = apply.clone();
    let scaled = window.scaling_changed(move || scaling());
    let globals = platform.globals().clone();
    let token = globals.composition_changed.subscribe(move |()| apply());
    Some(Subscription::new(Disposable::create(move || {
        resized.dispose();
        scaled.dispose();
        globals.composition_changed.unsubscribe(token);
    })))
}

fn rounded_region(width: i16, height: i16, radius: i16) -> Vec<xlib::XRectangle> {
    let r = radius.clamp(0, (width.min(height)) / 2);
    let mut shape = Vec::with_capacity(usize::from(r as u16) * 2 + 1);
    let radius = f64::from(r);
    for y in 0..r {
        // Include a pixel if its center falls inside the corner arc.
        let dy = radius - f64::from(y) - 0.5;
        let inset = (radius - (radius * radius - dy * dy).sqrt() - 0.5).ceil() as i16;
        for row in [y, height - 1 - y] {
            shape.push(xlib::XRectangle {
                x: inset,
                y: row,
                width: (width - 2 * inset) as u16,
                height: 1,
            });
        }
    }
    if height > 2 * r {
        shape.push(xlib::XRectangle {
            x: 0,
            y: r,
            width: width as u16,
            height: (height - 2 * r) as u16,
        });
    }
    shape
}
