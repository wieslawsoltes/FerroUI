use super::*;
use ferroui_base::threading::Dispatcher;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn malformed_frames_fail_before_reading_a_body() {
    assert!(ipc::receive(std::io::Cursor::new(vec![0u8; 14])).is_err());
    let mut oversized = b"i3-ipc".to_vec();
    oversized.extend_from_slice(&u32::MAX.to_ne_bytes());
    oversized.extend_from_slice(&4u32.to_ne_bytes());
    assert!(ipc::receive(std::io::Cursor::new(oversized)).is_err());
}

#[test]
fn workspace_names_cannot_add_compositor_commands() {
    let mut ws = tree::Workspace {
        id: 23,
        name: "code; kill, exec xyz".into(),
        number: 4,
        output: "HEADLESS-1".into(),
        area: Rect::default(),
    };
    assert_eq!(ws.target(&[ws.clone()]).as_deref(), Some("number 4"));
    ws.number = -1;
    assert_eq!(
        ws.target(&[ws.clone()]).as_deref(),
        Some("\"code; kill, exec xyz\"")
    );
    ws.name = "test\", exec no; 'also'".into();
    assert_eq!(ws.target(&[ws.clone()]), None);
    ws.name = "$workspace".into();
    assert_eq!(ws.target(&[ws.clone()]), None);
}

#[test]
fn groups_are_not_windows_and_internal_scratchpad_is_not_a_desktop() {
    let spaces = json!([{"id":4,"num":1,"name":"1","focused":true,"output":"HEADLESS-1","rect":{"x":-100,"y":20,"width":101,"height":80}}]);
    let tree = json!({"type":"root","nodes":[{"type":"output","nodes":[{"type":"workspace","id":4,"name":"1",
        "nodes":[{"type":"con","id":7,"nodes":[{"type":"con","id":8,"name":"Editor","app_id":"org.test.Editor","focused":true}]}],
        "floating_nodes":[{"type":"floating_con","id":9,"window":432,"name":"XWayland"}]}]},
        {"type":"output","name":"__i3","nodes":[{"type":"workspace","name":"__i3_scratch","floating_nodes":[{"type":"floating_con","id":10,"shell":"xdg_shell"}]}]}]});
    let model = Tree::parse(&spaces, &tree).unwrap();
    assert_eq!(model.snapshot.workspace_names, ["1"]);
    assert_eq!(
        model
            .snapshot
            .windows
            .iter()
            .map(|w| (w.id, w.active, w.minimized))
            .collect::<Vec<_>>(),
        [(8, true, false), (9, false, false), (10, false, true)]
    );
    assert_eq!(
        model.spaces[0].area.half(true),
        Rect {
            x: -50,
            y: 20,
            width: 51,
            height: 80
        }
    );
}

#[test]
fn native_title_and_focus_burst_uses_no_tree_requests() {
    let (mut input, mut events) = UnixStream::pair().unwrap();
    let (mut snapshots, spy) = UnixStream::pair().unwrap();
    snapshots.set_nonblocking(true).unwrap();
    let mut frames = Vec::new();
    let mut event = |kind: u32, payload: serde_json::Value| {
        let bytes = serde_json::to_vec(&payload).unwrap();
        frames.extend_from_slice(b"i3-ipc");
        frames.extend_from_slice(&(bytes.len() as u32).to_ne_bytes());
        frames.extend_from_slice(&(ipc::EVENT | kind).to_ne_bytes());
        frames.extend(bytes);
    };
    for title in 0..127 {
        event(
            3,
            json!({"change":"title","container":{"id":9,"name":title.to_string()}}),
        );
    }
    event(3, json!({"change":"focus","container":{"id":9}}));
    event(6, json!({"change":"exit"}));
    input.write_all(&frames).unwrap();
    let mut updates = Vec::new();
    pump(&mut events, &mut snapshots, |update| updates.push(update)).unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].focus, Some(9));
    assert_eq!(updates[0].titles.get(&9).map(String::as_str), Some("126"));
    assert!(updates[0].tree.is_none());
    assert!(!ipc::ready(&spy));
}

struct Client(Child);
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// Observe a real client acknowledging and committing its xdg configure. The wl_surface
// ID is read from the protocol trace instead of assuming an object allocation order.
fn committed_client_sizes(
    stderr: std::process::ChildStderr,
) -> std::sync::mpsc::Receiver<(i32, i32)> {
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut main_surface = String::new();
        let mut configured = None;
        let mut acknowledged = false;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some(args) = line.split(".get_xdg_surface(").nth(1) {
                if let Some(surface) = args.split(", ").nth(1).and_then(|s| s.strip_suffix(')')) {
                    main_surface = format!("{surface}.commit()");
                }
            } else if line.contains(" xdg_toplevel@") {
                if let Some(args) = line.split(".configure(").nth(1) {
                    let mut fields = args.split(", ");
                    configured = fields
                        .next()
                        .and_then(|w| w.parse().ok())
                        .zip(fields.next().and_then(|h| h.parse().ok()));
                    acknowledged = false;
                }
            } else if line.contains(".ack_configure(") {
                acknowledged = true;
            } else if acknowledged && !main_surface.is_empty() && line.ends_with(&main_surface) {
                if let Some(size) = configured.take() {
                    if sender.send(size).is_err() {
                        break;
                    }
                }
                acknowledged = false;
            }
        }
    });
    receiver
}

#[track_caller]
fn wait(mut predicate: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(6);
    loop {
        Dispatcher::ui_thread().run_jobs(None);
        if predicate() {
            break;
        }
        assert!(Instant::now() < end, "compositor state did not converge");
        thread::sleep(Duration::from_millis(12));
    }
}

#[test]
#[ignore = "requires private headless Sway + a native foot client"]
fn sway_native_client_lifecycle() {
    assert_eq!(
        std::env::var("FERRO_WAYLAND_PRIVATE_TEST").as_deref(),
        Ok("1")
    );
    let (socket, compositor_pid) = ipc::compositor_socket().unwrap();
    let current_pid = std::process::id() as libc::pid_t;
    assert_ne!(current_pid, compositor_pid);
    assert_eq!(
        ipc::connect(&socket, current_pid).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    let session = WaylandSession::connect().unwrap();
    let first_workspace = session.snapshot().current_workspace;
    let title = format!("Ferro IPC test {}", std::process::id());
    let mut client = Client(
        Command::new("foot")
            .current_dir("/tmp")
            .args([
                "--working-directory=/tmp",
                "-o",
                "resize-delay-ms=0",
                "--app-id=org.ferroui.IpcTest",
                "--title",
                &title,
                "sleep",
                "60",
            ])
            .env("WAYLAND_DEBUG", "client")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let commits = committed_client_sizes(client.0.stderr.take().unwrap());
    let find = || {
        session
            .snapshot()
            .windows
            .into_iter()
            .find(|w| w.title == title)
    };
    wait(|| find().is_some_and(|w| w.active));
    let id = find().unwrap().id;
    assert!(!find().unwrap().can_raise);
    let area = session.frame_and_area(id).unwrap().1;
    let wait_frame = |want: Rect| {
        let start = Instant::now();
        // A Sway workspace tree can already describe the pending rectangle before
        // the independent xdg-shell process has even received its first configure.
        while commits.recv_timeout(Duration::from_secs(4)).unwrap() != (want.width, want.height) {
            assert!(
                start.elapsed() < Duration::from_secs(4),
                "client never configured {want:?}"
            );
        }
        let mut stable = 0;
        loop {
            Dispatcher::ui_thread().run_jobs(None);
            let actual = session.frame_and_area(id).unwrap();
            // Sway exposes pending geometry before the xdg client acknowledges its
            // configure. Let consecutive independent samples confirm a settled frame.
            if actual.0.rect == want {
                stable += 1;
                if stable >= 4 {
                    break;
                }
            } else {
                stable = 0;
            }
            assert!(
                start.elapsed() < Duration::from_secs(4),
                "expected {want:?}, found {actual:?}"
            );
            thread::sleep(Duration::from_millis(16));
        }
    };
    wait_frame(area);
    assert!(session.snap(id, SnapSide::Left));
    wait_frame(area.half(false));
    assert!(session.snap(id, SnapSide::Right));
    wait_frame(area.half(true));
    assert!(session.toggle_maximized(id));
    wait_frame(area);
    assert!(session.toggle_maximized(id));
    wait_frame(area.half(true));
    assert!(session.toggle_fullscreen(id));
    wait(|| find().is_some_and(|w| w.fullscreen));
    assert!(!find().unwrap().can_snap);
    assert!(!session.snap(id, SnapSide::Right));
    assert!(session.toggle_fullscreen(id));
    wait(|| find().is_some_and(|w| !w.fullscreen));
    assert!(session.add_workspace());
    wait(|| session.snapshot().current_workspace != first_workspace);
    assert!(session.can_remove_last());
    assert!(session.remove_empty_last_workspace());
    wait(|| {
        session.snapshot().current_workspace == first_workspace
            && session.snapshot().workspace_names.len() == 1
    });
    assert!(!session.can_remove_last());
    assert!(session.add_workspace());
    wait(|| session.snapshot().current_workspace != first_workspace);
    let next = session.snapshot().current_workspace;
    assert!(session.move_to_workspace(id, next));
    wait(|| find().is_some_and(|w| w.workspace == Some(session.snapshot().current_workspace)));
    assert!(session.activate(id));
    wait(|| find().is_some_and(|w| w.active));
    assert!(session.minimize(id));
    wait(|| find().is_some_and(|w| w.minimized));
    assert!(session.activate(id));
    wait(|| find().is_some_and(|w| !w.minimized && w.active));
    assert!(session.close(id));
    wait(|| find().is_none());
}
