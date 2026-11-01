//! Run under a disposable X server with Openbox, through the sample's session launcher.
use super::*;
use ferroui_base::threading::CancellationToken;
use ferroui_x11::{initialize_x11_platform, X11PlatformOptions, X11RenderingMode};
use std::time::{Duration, Instant};

fn until(session: &Session, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        session
            .platform
            .dispatcher_impl()
            .event_dispatcher()
            .dispatch_x11_events(&CancellationToken::none());
        Dispatcher::ui_thread().run_jobs(None);
        if ready() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "WM event timed out: {:?}",
            session.snapshot()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "requires a disposable X server with the Ferro Desktop Openbox configuration"]
fn real_client_lifecycle() {
    let platform = initialize_x11_platform(Some(X11PlatformOptions {
        rendering_mode: vec![X11RenderingMode::Software],
        ..Default::default()
    }));
    let session = Session::connect().unwrap();
    until(&session, || session.snapshot().workspace_names.len() >= 2);
    // A second X connection behaves like an ordinary application external to the shell.
    let display = xlib::x_open_display().unwrap();
    let id = xlib::x_create_simple_window(display, xlib::x_default_root_window(display), 20, 50, 220, 120, 0, 0, 0);
    let a = platform.info().atoms();
    xlib::x_set_wm_protocols(display, id, &[a.WM_DELETE_WINDOW]);
    xlib::x_store_name(display, id, "Session test client");
    xlib::x_map_window(display, id);
    xlib::x_flush(display);
    let window = || session.snapshot().windows.into_iter().find(|w| w.id == id as u32);
    until(&session, || window().is_some_and(|w| w.title == "Session test client"));
    xlib::x_change_property_bytes(
        display,
        id,
        a._NET_WM_NAME,
        a.UTF8_STRING,
        PropertyMode::Replace,
        b"Edited title",
    );
    xlib::x_flush(display);
    until(&session, || window().is_some_and(|w| w.title == "Edited title"));
    assert!(session.move_to_workspace(id as u32, 1));
    until(&session, || window().is_some_and(|w| w.workspace == Some(1)));
    assert!(session.switch_workspace(1));
    until(&session, || session.snapshot().current_workspace == 1);
    assert!(session.activate(id as u32));
    until(&session, || window().is_some_and(|w| w.active));
    assert!(session.minimize(id as u32));
    until(&session, || window().is_some_and(|w| w.minimized));
    assert!(session.activate(id as u32));
    until(&session, || window().is_some_and(|w| w.active && !w.minimized));
    assert!(session.close(id as u32));
    until(&session, || {
        while xlib::x_pending(display) > 0 {
            let event = xlib::x_next_event(display);
            if xlib::event_type(&event) == XEventName::ClientMessage as i32 {
                let message = xlib::client_message_event(&event);
                if message.message_type == a.WM_PROTOCOLS && message.data.get_long(0) as Atom == a.WM_DELETE_WINDOW {
                    return true;
                }
            }
        }
        false
    });
    xlib::x_destroy_window(display, id);
    xlib::x_flush(display);
    until(&session, || window().is_none());
    assert!(!session.close(id as u32));
}

#[test]
#[ignore = "requires a private X display managed by ferro-desktop-session"]
fn running_openbox_redecorates_the_same_native_client() {
    use crate::shell::appearance::Appearance;
    use std::path::PathBuf;
    let state = PathBuf::from(std::env::var_os("FERRO_DESKTOP_SESSION_DIR").expect("use the login supervisor"));
    let platform = initialize_x11_platform(Some(X11PlatformOptions {
        rendering_mode: vec![X11RenderingMode::Software],
        ..Default::default()
    }));
    let session = Session::connect().unwrap();
    let display = xlib::x_open_display().unwrap();
    let a = platform.info().atoms();
    let root = platform.info().root_window();
    let theme = xlib::x_intern_atom(display, "_OB_THEME", false);
    let id = xlib::x_create_simple_window(display, root, 60, 60, 220, 160, 0, 0, 0);
    xlib::x_store_name(display, id, "Frame appearance probe");
    xlib::x_map_window(display, id);
    xlib::x_flush(display);
    until(&session, || {
        session.snapshot().windows.iter().any(|w| w.id == id as u32)
    });
    let extents = || session.longs(id, a._NET_FRAME_EXTENTS, a.CARDINAL, 4);
    let theme_name = || session.bytes(root, theme, a.UTF8_STRING);
    let base = std::fs::read(state.join("base.xml")).unwrap();
    let before = std::fs::read(state.join("openbox.xml")).unwrap();
    // Even when the host inherits the supervisor environment, an ordinary windowed shell must
    // leave the surrounding manager alone.
    session.set_window_appearance(Appearance::Material).unwrap();
    assert_eq!(std::fs::read(state.join("openbox.xml")).unwrap(), before);
    let mut heights = Vec::new();
    for appearance in [Appearance::Windows, Appearance::MacOS, Appearance::Material] {
        let previous = theme_name();
        decorations::apply(&state, appearance).unwrap();
        until(&session, || {
            theme_name() != previous && theme_name().starts_with(b"FerroDesktop-")
        });
        // Openbox publishes _OB_THEME before it redecorates existing clients. Wait for both
        // the selected title bar's pixels and the native client/frame offset reported by X11.
        // Sampling the private 24-bit TrueColor root also proves the client was really repainted.
        let palette = appearance.palette();
        let colors = [palette.surface, palette.tile].map(|hex| XID::from_str_radix(&hex[1..], 16).unwrap());
        let mut observed = 0;
        until(&session, || {
            let Some(&top) = extents().get(2) else { return false };
            let Some((client_x, client_y, frame)) = xlib::x_translate_coordinates(display, id, root, 0, 0) else {
                return false;
            };
            if frame == 0 || frame == id || top < 3 {
                return false;
            }
            let Some((_, offset_y, _)) = xlib::x_translate_coordinates(display, id, frame, 0, 0) else {
                return false;
            };
            if offset_y != top as i32 {
                return false;
            }
            let pixel = xlib::x_get_pixel(display, root, client_x + 110, client_y - offset_y + 2);
            if !pixel.is_some_and(|pixel| colors.contains(&(pixel & 0x00ff_ffff))) {
                return false;
            }
            observed = top;
            true
        });
        heights.push(observed);
        assert!(session.snapshot().windows.iter().any(|w| w.id == id as u32));
        assert_eq!(std::fs::read(state.join("base.xml")).unwrap(), base);
    }
    assert!(
        heights[1] < heights[0] && heights[0] < heights[2],
        "Native titlebar heights: {heights:?}"
    );
    xlib::x_destroy_window(display, id);
    xlib::x_flush(display);
}
