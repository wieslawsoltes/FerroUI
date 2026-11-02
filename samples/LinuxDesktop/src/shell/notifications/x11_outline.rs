//! A non-composited X server treats client alpha as opaque. Clip only the
//! native toast's bounding region there so the wallpaper remains visible
//! around rounded corners. A compositing manager uses the usual alpha path.
use super::*;
use ferroui_controls::TopLevel;

pub(super) fn attach(window: &Ref<Window>, radius: f64) {
    let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() else {
        return;
    };
    if !platform.info().has_x_fixes() {
        return;
    }
    let Some(handle) = window.try_get_platform_handle() else {
        return;
    };
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
    on_cleanup(move || {
        resized.dispose();
        scaled.dispose();
        globals.composition_changed.unsubscribe(token);
    });
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
