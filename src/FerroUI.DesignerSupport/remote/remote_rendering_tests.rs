//! A scene rendered by a remote server with the Skia backend, received as
//! the pixels of a frame at the other end of a TCP connection over the
//! loopback interface. Not from upstream: the upstream project tests the
//! remote rendering only through the previewer process.

use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::media::Brushes;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::remote::RemoteServer;
use ferroui_controls::testing::{CompositorTestServices, TestServices};
use ferroui_controls::{Border, Control};
use ferroui_remote_protocol::viewport::{
    ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage, FrameReceivedMessage, PixelFormat,
};
use ferroui_remote_protocol::{message_handler, BsonTcpTransport, TcpTransportBase};
use ferroui_skia::SkiaPlatform;
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn a_scene_rendered_by_the_server_arrives_as_the_pixels_of_a_frame() {
    let locator = FerroLocator::enter_scope();
    SkiaPlatform::initialize();
    let render_interface = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    let mut services = TestServices::styled_window().with_render_interface(render_interface);
    services.keyboard_device = Some(Rc::new(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)));
    let services = CompositorTestServices::start(services);
    // The offscreen implementation of the server creates its compositor on
    // the render loop of the services.
    let render_loop: Arc<dyn IRenderLoop> = services.render_loop().clone();
    FerroLocator::current_mutable().bind::<Arc<dyn IRenderLoop>>().to_constant(Rc::new(render_loop));

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
    let server_connection = accepted.recv_timeout(TIMEOUT).expect("the listener hands out the connection");

    // The client: it keeps the frames and acknowledges each.
    let frames = Arc::new(Mutex::new(Vec::<FrameMessage>::new()));
    {
        let frames = frames.clone();
        client.on_message(message_handler(move |connection, message| {
            if let Some(frame) = message.downcast_ref::<FrameMessage>() {
                frames.lock().unwrap().push(frame.clone());
                connection.send(Arc::new(FrameReceivedMessage { sequence_id: frame.sequence_id }));
            }
        }));
    }

    // The scene: a red border that fills the top-level.
    let server = RemoteServer::new(server_connection.clone());
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    server.set_content(Some(Control::boxed(border)));

    client.send(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
    client.send(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 192.0, dpi_y: 192.0 }));

    // A frame of the size of the viewport whose pixels are those of the
    // scene: the first frames may be rendered before the content is laid
    // out.
    let is_red = |frame: &FrameMessage| {
        (frame.width, frame.height) == (16, 8)
            && frame.data.as_deref().is_some_and(|data| data.chunks_exact(4).all(|pixel| pixel == [255, 0, 0, 255]))
    };
    let deadline = Instant::now() + TIMEOUT;
    loop {
        services.run_jobs();
        if frames.lock().unwrap().iter().any(is_red) {
            break;
        }
        assert!(Instant::now() < deadline, "timed out waiting for the frame of the scene");
        std::thread::sleep(Duration::from_millis(2));
    }
    {
        let frames = frames.lock().unwrap();
        let frame = frames.iter().find(|frame| is_red(frame)).unwrap();
        assert_eq!(PixelFormat::Rgba8888, frame.format);
        assert_eq!(64, frame.stride);
        assert_eq!((192.0, 192.0), (frame.dpi_x, frame.dpi_y));
        assert_eq!(Some(64 * 8), frame.data.as_ref().map(Vec::len));
    }

    server.dispose();
    client.dispose();
    server_connection.dispose();
    listener.dispose();
    services.dispose();
    locator.dispose();
}
