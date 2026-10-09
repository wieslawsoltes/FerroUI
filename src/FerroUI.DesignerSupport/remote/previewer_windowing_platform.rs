use crate::remote::detachable_transport_connection::DetachableTransportConnection;
use crate::remote::previewer_window_impl::PreviewerWindowImpl;
use crate::remote::stubs::{CursorFactoryStub, IconLoaderStub, WindowStub};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::platform::{DefaultPlatformSettings, ICursorFactory, IPlatformSettings};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::{IRenderLoop, IRenderTimer, RenderLoop, UiThreadRenderTimer};
use ferroui_base::FerroLocator;
use ferroui_controls::platform::{IPlatformIconLoader, ITopLevelImpl, ITrayIconImpl, IWindowImpl, IWindowingPlatform};
use ferroui_remote_protocol::{IFerroRemoteTransportConnection, Message};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    // The static fields of the original. They are per thread: the windows
    // are objects of the UI thread, and the previewer has one.
    static KEYBOARD: Rc<KeyboardDevice> = KeyboardDevice::new();
    static TRANSPORT: RefCell<Option<Arc<dyn IFerroRemoteTransportConnection>>> = const { RefCell::new(None) };
    static LAST_WINDOW_TRANSPORT: RefCell<Option<Arc<DetachableTransportConnection>>> = const { RefCell::new(None) };
    static LAST_WINDOW: RefCell<Option<Rc<PreviewerWindowImpl>>> = const { RefCell::new(None) };
    static PRE_FLIGHT_MESSAGES: RefCell<Vec<Message>> = const { RefCell::new(Vec::new()) };
}

/// The windowing platform of the previewer: the embeddable window is the
/// window of the previewed document, which renders into frames sent over
/// the connection of the session; every other window is a stub.
pub(crate) struct PreviewerWindowingPlatform;

impl PreviewerWindowingPlatform {
    /// `PreFlightMessages`: the messages of the client that every new
    /// window is given before anything else (its pixel formats, its
    /// viewport and its render information).
    pub(crate) fn pre_flight_messages() -> Vec<Message> {
        PRE_FLIGHT_MESSAGES.with(|messages| messages.borrow().clone())
    }

    /// `PreFlightMessages = value`.
    pub(crate) fn set_pre_flight_messages(value: Vec<Message>) {
        PRE_FLIGHT_MESSAGES.with(|messages| *messages.borrow_mut() = value);
    }

    /// The window of the previewed document, if one was created
    /// (`s_lastWindow`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn last_window() -> Option<Rc<PreviewerWindowImpl>> {
        LAST_WINDOW.with(|window| window.borrow().clone())
    }

    pub(crate) fn initialize(transport: Arc<dyn IFerroRemoteTransportConnection>) {
        TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport));
        let instance: Rc<dyn IWindowingPlatform> = Rc::new(PreviewerWindowingPlatform);
        let keyboard: Rc<dyn IKeyboardDevice> = KEYBOARD.with(|keyboard| keyboard.clone());
        let timer: Arc<dyn IRenderTimer> = Arc::new(UiThreadRenderTimer::new(60));
        let render_loop: Arc<dyn IRenderLoop> = RenderLoop::from_timer(timer);
        FerroLocator::current_mutable()
            .bind::<dyn ICursorFactory>()
            .to_singleton::<CursorFactoryStub>(|instance| instance)
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard)
            .bind::<dyn IPlatformSettings>()
            .to_singleton::<DefaultPlatformSettings>(|instance| instance)
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind::<dyn IWindowingPlatform>()
            .to_constant(instance)
            .bind::<dyn IPlatformIconLoader>()
            .to_singleton::<IconLoaderStub>(|instance| instance)
            .bind_to_self_singleton::<PlatformHotkeyConfiguration>();
    }

    /// Forgets the session: the connection, the last window and the
    /// messages for new windows. For tests, which run one after another on
    /// threads that are reused.
    #[cfg(test)]
    pub(crate) fn reset_for_unit_tests() {
        TRANSPORT.with(|slot| *slot.borrow_mut() = None);
        LAST_WINDOW_TRANSPORT.with(|slot| *slot.borrow_mut() = None);
        LAST_WINDOW.with(|slot| *slot.borrow_mut() = None);
        Self::set_pre_flight_messages(Vec::new());
    }
}

impl IWindowingPlatform for PreviewerWindowingPlatform {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        WindowStub::new(None)
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        let window: Rc<dyn ITopLevelImpl> = self.create_embeddable_window();
        window
    }

    /// # Panics
    /// Panics when the platform was not initialized with a connection.
    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        let last_window = LAST_WINDOW.with(|slot| slot.borrow_mut().take());
        if let Some(last_window) = last_window {
            let last_window_transport = LAST_WINDOW_TRANSPORT.with(|slot| slot.borrow_mut().take());
            if let Some(last_window_transport) = last_window_transport {
                last_window_transport.dispose();
            }
            //Ignore
            let _ = catch_unwind(AssertUnwindSafe(|| last_window.dispose()));
        }

        let transport = TRANSPORT
            .with(|slot| slot.borrow().clone())
            .expect("the windowing platform of the previewer is initialized with a connection");
        let last_window_transport = DetachableTransportConnection::new(transport);
        let last_window = PreviewerWindowImpl::new(last_window_transport.clone());
        LAST_WINDOW_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(last_window_transport.clone()));
        LAST_WINDOW.with(|slot| *slot.borrow_mut() = Some(last_window.clone()));
        for pf in Self::pre_flight_messages() {
            last_window_transport.fire_on_message(&*last_window_transport, &pf);
        }
        last_window
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        None
    }

    fn get_windows_z_order(&self, _windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        z_order.fill(0);
    }
}
