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

fn native_frame(session: &Session, display: xlib::XDisplay, id: XID) -> Option<PixelRect> {
    let a = session.platform.info().atoms();
    let (x, y, _) = xlib::x_translate_coordinates(display, id, session.platform.info().root_window(), 0, 0)?;
    let geometry = xlib::x_get_geometry(display, id)?;
    let frame = session.longs(id, a._NET_FRAME_EXTENTS, a.CARDINAL, 4);
    let [left, right, top, bottom] = frame.as_slice() else { return None };
    Some(PixelRect::new(x - *left as i32, y - *top as i32,
        geometry.width + (*left + *right) as i32, geometry.height + (*top + *bottom) as i32))
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
    assert!(window().unwrap().last_activation > 0);
    let original = native_frame(&session, display, id).unwrap();
    assert!(session.toggle_maximized(id as u32));
    until(&session, || window().is_some_and(|w| w.maximized));
    assert!(session.toggle_maximized(id as u32));
    until(&session, || window().is_some_and(|w| !w.maximized)
        && native_frame(&session, display, id) == Some(original));
    // Snap directly out of maximized state. Openbox changes frame extents on restore.
    assert!(session.toggle_maximized(id as u32));
    until(&session, || window().is_some_and(|w| w.maximized));
    let work = session.longs(platform.info().root_window(), a._NET_WORKAREA, a.CARDINAL, 8);
    let [x, y, width, height] = work[4..8] else { unreachable!() };
    let half = width / 2;
    for (side, target) in [
        (SnapSide::Left, PixelRect::new(x as i32, y as i32, half as i32, height as i32)),
        (SnapSide::Right, PixelRect::new((x+half) as i32, y as i32, (width-half) as i32, height as i32)),
    ] {
        assert!(session.snap(id as u32, side));
        until(&session, || native_frame(&session, display, id) == Some(target));
        assert!(!window().unwrap().maximized);
    }
    assert!(session.toggle_fullscreen(id as u32));
    until(&session, || window().is_some_and(|w| w.fullscreen));
    assert!(!session.snap(id as u32, SnapSide::Left));
    assert!(session.toggle_fullscreen(id as u32));
    until(&session, || window().is_some_and(|w| !w.fullscreen));
    assert!(session.toggle_above(id as u32));
    until(&session, || window().is_some_and(|w| w.above));
    assert!(session.toggle_above(id as u32));
    until(&session, || window().is_some_and(|w| !w.above));
    let previous_count = session.snapshot().workspace_names.len();
    assert!(session.add_workspace());
    until(&session, || session.snapshot().workspace_names.len() == previous_count + 1);
    assert!(session.move_to_workspace(id as u32, previous_count));
    until(&session, || window().is_some_and(|w| w.workspace == Some(previous_count)));
    assert!(!session.remove_empty_last_workspace());
    assert!(session.move_to_workspace(id as u32, 1));
    until(&session, || window().is_some_and(|w| w.workspace == Some(1)));
    assert!(session.remove_empty_last_workspace());
    until(&session, || session.snapshot().workspace_names.len() == previous_count);
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

/// Counts protocol requests on the shell connection. The client connection submits and
/// serializes each title change before the timed interval so server and shell input are ready.
#[test]
#[ignore = "benchmark on a private X11 display: tests/session.sh benchmark"]
fn window_title_traffic() {
    use ferroui_base::platform::native_library::NativeLibrary;
    use std::ffi::{c_ulong, c_void};
    ferroui_base::native_functions! {
        struct Sequence(xlib) {
            xlib::next_request as "XNextRequest": unsafe extern "C" fn(*mut c_void) -> c_ulong;
        }
    }
    let sequence = Sequence::load(&NativeLibrary::open(c"libX11.so.6").unwrap()).unwrap();
    let platform = initialize_x11_platform(Some(X11PlatformOptions {
        rendering_mode: vec![X11RenderingMode::Software],
        ..Default::default()
    }));
    let session = Session::connect().unwrap();
    let info = platform.info();
    let a = info.atoms();
    let producer = xlib::x_open_display().unwrap();
    let mut ids = Vec::new();
    for i in 0..32 {
        let id = xlib::x_create_simple_window(producer, info.root_window(), 20+i, 30+i, 140, 110, 0, 0, 0);
        xlib::x_store_name(producer, id, "Static window");
        xlib::x_map_window(producer, id);
        ids.push(id);
    }
    xlib::x_sync(producer, false);
    until(&session, || session.snapshot().windows.len() == ids.len());
    // SAFETY: platform retains the live UI-thread Xlib display used by this benchmark.
    let serial = || unsafe { (sequence.next_request)(info.display().as_ptr().cast()) };
    let mut requests = 0;
    let mut times = Vec::with_capacity(200);
    for i in 0..205 {
        let title = format!("Document {i}");
        let id = ids[i % ids.len()];
        xlib::x_change_property_bytes(producer, id, a._NET_WM_NAME, a.UTF8_STRING, PropertyMode::Replace, title.as_bytes());
        xlib::x_sync(producer, false);
        xlib::x_sync(info.display(), false);
        let before = serial();
        let start = Instant::now();
        platform.dispatcher_impl().event_dispatcher().dispatch_x11_events(&CancellationToken::none());
        Dispatcher::ui_thread().run_jobs(None);
        let elapsed = start.elapsed();
        let after = serial();
        assert!(session.snapshot().windows.iter().any(|w| w.id == id as u32 && w.title == title));
        if i >= 5 {
            times.push(elapsed);
            requests += after.wrapping_sub(before);
        }
    }
    times.sort_unstable();
    println!("32 clients, 200 title updates: {requests} X11 requests, median {:?}, p95 {:?}",
        times[times.len()/2], times[times.len()*95/100]);
    for id in ids {
        xlib::x_destroy_window(producer, id);
    }
    xlib::x_flush(producer);
}
