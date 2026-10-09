use super::remote_server_top_level_impl_framebuffer::{FrameStatus, Framebuffer};
use crate::embedding::offscreen::{OffscreenTopLevelImplBase, OffscreenTopLevelImplOverrides};
use crate::platform::ITopLevelImpl;
use crate::remote::UiThreadHandle;
use ferroui_base::input::raw::{
    IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
    RawPointerEventType, RawTextInputEventArgs,
};
use ferroui_base::input::{
    IInputDevice, IInputRoot, IKeyboardDevice, IMouseDevice, Key, KeyDeviceType, MouseDevice, PhysicalKey,
    RawInputModifiers,
};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::threading::DispatcherPriority;
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Rect, Size, Vector};
use ferroui_remote_protocol::input::{
    InputModifiers, KeyEventMessage, MouseButton as ProtocolMouseButton, PointerMovedEventMessage,
    PointerPressedEventMessage, PointerReleasedEventMessage, ScrollEventMessage, TextInputEventMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientRenderInfoMessage, ClientSupportedPixelFormatsMessage, ClientViewportAllocatedMessage, FrameReceivedMessage,
    MeasureViewportMessage, PixelFormat as ProtocolPixelFormat,
};
use ferroui_remote_protocol::{message_handler, IFerroRemoteTransportConnection, Message};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak as ArcWeak};

/// What a handler on the reader thread of the connection has of the
/// implementation: its registration on the UI thread, through which an
/// action is posted to it.
pub type RemoteServerUiThreadHandle = UiThreadHandle<Weak<RemoteServerTopLevelImpl>>;

/// The override of `OnMessage` of a deriving class. It runs on the reader
/// thread of the connection, before the handling of this class, and returns
/// whether it handled the message (the override of the original returns
/// without calling the base).
pub type OnMessageOverride =
    Arc<dyn Fn(&RemoteServerUiThreadHandle, &dyn IFerroRemoteTransportConnection, &Message) -> bool + Send + Sync>;

/// The overridable members of [`RemoteServerTopLevelImpl`] that run on the
/// UI thread: what a class deriving from it provides. (`OnMessage` runs on
/// the reader thread and is an [`OnMessageOverride`].)
///
/// This API is unstable.
pub trait RemoteServerTopLevelImplOverrides {
    /// `Measure`: the size the root element wants within `constraint`.
    fn measure(&self, base: &RemoteServerTopLevelImpl, constraint: Size) -> Size {
        base.base_measure(constraint)
    }

    /// `DesktopScaling`; the render scaling by default.
    fn desktop_scaling(&self, base: &RemoteServerTopLevelImpl) -> f64 {
        base.render_scaling()
    }

    /// `TryGetFeature`; none by default.
    fn try_get_feature(&self, _base: &RemoteServerTopLevelImpl, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

/// The fields the original guards with `_lock`, and the values of the
/// offscreen implementation that the thread which renders reads.
struct State {
    framebuffer: Arc<Framebuffer>,
    last_sent_frame: i64,
    last_received_frame: i64,
    next_frame_number: i64,
    pending_allocation: Option<ClientViewportAllocatedMessage>,
    format: Option<ProtocolPixelFormat>,
    client_size: Size,
    render_scaling: f64,
    is_disposed: bool,
}

/// The part of the implementation that the reader thread of the connection
/// and the thread that renders use: the handling of the messages, the frame
/// counters and the framebuffer surface.
///
/// Deviation (DEVIATIONS.md, Remote rendering): the original is one object,
/// which is the top-level implementation, the framebuffer surface and the
/// handler of the connection. Here the top-level implementation is an
/// object of the UI thread, so what the other threads use lives in this
/// object, which the threads share, as the surface of a headless window
/// does.
struct RemoteServerSurface {
    this: ArcWeak<RemoteServerSurface>,
    transport: Arc<dyn IFerroRemoteTransportConnection>,
    lock: Mutex<State>,
    handle: RemoteServerUiThreadHandle,
    on_message_override: Option<OnMessageOverride>,
}

/// A top-level implementation that is rendered into frames which are sent
/// over a connection of the remote protocol, and that takes its size, its
/// scaling, its pixel format and its input from the messages of the other
/// end.
///
/// This API is unstable.
pub struct RemoteServerTopLevelImpl {
    base: Rc<OffscreenTopLevelImplBase>,
    overrides: Option<Rc<dyn RemoteServerTopLevelImplOverrides>>,
    surface: Arc<RemoteServerSurface>,
    mouse_device: Rc<MouseDevice>,
    keyboard_device: Rc<dyn IKeyboardDevice>,
}

/// The members of the offscreen implementation this class overrides.
struct OffscreenOverrides {
    owner: Weak<RemoteServerTopLevelImpl>,
    surface: Arc<RemoteServerSurface>,
    mouse_device: Rc<MouseDevice>,
}

impl OffscreenTopLevelImplOverrides for OffscreenOverrides {
    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        let surface: Arc<dyn IPlatformRenderSurface> = self.surface.clone();
        vec![surface]
    }

    fn mouse_device(&self) -> Rc<dyn IMouseDevice> {
        self.mouse_device.clone()
    }

    fn dispose(&self, base: &OffscreenTopLevelImplBase) {
        self.surface.state().is_disposed = true;
        self.surface.handle.unregister();
        base.base_dispose();
    }

    fn desktop_scaling(&self, base: &OffscreenTopLevelImplBase) -> f64 {
        match self.owner.upgrade() {
            Some(owner) => owner.desktop_scaling(),
            None => base.render_scaling(),
        }
    }

    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        let owner = self.owner.upgrade()?;
        owner.overrides.as_ref()?.try_get_feature(&owner, feature_type)
    }
}

fn get_ferro_event_type(button: ProtocolMouseButton, pressed: bool) -> RawPointerEventType {
    match button {
        ProtocolMouseButton::Left => {
            if pressed {
                RawPointerEventType::LeftButtonDown
            } else {
                RawPointerEventType::LeftButtonUp
            }
        }

        ProtocolMouseButton::Middle => {
            if pressed {
                RawPointerEventType::MiddleButtonDown
            } else {
                RawPointerEventType::MiddleButtonUp
            }
        }

        ProtocolMouseButton::Right => {
            if pressed {
                RawPointerEventType::RightButtonDown
            } else {
                RawPointerEventType::RightButtonUp
            }
        }

        _ => RawPointerEventType::Move,
    }
}

fn get_ferro_raw_input_modifiers(modifiers: Option<&[InputModifiers]>) -> RawInputModifiers {
    let mut result = RawInputModifiers::NONE;

    let Some(modifiers) = modifiers else {
        return result;
    };

    for modifier in modifiers {
        match modifier {
            InputModifiers::Control => result |= RawInputModifiers::CONTROL,

            InputModifiers::Alt => result |= RawInputModifiers::ALT,

            InputModifiers::Shift => result |= RawInputModifiers::SHIFT,

            InputModifiers::Windows => result |= RawInputModifiers::META,

            InputModifiers::LeftMouseButton => result |= RawInputModifiers::LEFT_MOUSE_BUTTON,

            InputModifiers::MiddleMouseButton => result |= RawInputModifiers::MIDDLE_MOUSE_BUTTON,

            InputModifiers::RightMouseButton => result |= RawInputModifiers::RIGHT_MOUSE_BUTTON,
        }
    }

    result
}

fn try_get_valid_pixel_format(formats: Option<&[ProtocolPixelFormat]>) -> Option<ProtocolPixelFormat> {
    if let Some(formats) = formats {
        for format in formats {
            let value = *format as i32;
            if value >= 0 && value <= ProtocolPixelFormat::MaxValue as i32 {
                return Some(*format);
            }
        }
    }

    None
}

impl RemoteServerSurface {
    fn state(&self) -> MutexGuard<'_, State> {
        self.lock.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `OnMessage`: runs on the reader thread of the connection. What a
    /// message asks of the top-level is posted to the UI thread with the
    /// data of the message; nothing of the UI thread is touched here.
    fn on_message(&self, transport: &dyn IFerroRemoteTransportConnection, obj: &Message) {
        if let Some(on_message_override) = &self.on_message_override {
            if on_message_override(&self.handle, transport, obj) {
                return;
            }
        }

        let mut state = self.state();

        if let Some(last_frame) = obj.downcast_ref::<FrameReceivedMessage>() {
            state.last_received_frame = last_frame.sequence_id.max(state.last_received_frame);
            self.handle.post(|this| this.with(|this| this.surface.send_last_frame_if_needed()), DispatcherPriority::DEFAULT);
        } else if let Some(render_info) = obj.downcast_ref::<ClientRenderInfoMessage>() {
            let dpi_x = render_info.dpi_x;
            self.handle.post(
                move |this| {
                    this.with(|this| {
                        this.set_render_scaling(dpi_x / 96.0);
                        this.render_and_send_frame_if_needed();
                    })
                },
                DispatcherPriority::DEFAULT,
            );
        } else if let Some(supported_formats) = obj.downcast_ref::<ClientSupportedPixelFormatsMessage>() {
            state.format = try_get_valid_pixel_format(supported_formats.formats.as_deref());
            self.handle.post(|this| this.with(|this| this.render_and_send_frame_if_needed()), DispatcherPriority::DEFAULT);
        } else if let Some(measure) = obj.downcast_ref::<MeasureViewportMessage>() {
            let constraint = Size::new(measure.width, measure.height);
            self.handle.post(
                move |this| {
                    this.with(|this| {
                        let m = this.measure(constraint);
                        this.surface.transport.send(Arc::new(MeasureViewportMessage { width: m.width, height: m.height }));
                    })
                },
                DispatcherPriority::DEFAULT,
            );
        } else if let Some(allocated) = obj.downcast_ref::<ClientViewportAllocatedMessage>() {
            if state.pending_allocation.is_none() {
                self.handle.post(
                    |this| {
                        this.with(|this| {
                            let allocation = this.surface.state().pending_allocation.take();
                            let Some(allocation) = allocation else {
                                return;
                            };

                            this.set_render_scaling(allocation.dpi_x / 96.0);
                            this.set_client_size(Size::new(allocation.width, allocation.height));
                            this.render_and_send_frame_if_needed();
                        })
                    },
                    DispatcherPriority::DEFAULT,
                );
            }

            state.pending_allocation = Some(allocated.clone());
        } else if let Some(pointer) = obj.downcast_ref::<PointerMovedEventMessage>() {
            let position = Point::new(pointer.x, pointer.y);
            let modifiers = get_ferro_raw_input_modifiers(pointer.modifiers.as_deref());
            self.handle.post(
                move |this| this.with(|this| this.raise_pointer(RawPointerEventType::Move, position, modifiers)),
                DispatcherPriority::INPUT,
            );
        } else if let Some(pressed) = obj.downcast_ref::<PointerPressedEventMessage>() {
            let type_ = get_ferro_event_type(pressed.button, true);
            let position = Point::new(pressed.x, pressed.y);
            let modifiers = get_ferro_raw_input_modifiers(pressed.modifiers.as_deref());
            self.handle.post(
                move |this| this.with(|this| this.raise_pointer(type_, position, modifiers)),
                DispatcherPriority::INPUT,
            );
        } else if let Some(released) = obj.downcast_ref::<PointerReleasedEventMessage>() {
            let type_ = get_ferro_event_type(released.button, false);
            let position = Point::new(released.x, released.y);
            let modifiers = get_ferro_raw_input_modifiers(released.modifiers.as_deref());
            self.handle.post(
                move |this| this.with(|this| this.raise_pointer(type_, position, modifiers)),
                DispatcherPriority::INPUT,
            );
        } else if let Some(scroll) = obj.downcast_ref::<ScrollEventMessage>() {
            let position = Point::new(scroll.x, scroll.y);
            let delta = Vector::new(scroll.delta_x, scroll.delta_y);
            let modifiers = get_ferro_raw_input_modifiers(scroll.modifiers.as_deref());
            self.handle.post(
                move |this| {
                    this.with(|this| {
                        let Some(input) = this.base.input() else {
                            return;
                        };
                        input(Rc::new(RawMouseWheelEventArgs::new(
                            this.mouse_input_device(),
                            0,
                            this.required_input_root(),
                            position,
                            delta,
                            modifiers,
                        )));
                    })
                },
                DispatcherPriority::INPUT,
            );
        } else if let Some(key) = obj.downcast_ref::<KeyEventMessage>() {
            let key = key.clone();
            self.handle.post(
                move |this| {
                    this.with(|this| {
                        this.run_jobs_above_input();

                        let Some(input) = this.base.input() else {
                            return;
                        };
                        // `(Key)key.Key`: the two enumerations are compiled
                        // from the same source and have the same numbers.
                        input(Rc::new(RawKeyEventArgs::new(
                            this.keyboard_input_device(),
                            0,
                            this.required_input_root(),
                            if key.is_down { RawKeyEventType::KeyDown } else { RawKeyEventType::KeyUp },
                            Key::from_value(key.key as i32).unwrap_or(Key::None),
                            get_ferro_raw_input_modifiers(key.modifiers.as_deref()),
                            PhysicalKey::from_value(key.physical_key as i32).unwrap_or(PhysicalKey::None),
                            key.key_symbol.clone(),
                            KeyDeviceType::Keyboard,
                        )));
                    })
                },
                DispatcherPriority::INPUT,
            );
        } else if let Some(text) = obj.downcast_ref::<TextInputEventMessage>() {
            let text = text.text.clone();
            self.handle.post(
                move |this| {
                    this.with(|this| {
                        this.run_jobs_above_input();

                        let Some(input) = this.base.input() else {
                            return;
                        };
                        input(Rc::new(RawTextInputEventArgs::new(
                            this.keyboard_input_device(),
                            0,
                            this.required_input_root(),
                            text,
                        )));
                    })
                },
                DispatcherPriority::INPUT,
            );
        }
    }

    fn get_or_create_framebuffer(&self) -> Arc<Framebuffer> {
        let mut state = self.state();

        match state.format {
            // The empty framebuffer of this top-level: kept while it is one.
            None => {
                if state.framebuffer.stride() > 0 {
                    state.framebuffer = Framebuffer::empty();
                }
            }
            Some(format) => {
                if state.framebuffer.format() != format
                    || state.framebuffer.client_size() != state.client_size
                    || state.framebuffer.render_scaling() != state.render_scaling
                {
                    state.framebuffer = Arc::new(Framebuffer::new(format, state.client_size, state.render_scaling));
                }
            }
        }

        state.framebuffer.clone()
    }

    /// `SendLastFrameIfNeeded`: called on the UI thread and, when a frame is
    /// unlocked, on the thread that rendered it.
    fn send_last_frame_if_needed(&self) {
        let framebuffer;
        let sequence_id;

        {
            let mut state = self.state();

            if state.is_disposed {
                return;
            }

            if state.last_received_frame != state.last_sent_frame || state.framebuffer.get_status() != FrameStatus::Rendered
            {
                return;
            }

            framebuffer = state.framebuffer.clone();
            state.last_sent_frame = state.next_frame_number;
            state.next_frame_number += 1;
            sequence_id = state.last_sent_frame;
        }

        self.transport.send(Arc::new(framebuffer.to_message(sequence_id)));
    }
}

impl IPlatformRenderSurface for RemoteServerSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for RemoteServerSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let surface = self.this.upgrade().expect("the surface is alive while it is used");
        Rc::new(FuncFramebufferRenderTarget::new(move || {
            let unlocked = surface.clone();
            surface.get_or_create_framebuffer().lock(Box::new(move || unlocked.send_last_frame_if_needed()))
        }))
    }
}

/// `this` of a posted action: the implementation, if it is still alive.
trait PostedTarget {
    fn with(&self, action: impl FnOnce(&Rc<RemoteServerTopLevelImpl>));
}

impl PostedTarget for Weak<RemoteServerTopLevelImpl> {
    fn with(&self, action: impl FnOnce(&Rc<RemoteServerTopLevelImpl>)) {
        if let Some(this) = self.upgrade() {
            action(&this);
        }
    }
}

impl RemoteServerTopLevelImpl {
    /// `new RemoteServerTopLevelImpl(transport)`.
    ///
    /// # Panics
    /// Panics when no keyboard device is registered.
    pub fn new(transport: Arc<dyn IFerroRemoteTransportConnection>) -> Rc<RemoteServerTopLevelImpl> {
        Self::with_overrides(transport, None, None)
    }

    /// The constructor a deriving class calls, with its overrides.
    ///
    /// # Panics
    /// Panics when no keyboard device is registered.
    pub fn with_overrides(
        transport: Arc<dyn IFerroRemoteTransportConnection>,
        overrides: Option<Rc<dyn RemoteServerTopLevelImplOverrides>>,
        on_message_override: Option<OnMessageOverride>,
    ) -> Rc<RemoteServerTopLevelImpl> {
        let keyboard_device = FerroLocator::current().get_required_service::<dyn IKeyboardDevice>();
        let mouse_device = MouseDevice::new();

        let this = Rc::new_cyclic(|this: &Weak<RemoteServerTopLevelImpl>| {
            let handle = UiThreadHandle::register(this.clone(), |this| this.strong_count() > 0);
            let surface = Arc::new_cyclic(|surface: &ArcWeak<RemoteServerSurface>| RemoteServerSurface {
                this: surface.clone(),
                transport: transport.clone(),
                lock: Mutex::new(State {
                    framebuffer: Framebuffer::empty(),
                    last_sent_frame: -1,
                    last_received_frame: -1,
                    next_frame_number: 1,
                    pending_allocation: None,
                    format: None,
                    client_size: Size::default(),
                    render_scaling: 1.0,
                    is_disposed: false,
                }),
                handle,
                on_message_override,
            });
            let base = OffscreenTopLevelImplBase::new(Rc::new(OffscreenOverrides {
                owner: this.clone(),
                surface: surface.clone(),
                mouse_device: mouse_device.clone(),
            }));
            RemoteServerTopLevelImpl { base, overrides, surface, mouse_device, keyboard_device }
        });

        // `_transport.OnMessage += OnMessage`. The handler holds the shared
        // part weakly: the connection outlives a top-level in the previewer
        // and the original never removes the handler.
        let surface = Arc::downgrade(&this.surface);
        transport.on_message(message_handler(move |transport, obj| {
            if let Some(surface) = surface.upgrade() {
                surface.on_message(transport, obj);
            }
        }));

        this
    }

    /// The offscreen implementation this class derives from: what a
    /// top-level is created over.
    pub fn base(&self) -> &Rc<OffscreenTopLevelImplBase> {
        &self.base
    }

    /// The implementation as the contract of a top-level.
    pub fn as_top_level_impl(&self) -> Rc<dyn ITopLevelImpl> {
        self.base.clone()
    }

    /// `IsDisposed`.
    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// `InputRoot`.
    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.base.input_root()
    }

    /// `ClientSize`.
    pub fn client_size(&self) -> Size {
        self.base.client_size()
    }

    /// `ClientSize = value`. The size is also given to the part the thread
    /// that renders reads.
    pub fn set_client_size(&self, value: Size) {
        self.surface.state().client_size = value;
        self.base.set_client_size(value);
    }

    /// `RenderScaling`.
    pub fn render_scaling(&self) -> f64 {
        self.base.render_scaling()
    }

    /// `RenderScaling = value`. The scaling is also given to the part the
    /// thread that renders reads.
    pub fn set_render_scaling(&self, value: f64) {
        self.surface.state().render_scaling = value;
        self.base.set_render_scaling(value);
    }

    /// `DesktopScaling`.
    pub fn desktop_scaling(&self) -> f64 {
        match &self.overrides {
            Some(overrides) => overrides.desktop_scaling(self),
            None => self.render_scaling(),
        }
    }

    /// `Measure` (virtual).
    pub fn measure(&self, constraint: Size) -> Size {
        match &self.overrides {
            Some(overrides) => overrides.measure(self, constraint),
            None => self.base_measure(constraint),
        }
    }

    /// The `Measure` of this class.
    ///
    /// # Panics
    /// Panics when the top-level has not set its input root.
    pub fn base_measure(&self, constraint: Size) -> Size {
        let l = self.required_input_root().root_element();
        l.measure(constraint);
        l.desired_size()
    }

    /// `MouseDevice`.
    pub fn mouse_device(&self) -> Rc<dyn IMouseDevice> {
        self.mouse_device.clone()
    }

    /// `KeyboardDevice`.
    pub fn keyboard_device(&self) -> Rc<dyn IKeyboardDevice> {
        self.keyboard_device.clone()
    }

    /// `Surfaces`.
    pub fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        self.base.surfaces()
    }

    /// `RenderAndSendFrameIfNeeded` (protected: for deriving classes).
    pub fn render_and_send_frame_if_needed(&self) {
        if self.is_disposed() {
            return;
        }

        {
            let mut state = self.surface.state();
            if state.last_received_frame != state.last_sent_frame || state.format.is_none() {
                return;
            }

            // The size and the scaling as they are now, however they were set.
            state.client_size = self.base.client_size();
            state.render_scaling = self.base.render_scaling();
        }

        let framebuffer = self.surface.get_or_create_framebuffer();

        if framebuffer.stride() > 0 {
            if let Some(paint) = self.base.paint() {
                paint(Rect::from_size(framebuffer.client_size()));
            }
        }

        self.surface.send_last_frame_if_needed();
    }

    /// `InputRoot!`: a raw event without a root is an
    /// `ArgumentNullException` of its constructor.
    fn required_input_root(&self) -> Rc<dyn IInputRoot> {
        self.base.input_root().expect("Value cannot be null. (Parameter 'root')")
    }

    fn mouse_input_device(&self) -> Rc<dyn IInputDevice> {
        let device: Rc<dyn IInputDevice> = self.mouse_device.clone();
        device
    }

    fn keyboard_input_device(&self) -> Rc<dyn IInputDevice> {
        let device: Rc<dyn IInputDevice> = self.keyboard_device.clone();
        device
    }

    /// `Dispatcher.UIThread.RunJobs(DispatcherPriority.Input + 1)`.
    fn run_jobs_above_input(&self) {
        let priority = DispatcherPriority::from_value(DispatcherPriority::INPUT.value() + 1);
        self.surface.handle.dispatcher().run_jobs(Some(priority));
    }

    fn raise_pointer(&self, type_: RawPointerEventType, position: Point, modifiers: RawInputModifiers) {
        let Some(input) = self.base.input() else {
            return;
        };
        let args: Rc<dyn IRawInputEventArgs> = Rc::new(RawPointerEventArgs::new(
            self.mouse_input_device(),
            0,
            self.required_input_root(),
            type_,
            position,
            modifiers,
        ));
        input(args);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn a_mouse_button_maps_to_the_raw_event_of_the_press_or_the_release() {
        assert_eq!(RawPointerEventType::LeftButtonDown, get_ferro_event_type(ProtocolMouseButton::Left, true));
        assert_eq!(RawPointerEventType::LeftButtonUp, get_ferro_event_type(ProtocolMouseButton::Left, false));
        assert_eq!(RawPointerEventType::MiddleButtonDown, get_ferro_event_type(ProtocolMouseButton::Middle, true));
        assert_eq!(RawPointerEventType::MiddleButtonUp, get_ferro_event_type(ProtocolMouseButton::Middle, false));
        assert_eq!(RawPointerEventType::RightButtonDown, get_ferro_event_type(ProtocolMouseButton::Right, true));
        assert_eq!(RawPointerEventType::RightButtonUp, get_ferro_event_type(ProtocolMouseButton::Right, false));
        assert_eq!(RawPointerEventType::Move, get_ferro_event_type(ProtocolMouseButton::None, true));
    }

    #[test]
    fn the_modifiers_of_a_message_become_raw_modifiers() {
        assert_eq!(RawInputModifiers::NONE, get_ferro_raw_input_modifiers(None));
        assert_eq!(RawInputModifiers::NONE, get_ferro_raw_input_modifiers(Some(&[])));
        assert_eq!(
            RawInputModifiers::CONTROL
                | RawInputModifiers::ALT
                | RawInputModifiers::SHIFT
                | RawInputModifiers::META
                | RawInputModifiers::LEFT_MOUSE_BUTTON
                | RawInputModifiers::MIDDLE_MOUSE_BUTTON
                | RawInputModifiers::RIGHT_MOUSE_BUTTON,
            get_ferro_raw_input_modifiers(Some(&[
                InputModifiers::Control,
                InputModifiers::Alt,
                InputModifiers::Shift,
                InputModifiers::Windows,
                InputModifiers::LeftMouseButton,
                InputModifiers::MiddleMouseButton,
                InputModifiers::RightMouseButton,
            ]))
        );
    }

    #[test]
    fn the_first_pixel_format_of_the_client_is_taken() {
        assert_eq!(None, try_get_valid_pixel_format(None));
        assert_eq!(None, try_get_valid_pixel_format(Some(&[])));
        assert_eq!(
            Some(ProtocolPixelFormat::Bgra8888),
            try_get_valid_pixel_format(Some(&[ProtocolPixelFormat::Bgra8888, ProtocolPixelFormat::Rgba8888]))
        );
    }

    #[test]
    fn the_shared_part_is_shared_between_the_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RemoteServerSurface>();
    }
}
