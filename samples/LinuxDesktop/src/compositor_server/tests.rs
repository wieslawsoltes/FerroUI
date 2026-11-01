//! An independent Wayland client verifies protocol ordering, client-owned
//! padded pixels, frame throttling and window state across desktop changes.
use super::{Input, Scene, Server};
use std::{
    fs::{self, File},
    io::Write,
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
        wl_buffer, wl_callback, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface,
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
    server.send(Input::MoveToWorkspace { id, workspace: 1 });
    let hidden = snapshot(&server, &wake, |s| {
        s.surfaces.is_empty() && s.windows[0].workspace == 1
    });
    server.send(Input::Presented(hidden.serial));
    server.send(Input::AddWorkspace);
    snapshot(&server, &wake, |s| s.current_workspace == 4);
    queue.roundtrip(&mut client).unwrap();
    assert_eq!(client.frames, 0);
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
