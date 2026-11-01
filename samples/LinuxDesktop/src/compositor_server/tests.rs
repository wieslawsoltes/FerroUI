//! An independent Wayland client verifies protocol ordering, client-owned
//! padded pixels, frame throttling and window state across desktop changes.
use super::{Input, Scene, Server};
use smithay::input::keyboard::xkb;
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::{
        fd::AsFd,
        unix::{fs::DirBuilderExt, net::UnixStream},
    },
    process::Command,
    sync::mpsc,
    time::{Duration, Instant},
};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

#[derive(Default)]
struct Client {
    configured: bool,
    frames: usize,
    released: usize,
    size: (i32, i32),
    closed: bool,
    keymap_count: usize,
    keyboard: Option<xkb::State>,
    typed: String,
}
delegate_noop!(Client: ignore wl_compositor::WlCompositor);
delegate_noop!(Client: ignore wl_shm::WlShm);
delegate_noop!(Client: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Client: ignore wl_surface::WlSurface);
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Client {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wl_buffer::WlBuffer, ()> for Client {
    fn event(
        client: &mut Self,
        _: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_buffer::Event::Release) {
            client.released += 1;
        }
    }
}
delegate_noop!(Client: ignore wl_seat::WlSeat);
impl Dispatch<wl_keyboard::WlKeyboard, ()> for Client {
    fn event(
        client: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { fd, .. } => {
                let mut text = String::new();
                File::from(fd).read_to_string(&mut text).unwrap();
                let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
                let map = xkb::Keymap::new_from_string(
                    &ctx,
                    text.trim_end_matches(char::from(0)).to_owned(),
                    xkb::KEYMAP_FORMAT_TEXT_V1,
                    xkb::KEYMAP_COMPILE_NO_FLAGS,
                )
                .unwrap();
                client.keyboard = Some(xkb::State::new(&map));
                client.keymap_count += 1;
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                client.keyboard.as_mut().unwrap().update_mask(
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    0,
                    0,
                    group,
                );
            }
            wl_keyboard::Event::Key { key, state, .. }
                if state == wayland_client::WEnum::Value(wl_keyboard::KeyState::Pressed) =>
            {
                client.typed.push_str(
                    &client
                        .keyboard
                        .as_ref()
                        .unwrap()
                        .key_get_utf8((key + 8).into()),
                );
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for Client {
    fn event(
        client: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_callback::Event::Done { .. }) {
            client.frames += 1;
        }
    }
}
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Client {
    fn event(
        _: &mut Self,
        wm: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm.pong(serial);
        }
    }
}
impl Dispatch<xdg_surface::XdgSurface, ()> for Client {
    fn event(
        client: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            client.configured = true;
        }
    }
}
impl Dispatch<xdg_toplevel::XdgToplevel, ()> for Client {
    fn event(
        client: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => client.size = (width, height),
            xdg_toplevel::Event::Close => client.closed = true,
            _ => {}
        }
    }
}

fn snapshot(server: &Server, wake: &mpsc::Receiver<()>, accept: impl Fn(&Scene) -> bool) -> Scene {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let scene = server.snapshot();
        if accept(&scene) {
            return scene;
        }
        wake.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("compositor scene");
    }
}

#[test]
fn commit_present_and_switch_workspace() {
    const CHILD: &str = "FERRO_PRIVATE_PROTOCOL_TEST";
    if std::env::var_os(CHILD).is_none() {
        let dir = std::env::temp_dir().join(format!(
            "ferro-wltest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "compositor_server::tests::commit_present_and_switch_workspace",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env("XDG_RUNTIME_DIR", &dir)
            .status()
            .unwrap();
        fs::remove_dir(&dir).unwrap();
        assert!(status.success());
        return;
    }

    let dir = std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").unwrap());
    let (tx, wake) = mpsc::channel();
    let server = Server::start(640, 480, Some("ferro-test"), move || {
        let _ = tx.send(());
    })
    .unwrap();
    assert!(Server::start(640, 480, Some("ferro-test"), || {}).is_err());
    let conn =
        Connection::from_socket(UnixStream::connect(dir.join(server.socket_name())).unwrap())
            .unwrap();
    let (globals, mut queue) = registry_queue_init::<Client>(&conn).unwrap();
    let handle = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&handle, 1..=4, ()).unwrap();
    let shm: wl_shm::WlShm = globals.bind(&handle, 1..=1, ()).unwrap();
    let shell: xdg_wm_base::XdgWmBase = globals.bind(&handle, 1..=3, ()).unwrap();
    let seat: wl_seat::WlSeat = globals.bind(&handle, 1..=7, ()).unwrap();
    let _kbd = seat.get_keyboard(&handle, ());
    let surface = compositor.create_surface(&handle, ());
    let role = shell.get_xdg_surface(&surface, &handle, ());
    let top = role.get_toplevel(&handle, ());
    top.set_title("Native test window".into());
    surface.commit();
    let mut client = Client::default();
    queue.roundtrip(&mut client).unwrap();
    assert!(client.configured);

    let path = dir.join("pool");
    let mut file = File::options()
        .create_new(true)
        .write(true)
        .read(true)
        .open(&path)
        .unwrap();
    fs::remove_file(path).unwrap();
    // Two rows of three XRGB pixels, each with a four-byte unused stride tail.
    let src = 0x00112233_u32.to_ne_bytes();
    file.write_all(&[src, src, src, [0x77; 4], src, src, src, [0x88; 4]].concat())
        .unwrap();
    let pool = shm.create_pool(file.as_fd(), 32, &handle, ());
    let buffer = pool.create_buffer(0, 3, 2, 16, wl_shm::Format::Xrgb8888, &handle, ());
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, 3, 2);
    surface.frame(&handle, ());
    surface.commit();
    queue.roundtrip(&mut client).unwrap();
    let shown = snapshot(&server, &wake, |s| s.surfaces.len() == 1);
    assert_eq!(
        shown.surfaces[0].pixels.bytes,
        [0x33, 0x22, 0x11, 0xff].repeat(6)
    );
    assert_eq!(client.released, 1);
    assert_eq!(client.frames, 0);
    let id = shown.windows[0].id;
    // Parent uses German layout and holds Shift before the child gained focus.
    // The same physical key then arrives without Shift, without a second map.
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let map = xkb::Keymap::new_from_names(
        &ctx,
        "evdev",
        "pc105",
        "de",
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .unwrap();
    let shift = 1 << map.mod_get_index(xkb::MOD_NAME_SHIFT);
    let serialized: std::sync::Arc<str> = map.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1).into();
    let previous_maps = client.keymap_count;
    for depressed in [shift, 0] {
        for pressed in [true, false] {
            server.send(Input::WaylandKey {
                event: ferroui_wayland::WaylandKeyEvent {
                    code: 21,
                    keymap: Some(serialized.clone()),
                    modifiers: ferroui_wayland::WaylandKeyModifiers {
                        depressed,
                        latched: 0,
                        locked: 0,
                        group: 0,
                    },
                    repeat: false,
                },
                pressed,
                time: 42,
            });
        }
    }
    server.send(Input::MoveToWorkspace { id, workspace: 1 });
    let hidden = snapshot(&server, &wake, |s| {
        s.surfaces.is_empty() && s.windows[0].workspace == 1
    });
    server.send(Input::Presented(hidden.serial));
    server.send(Input::AddWorkspace);
    snapshot(&server, &wake, |s| s.current_workspace == 4);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.frames, 0);
    assert_eq!(client.typed, "Zz");
    assert_eq!(client.keymap_count, previous_maps + 1);
    server.send(Input::RemoveWorkspace);
    server.send(Input::Activate(id));
    let restored = snapshot(&server, &wake, |s| {
        s.current_workspace == 1 && !s.surfaces.is_empty()
    });
    assert_eq!(restored.workspace_count, 4);
    server.send(Input::Presented(shown.serial)); // Obsolete credit from before hiding.
    server.send(Input::AlwaysOnTop(id));
    snapshot(&server, &wake, |s| s.windows[0].above);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.frames, 0);
    server.send(Input::Presented(restored.serial));
    server.send(Input::WorkArea {
        top: 20,
        bottom: 40,
    });
    server.send(Input::Maximize(id));
    snapshot(&server, &wake, |s| s.windows[0].maximized);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.frames, 1);
    assert_eq!(client.size, (640, 420));
    server.send(Input::Fullscreen(id));
    snapshot(&server, &wake, |s| s.windows[0].fullscreen);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.size, (640, 480));
    server.send(Input::Fullscreen(id));
    snapshot(&server, &wake, |s| !s.windows[0].fullscreen);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.size, (640, 420));
    server.send(Input::Resize {
        width: 641,
        height: 480,
    });
    server.send(Input::Snap { id, right: true });
    let snapped = snapshot(&server, &wake, |s| {
        !s.windows[0].maximized && s.windows[0].x == 320
    });
    queue.roundtrip(&mut client).unwrap();
    assert!(!snapped.windows[0].fullscreen);
    assert_eq!(client.size, (321, 420));
    server.send(Input::Minimize(id));
    snapshot(&server, &wake, |s| {
        s.windows[0].minimized && s.surfaces.is_empty()
    });
    server.send(Input::Close(id));
    server.send(Input::Activate(id));
    snapshot(&server, &wake, |s| !s.surfaces.is_empty());
    queue.roundtrip(&mut client).unwrap();
    assert!(client.closed);
    drop(server);
    assert!(!dir.join("ferro-test").exists());
}
