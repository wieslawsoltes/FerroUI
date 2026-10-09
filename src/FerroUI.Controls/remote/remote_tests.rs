//! Tests of the remote rendering controls. Not from upstream: the upstream
//! project tests them only through the previewer, by starting a process and
//! talking to it over TCP. Here a [`RemoteServer`] and the other end of its
//! connection run in the test, joined by the TCP transport of the protocol
//! over the loopback interface, so that the messages arrive on the reader
//! threads of the connections as they do in an application.
//!
//! The render interface of the tests is the mock one with render targets
//! that lock the framebuffer of their surface and fill it with a known
//! pixel ([`FILL`]): what is checked is the way of a frame from the
//! framebuffer of the server to the bitmap of the widget, not the drawing
//! of a scene.

use crate::platform::ITopLevelImpl;
use crate::remote::{RemoteServer, RemoteWidget, SizingMode};
use crate::testing::{CompositorTestServices, TestServices};
use crate::{Border, Control};
use ferroui_base::input::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType, RawTextInputEventArgs};
use ferroui_base::input::{IKeyboardDevice, Key, KeyboardDevice, PhysicalKey, RawInputModifiers};
use ferroui_base::platform::surfaces::{IFramebufferRenderTarget, IPlatformRenderSurface};
use ferroui_base::platform::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformRenderInterfaceContext,
    IRenderTarget, RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::rendering::testing::{
    DrawingLog, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockPlatformRenderInterface,
};
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::{FerroLocator, PixelSize, Point, Rect, Size, Vector};
use ferroui_remote_protocol::input::{
    InputModifiers, KeyEventMessage, MouseButton, PointerEventMessageBase, PointerPressedEventMessage,
    TextInputEventMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameMessage, FrameReceivedMessage,
    MeasureViewportMessage, PixelFormat,
};
use ferroui_remote_protocol::{
    message_handler, BsonTcpTransport, DisposableServer, IFerroRemoteTransportConnection, TcpTransportBase,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::net::{IpAddr, Ipv4Addr};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

/// The pixel the render targets of the tests fill a frame with.
const FILL: [u8; 4] = [10, 20, 30, 255];

/// A render target that locks the framebuffer of its surface for every
/// frame, fills it and unlocks it.
struct FillingRenderTarget {
    log: DrawingLog,
    target: Option<Rc<dyn IFramebufferRenderTarget>>,
}

impl IRenderTarget for FillingRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        if let Some(target) = &self.target {
            let (framebuffer, _) = target.lock(scene_info);
            framebuffer.with_data(&mut |data| {
                for pixel in data.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&FILL);
                }
            });
            framebuffer.dispose();
        }
        (Box::new(MockDrawingContextImpl::new(self.log.clone())), RenderTargetDrawingContextProperties::default())
    }

    fn dispose(&self) {
        if let Some(target) = &self.target {
            target.dispose();
        }
    }
}

struct FillingContext {
    log: DrawingLog,
}

impl IOptionalFeatureProvider for FillingContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for FillingContext {
    fn create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        let target = surfaces
            .iter()
            .find_map(|surface| surface.as_framebuffer_surface().map(|surface| surface.create_framebuffer_render_target()));
        Rc::new(FillingRenderTarget { log: self.log.clone(), target })
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        _scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(MockDrawingContextLayerImpl::new(self.log.clone(), pixel_size))
    }

    fn is_lost(&self) -> bool {
        false
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        None
    }

    fn dispose(&self) {}
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
}

impl Drop for Connections {
    fn drop(&mut self) {
        self.client.dispose();
        self.server.dispose();
        self.listener.dispose();
    }
}

/// The services of a test: a unit test application whose top-levels render
/// through the compositor into the filling render targets, with a keyboard
/// device and with the render loop the offscreen implementation of the
/// server creates its compositor on.
fn start() -> CompositorTestServices {
    let render_interface = MockPlatformRenderInterface::new(DrawingLog::new());
    render_interface.set_backend_context_factory(|log, _| Rc::new(FillingContext { log: log.clone() }));
    let mut services = TestServices::styled_window();
    services.render_interface = Some(render_interface);
    services.keyboard_device = Some(Rc::new(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)));
    let services = CompositorTestServices::start(services);
    let render_loop: Arc<dyn IRenderLoop> = services.render_loop().clone();
    FerroLocator::current_mutable().bind::<Arc<dyn IRenderLoop>>().to_constant(Rc::new(render_loop));
    services
}

/// Runs the jobs of the UI thread and renders frames until `done`.
fn pump_until(services: &CompositorTestServices, what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        services.run_jobs();
        if done() {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(2));
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

fn assert_filled(data: &[u8], width: i32, height: i32, stride: i32) {
    assert_eq!((stride * height) as usize, data.len());
    for y in 0..height as usize {
        let row = &data[y * stride as usize..y * stride as usize + width as usize * 4];
        assert!(row.chunks_exact(4).all(|pixel| pixel == FILL), "row {y} is not the rendered one");
    }
}

#[test]
fn the_server_sends_no_pixels_before_the_client_states_its_pixel_formats() {
    let services = start();
    let connections = Connections::open();
    let frames = collect_frames(&connections.client);
    let server = RemoteServer::new(connections.server.clone());

    connections.client.send(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 96.0, dpi_y: 96.0 }));
    pump_until(&services, "the allocation", || server.top_level().client_size() == Size::new(8.0, 4.0));
    // Without a format the framebuffer is the empty one: a frame that is
    // rendered meanwhile is sent without pixels, as in the original.
    assert!(frames.lock().unwrap().iter().all(|frame| (frame.width, frame.height, frame.stride) == (0, 0, 0)));

    connections.client.send(Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }));
    pump_until(&services, "the first frame with pixels", || frames.lock().unwrap().iter().any(|frame| frame.width != 0));
    {
        let frames = frames.lock().unwrap();
        let frame = frames.iter().find(|frame| frame.width != 0).unwrap();
        assert_eq!(PixelFormat::Rgba8888, frame.format);
        assert_eq!((8, 4, 32), (frame.width, frame.height, frame.stride));
        assert_eq!((96.0, 96.0), (frame.dpi_x, frame.dpi_y));
        assert_filled(frame.data.as_deref().unwrap(), 8, 4, 32);
    }

    server.dispose();
    assert!(server.platform_impl().is_disposed());
    drop(connections);
    services.dispose();
}

#[test]
fn an_allocation_of_the_viewport_resizes_the_top_level_and_the_frames() {
    let services = start();
    let connections = Connections::open();
    let frames = collect_frames(&connections.client);
    let server = RemoteServer::new(connections.server.clone());

    connections.client.send(Arc::new(ClientSupportedPixelFormatsMessage {
        formats: Some(vec![PixelFormat::Bgra8888, PixelFormat::Rgba8888]),
    }));
    connections.client.send(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 96.0, dpi_y: 96.0 }));
    pump_until(&services, "a frame of the first size", || {
        frames.lock().unwrap().iter().any(|frame| (frame.width, frame.height) == (8, 4))
    });

    // Twice the scaling and another size: the frame has the pixels of both.
    connections.client.send(Arc::new(ClientViewportAllocatedMessage {
        width: 30.0,
        height: 20.0,
        dpi_x: 192.0,
        dpi_y: 192.0,
    }));
    pump_until(&services, "a frame of the second size", || {
        frames.lock().unwrap().iter().any(|frame| (frame.width, frame.height) == (60, 40))
    });
    assert_eq!(Size::new(30.0, 20.0), server.top_level().client_size());
    assert_eq!(2.0, server.top_level().render_scaling());
    assert_eq!(2.0, server.platform_impl().render_scaling());
    {
        let frames = frames.lock().unwrap();
        let frame = frames.iter().find(|frame| frame.width == 60).unwrap();
        // The first format of the client is the format of the frames.
        assert_eq!(PixelFormat::Bgra8888, frame.format);
        assert_eq!(240, frame.stride);
        assert_eq!((192.0, 192.0), (frame.dpi_x, frame.dpi_y));
        assert_filled(frame.data.as_deref().unwrap(), 60, 40, 240);
        // The sequence numbers count up from one.
        let ids: Vec<i64> = frames.iter().map(|frame| frame.sequence_id).collect();
        assert_eq!((1..=ids.len() as i64).collect::<Vec<_>>(), ids);
    }

    server.dispose();
    drop(connections);
    services.dispose();
}

#[test]
fn a_measure_request_is_answered_with_the_desired_size_of_the_root() {
    let services = start();
    let connections = Connections::open();
    let answers = Arc::new(Mutex::new(Vec::new()));
    {
        let answers = answers.clone();
        connections.client.on_message(message_handler(move |_, message| {
            if let Some(measure) = message.downcast_ref::<MeasureViewportMessage>() {
                answers.lock().unwrap().push((measure.width, measure.height));
            }
        }));
    }
    let server = RemoteServer::new(connections.server.clone());

    // The root of the server enforces the client size of its implementation
    // (`EmbeddableControlRoot.EnforceClientSize`): whatever the constraint,
    // it wants the size of the viewport it was allocated.
    connections.client.send(Arc::new(ClientViewportAllocatedMessage { width: 100.0, height: 80.0, dpi_x: 96.0, dpi_y: 96.0 }));
    pump_until(&services, "the allocation", || server.top_level().client_size() == Size::new(100.0, 80.0));
    connections.client.send(Arc::new(MeasureViewportMessage { width: 300.0, height: 200.0 }));
    pump_until(&services, "the answer", || !answers.lock().unwrap().is_empty());
    assert_eq!(vec![(100.0, 80.0)], *answers.lock().unwrap());

    server.dispose();
    drop(connections);
    services.dispose();
}

#[test]
fn input_messages_arrive_as_raw_input_of_the_top_level() {
    let services = start();
    let connections = Connections::open();
    let server = RemoteServer::new(connections.server.clone());

    // The raw input of the top-level, recorded in front of the top-level.
    let recorded: Rc<RefCell<Vec<Rc<dyn IRawInputEventArgs>>>> = Rc::new(RefCell::new(Vec::new()));
    let platform_impl = server.platform_impl().clone();
    let input = platform_impl.base().input().expect("the top-level listens to the input of its implementation");
    {
        let recorded = recorded.clone();
        platform_impl.base().set_input(Some(Rc::new(move |args: Rc<dyn IRawInputEventArgs>| {
            recorded.borrow_mut().push(args.clone());
            input(args);
        })));
    }

    connections.client.send(Arc::new(PointerPressedEventMessage {
        base: PointerEventMessageBase {
            x: 3.0,
            y: 2.0,
            base: ferroui_remote_protocol::input::InputEventMessageBase {
                modifiers: Some(vec![InputModifiers::Control, InputModifiers::LeftMouseButton]),
            },
        },
        button: MouseButton::Left,
    }));
    connections.client.send(Arc::new(KeyEventMessage {
        is_down: true,
        key: ferroui_remote_protocol::input::Key::A,
        physical_key: ferroui_remote_protocol::input::PhysicalKey::A,
        key_symbol: Some("a".to_string()),
        ..KeyEventMessage::default()
    }));
    connections
        .client
        .send(Arc::new(TextInputEventMessage { text: "a".to_string(), ..TextInputEventMessage::default() }));
    pump_until(&services, "the input", || recorded.borrow().len() == 3);

    let recorded = recorded.borrow();
    let pointer = recorded[0].downcast_ref::<RawPointerEventArgs>().expect("a pointer event");
    assert_eq!(RawPointerEventType::LeftButtonDown, pointer.type_());
    assert_eq!(Point::new(3.0, 2.0), pointer.position());
    assert_eq!(RawInputModifiers::CONTROL | RawInputModifiers::LEFT_MOUSE_BUTTON, pointer.input_modifiers());
    assert_eq!(0, pointer.timestamp());

    let key = recorded[1].downcast_ref::<RawKeyEventArgs>().expect("a key event");
    assert_eq!(RawKeyEventType::KeyDown, key.type_());
    assert_eq!(Key::A, key.key());
    assert_eq!(PhysicalKey::A, key.physical_key());
    assert_eq!(Some("a".to_string()), key.key_symbol());
    assert_eq!(RawInputModifiers::NONE, key.modifiers());

    let text = recorded[2].downcast_ref::<RawTextInputEventArgs>().expect("a text event");
    assert_eq!("a", text.text());
    drop(recorded);

    server.dispose();
    drop(connections);
    services.dispose();
}

#[test]
fn a_frame_of_the_server_arrives_at_the_widget() {
    let services = start();
    let connections = Connections::open();
    let server = RemoteServer::new(connections.server.clone());
    let content = Border::new();
    server.set_content(Some(Control::boxed(content.clone())));
    assert!(server.content().is_some());

    // The widget arranged at 6 by 3: it asks for a viewport of its size at
    // ten times the scaling, in the first format it supports. It is not in
    // a root that is rendered: the mock render interface has no writeable
    // bitmaps, so the way of the frame into the bitmap of the widget is
    // tested with a real backend (the tests of the designer support crate).
    let widget = RemoteWidget::new(connections.client.clone());
    assert_eq!(SizingMode::Local, widget.mode());
    widget.measure(Size::new(6.0, 3.0));
    widget.arrange(Rect::new(0.0, 0.0, 6.0, 3.0));

    pump_until(&services, "a frame of the size of the widget", || {
        widget
            .last_frame()
            .and_then(|frame| frame.downcast_ref::<FrameMessage>().map(|frame| (frame.width, frame.height)))
            == Some((60, 30))
    });
    assert_eq!(Size::new(6.0, 3.0), server.top_level().client_size());
    assert_eq!(10.0, server.top_level().render_scaling());
    {
        let frame = widget.last_frame().unwrap();
        let frame = frame.downcast_ref::<FrameMessage>().unwrap();
        assert_eq!(PixelFormat::Bgra8888, frame.format);
        assert_eq!(240, frame.stride);
        assert_filled(frame.data.as_deref().unwrap(), 60, 30, 240);
    }

    // In the remote mode the widget leaves the size of the viewport to the
    // other end.
    widget.set_mode(SizingMode::Remote);
    assert_eq!(SizingMode::Remote, widget.mode());

    server.dispose();
    drop(connections);
    services.dispose();
}

#[test]
fn a_top_level_implementation_is_a_framebuffer_surface() {
    let services = start();
    let connections = Connections::open();
    let server = RemoteServer::new(connections.server.clone());
    let surfaces = server.platform_impl().surfaces();
    assert_eq!(1, surfaces.len());
    assert!(surfaces[0].as_framebuffer_surface().is_some());
    assert_eq!(1.0, server.platform_impl().desktop_scaling());
    assert!(server.platform_impl().base().handle().is_none());

    server.dispose();
    drop(connections);
    services.dispose();
}
