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
