//! Scenes rendered by a remote server with the Skia backend, received as
//! the pixels of frames at the other end of a TCP connection over the
//! loopback interface. Not from upstream: the upstream project tests the
//! remote rendering only through the previewer process.

use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::media::Brushes;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::{FerroLocator, LocatorExtensions};
use crate::remote::test_connection::TestConnection;
use ferroui_controls::remote::{RemoteServer, RemoteWidget};
use ferroui_controls::testing::{CompositorTestServices, TestServices};
use ferroui_controls::{Border, Control};
use ferroui_remote_protocol::viewport::{
    ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage, FrameReceivedMessage, PixelFormat,
};
use ferroui_remote_protocol::{
    message_handler, BsonTcpTransport, DisposableServer, IFerroRemoteTransportConnection, TcpTransportBase,
};
use ferroui_skia::SkiaPlatform;
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

const RED: [u8; 4] = [255, 0, 0, 255];

/// A unit test application that renders with Skia through the compositor.
pub(crate) struct Services {
    services: CompositorTestServices,
    locator: Rc<dyn IDisposable>,
}

impl Services {
    pub(crate) fn start() -> Services {
        let locator = FerroLocator::enter_scope();
        SkiaPlatform::initialize();
        let render_interface = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
        let mut services = TestServices::styled_window().with_render_interface(render_interface);
        services.keyboard_device = Some(Rc::new(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)));
        let services = CompositorTestServices::start(services);
        // The offscreen implementation of a server creates its compositor
        // on the render loop of the services.
        let render_loop: Arc<dyn IRenderLoop> = services.render_loop().clone();
        FerroLocator::current_mutable().bind::<Arc<dyn IRenderLoop>>().to_constant(Rc::new(render_loop));
        Services { services, locator }
    }

    /// Runs the jobs of the UI thread and renders frames until `done`.
    pub(crate) fn pump_until(&self, what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            self.services.run_jobs();
            if done() {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Runs the jobs of the UI thread and renders a frame, `times` times.
    fn pump(&self, times: usize) {
        for _ in 0..times {
            self.services.run_jobs();
        }
    }

    pub(crate) fn end(self) {
        self.services.dispose();
        self.locator.dispose();
    }
}

/// The two ends of a connection over the loopback interface.
struct Connections {
    server: Arc<dyn IFerroRemoteTransportConnection>,
    client: Arc<dyn IFerroRemoteTransportConnection>,
    listener: DisposableServer,
}

impl Connections {
    fn open() -> Connections {
        let transport = BsonTcpTransport::empty();
        let (sender, accepted) = channel();
        let sender = Mutex::new(sender);
        let listener = transport
            .listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 0, move |connection| {
                let _ = sender.lock().unwrap().send(connection);
            })
            .expect("the loopback interface can be listened on");
        let client = transport
            .connect(IpAddr::V4(Ipv4Addr::LOCALHOST), listener.local_addr().port())
            .expect("the listener accepts a connection");
        let server = accepted.recv_timeout(TIMEOUT).expect("the listener hands out the connection");
        Connections { server, client, listener }
    }

    fn close(self) {
        self.client.dispose();
        self.server.dispose();
        self.listener.dispose();
    }
}

/// The frames a client receives, each acknowledged as the widget does.
fn collect_frames(client: &Arc<dyn IFerroRemoteTransportConnection>) -> Arc<Mutex<Vec<FrameMessage>>> {
    let frames = Arc::new(Mutex::new(Vec::new()));
    let received = frames.clone();
    client.on_message(message_handler(move |connection, message| {
        if let Some(frame) = message.downcast_ref::<FrameMessage>() {
            received.lock().unwrap().push(frame.clone());
            connection.send(Arc::new(FrameReceivedMessage { sequence_id: frame.sequence_id }));
        }
    }));
    frames
}

/// Whether the frame has the given size and every pixel of it is red.
pub(crate) fn is_red(frame: &FrameMessage, width: i32, height: i32) -> bool {
    (frame.width, frame.height) == (width, height)
        && frame.data.as_deref().is_some_and(|data| data.chunks_exact(4).all(|pixel| pixel == RED))
}

/// A server whose scene is a red border that fills the top-level.
pub(crate) fn red_server(connection: &Arc<dyn IFerroRemoteTransportConnection>) -> RemoteServer {
    let server = RemoteServer::new(connection.clone());
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    server.set_content(Some(Control::boxed(border)));
    server
}

#[test]
fn a_scene_rendered_by_the_server_arrives_as_the_pixels_of_a_frame() {
    let services = Services::start();
    let connections = Connections::open();
    let frames = collect_frames(&connections.client);
    let server = red_server(&connections.server);

    connections.client.send(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
    connections.client.send(Arc::new(ClientViewportAllocatedMessage {
        width: 8.0,
        height: 4.0,
        dpi_x: 192.0,
        dpi_y: 192.0,
    }));

    // A frame of the size of the viewport whose pixels are those of the
    // scene: the first frames may be rendered before the content is laid
    // out.
    services.pump_until("the frame of the scene", || frames.lock().unwrap().iter().any(|frame| is_red(frame, 16, 8)));
    {
        let frames = frames.lock().unwrap();
        let frame = frames.iter().find(|frame| is_red(frame, 16, 8)).unwrap();
        assert_eq!(PixelFormat::Rgba8888, frame.format);
        assert_eq!(64, frame.stride);
        assert_eq!((192.0, 192.0), (frame.dpi_x, frame.dpi_y));
        assert_eq!(Some(64 * 8), frame.data.as_ref().map(Vec::len));
    }

    server.dispose();
    connections.close();
    services.end();
}

/// The frames a server has sent over a connection of the tests.
fn frames_of(connection: &TestConnection) -> Vec<FrameMessage> {
    connection.sent_of::<FrameMessage>()
}

// The compositor renders a top-level from the moment the server starts it,
// and the messages of the client arrive when they arrive: without a pixel
// format or without a viewport the framebuffer has no pixels, and Skia
// cannot create a surface over it. The messages are raised by the test, so
// that frames are rendered in each of those states.
#[test]
fn nothing_is_rendered_while_the_framebuffer_of_the_server_has_no_pixels() {
    let services = Services::start();
    let connection = TestConnection::new();
    let transport: Arc<dyn IFerroRemoteTransportConnection> = connection.clone();
    let server = red_server(&transport);

    // Nothing from the client yet.
    services.pump(5);
    assert!(frames_of(&connection).is_empty());

    // The pixel formats without a viewport.
    connection.raise_message(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
    services.pump(5);
    assert!(frames_of(&connection).is_empty());

    // The viewport: the scene is rendered.
    connection.raise_message(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 96.0, dpi_y: 96.0 }));
    services.pump_until("the frame of the scene", || frames_of(&connection).iter().any(|frame| is_red(frame, 8, 4)));
    let sent = frames_of(&connection);
    assert!(sent.iter().all(|frame| frame.width != 0 && frame.height != 0));

    // A viewport without pixels again: no frame without pixels is rendered,
    // whatever is acknowledged.
    connection.raise_message(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 0.0, dpi_x: 96.0, dpi_y: 96.0 }));
    for _ in 0..3 {
        for frame in frames_of(&connection) {
            connection.raise_message(Arc::new(FrameReceivedMessage { sequence_id: frame.sequence_id }));
        }
        services.pump(5);
    }
    assert!(frames_of(&connection).iter().all(|frame| frame.width != 0 && frame.height != 0));

    server.dispose();
    services.end();
}

/// The way of a frame through the widget: the scene of one server is shown
/// by a widget, which is the scene of a second server, whose frames are
/// looked at. The widget copies the frame it receives into its bitmap and
/// draws the bitmap over its bounds, so the frames of the second server
/// have the pixels of the scene of the first.
#[test]
fn a_frame_of_the_server_is_drawn_by_the_widget() {
    let services = Services::start();
    let scene = Connections::open();
    let scene_server = red_server(&scene.server);

    let shown = Connections::open();
    let frames = collect_frames(&shown.client);
    let widget_server = RemoteServer::new(shown.server.clone());
    // The widget states its pixel formats and, when it is arranged, the
    // size of its viewport, at ten times the scaling.
    let widget = RemoteWidget::new(scene.client.clone());
    widget_server.set_content(Some(Control::boxed(widget.clone())));

    shown.client.send(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
    shown.client.send(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 96.0, dpi_y: 96.0 }));

    services.pump_until("the frame that shows the widget with the scene", || {
        frames.lock().unwrap().iter().any(|frame| is_red(frame, 8, 4))
    });

    widget_server.dispose();
    scene_server.dispose();
    shown.close();
    scene.close();
    services.end();
}
