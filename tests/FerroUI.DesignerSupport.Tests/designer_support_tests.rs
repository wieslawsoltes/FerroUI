//! Port of upstream's `DesignerSupportTests`: a previewer process is started
//! with the arguments an IDE gives it, connects to the test over the BSON TCP
//! transport, receives the text of the main window of an application and
//! answers whether it could preview it.
//!
//! The application is the host of its own previewer in the port
//! (DEVIATIONS.md, Designer support): the rows of the test application start
//! its binary (`test_app/program.rs`), the rows of the ControlCatalog sample
//! the previewer of the catalog (`catalog_host.rs`), where upstream starts its
//! host application with the assembly of the application. The assembly path of
//! the update message names the assembly of the application: the previewer
//! takes the registered assembly of that name as the local assembly of the
//! document.

use ferroui_remote_protocol::designer::{StartDesignerSessionMessage, UpdateXamlMessage, UpdateXamlResultMessage};
use ferroui_remote_protocol::viewport::{ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, PixelFormat};
use ferroui_remote_protocol::{
    message_handler, BsonTcpTransport, Guid, IFerroRemoteTransportConnection, Message, TcpTransportBase,
};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// `Guid.NewGuid()`: a random identifier of version four.
fn new_guid() -> Guid {
    Guid::from(uuid::Uuid::new_v4())
}

/// The root of the repository.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What the result message reported.
#[derive(Default)]
struct Outcome {
    handle: i64,
    success: bool,
    error: Option<String>,
}

/// The theory of upstream: previews `xaml_file` of the application whose
/// previewer is `executable` and whose assembly is `assembly_name`, with the
/// method `method`.
fn designer_in_win32_mode_should_provide_valid_hwnd(
    executable: &str,
    assembly_name: &str,
    xaml_file: &Path,
    method: &'static str,
) {
    let xaml = std::fs::read_to_string(xaml_file).expect("the document of the main window");

    let executable = Path::new(executable);
    assert!(executable.exists(), "executable.exists()");
    let output_dir = executable.parent().expect("the directory of the executable");
    let assembly_path = output_dir.join(assembly_name).to_string_lossy().into_owned();

    let session_id = new_guid();
    let outcome = Arc::new(Mutex::new(Outcome::default()));
    let (result_message_received, received) = mpsc::channel::<()>();
    let result_message_received = Mutex::new(result_message_received);

    // The connections the transport accepted: a connection lives as long as
    // a handle to it (the original keeps it alive through its handlers).
    let connections: Arc<Mutex<Vec<Arc<dyn IFerroRemoteTransportConnection>>>> = Arc::new(Mutex::new(Vec::new()));
    let accepted = connections.clone();

    let transport = BsonTcpTransport::empty();
    let handler_outcome = outcome.clone();
    let server = transport
        .listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 0, move |conn: Arc<dyn IFerroRemoteTransportConnection>| {
            let outcome = handler_outcome.clone();
            let result_message_received = Mutex::new(result_message_received.lock().unwrap().clone());
            let assembly_path = assembly_path.clone();
            let xaml = xaml.clone();
            conn.on_message(message_handler(move |conn, msg: &Message| {
                if let Some(start) = msg.downcast_ref::<StartDesignerSessionMessage>() {
                    assert_eq!(
                        session_id,
                        Guid::parse(start.session_id.as_deref().unwrap_or_default()).expect("the session")
                    );
                    if method == "ferroui-remote" {
                        let _ = conn.send(Arc::new(ClientSupportedPixelFormatsMessage {
                            formats: Some(vec![PixelFormat::Rgba8888]),
                        }));
                        let _ = conn.send(Arc::new(ClientViewportAllocatedMessage {
                            dpi_x: 96.0,
                            dpi_y: 96.0,
                            width: 1024.0,
                            height: 768.0,
                        }));
                    }

                    let _ = conn.send(Arc::new(UpdateXamlMessage {
                        assembly_path: Some(assembly_path.clone()),
                        xaml: Some(xaml.clone()),
                        ..UpdateXamlMessage::default()
                    }));
                } else if let Some(result) = msg.downcast_ref::<UpdateXamlResultMessage>() {
                    {
                        let mut outcome = outcome.lock().unwrap();
                        if let Some(error) = &result.error {
                            outcome.error = Some(error.clone());
                            println!("{error}");
                        } else {
                            outcome.success = true;
                        }
                        if method == "win32" {
                            outcome.handle =
                                result.handle.as_deref().map(|handle| handle.parse().expect("a handle")).unwrap_or(0);
                        }
                    }
                    let _ = result_message_received.lock().unwrap().send(());
                    conn.dispose();
                }
            }));
            accepted.lock().unwrap().push(conn);
        })
        .expect("the transport listens");
    let port = server.local_addr().port();

    let mut proc = Command::new(executable)
        .args(["--transport", &format!("tcp-bson://127.0.0.1:{port}/")])
        .args(["--session-id", &session_id.to_string()])
        .args(["--method", method])
        .arg(executable)
        .current_dir(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the previewer starts");

    // Deviation (DEVIATIONS.md, Tests and test support): upstream waits ten seconds. The
    // previewer of the catalog takes longer than that to load and show its main window in a
    // build without optimisation; the wait ends as soon as the result arrives.
    let cancelled = received.recv_timeout(Duration::from_secs(60)).is_ok();

    //
    let _ = proc.kill();

    let _ = proc.wait();
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut out) = proc.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(mut err) = proc.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    server.dispose();
    drop(std::mem::take(&mut *connections.lock().unwrap()));

    let outcome = outcome.lock().unwrap();
    assert!(cancelled, "Message Not Received.\n{stderr}\n{stdout}");
    assert!(outcome.success, "{}", outcome.error.as_deref().unwrap_or_default());
    if method == "win32" {
        assert_ne!(0, outcome.handle);
    }
}

fn test_app() -> (&'static str, &'static str, PathBuf) {
    (
        env!("CARGO_BIN_EXE_designer-support-test-app"),
        "FerroUI.DesignerSupport.TestApp",
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test_app/MainWindow.xaml"),
    )
}

fn control_catalog() -> (&'static str, &'static str, PathBuf) {
    (
        env!("CARGO_BIN_EXE_control-catalog-designer"),
        "ControlCatalog",
        repository_root().join("samples/ControlCatalog/MainWindow.xaml"),
    )
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows-only: the win32 method shows the document in a window of the Win32 platform")]
fn designer_in_win32_mode_should_provide_valid_hwnd_test_app_win32() {
    let (executable, assembly_name, xaml_file) = test_app();
    designer_in_win32_mode_should_provide_valid_hwnd(executable, assembly_name, &xaml_file, "win32");
}

#[test]
#[cfg_attr(not(windows), ignore = "Windows-only: the win32 method shows the document in a window of the Win32 platform")]
fn designer_in_win32_mode_should_provide_valid_hwnd_control_catalog_win32() {
    let (executable, assembly_name, xaml_file) = control_catalog();
    designer_in_win32_mode_should_provide_valid_hwnd(executable, assembly_name, &xaml_file, "win32");
}

#[test]
fn designer_in_win32_mode_should_provide_valid_hwnd_test_app_ferroui_remote() {
    let (executable, assembly_name, xaml_file) = test_app();
    designer_in_win32_mode_should_provide_valid_hwnd(executable, assembly_name, &xaml_file, "ferroui-remote");
}

#[test]
fn designer_in_win32_mode_should_provide_valid_hwnd_control_catalog_ferroui_remote() {
    let (executable, assembly_name, xaml_file) = control_catalog();
    designer_in_win32_mode_should_provide_valid_hwnd(executable, assembly_name, &xaml_file, "ferroui-remote");
}
