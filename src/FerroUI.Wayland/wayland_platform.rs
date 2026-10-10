//! The start of the backend (the port of `WaylandPlatform.cs`).
//!
//! Not bound yet (`docs/porting/wayland-platform.md`, stage 2): the
//! clipboard and the drag source.

use crate::screens::i_wayland_outputs_sink::{IWaylandOutputsSink, WaylandOutputsSinkProxy};
use crate::screens::snapshot_screens_impl::SnapshotScreensImpl;
use crate::server::wayland_worker::WaylandWorker;
use crate::server::wayland_worker_client::WaylandWorkerClient;
use crate::wayland_cursor_factory::WaylandCursorFactory;
use crate::ferro_wayland_exception::FerroWaylandException;
use crate::wayland_glib_dispatcher::WaylandGlibDispatcher;
use crate::wayland_platform_options::WaylandPlatformOptions;
use crate::wayland_top_level_factory::WaylandTopLevelFactory;
use ferroui_base::input::platform::{KeyGestureFormatInfo, PlatformHotkeyConfiguration};
use ferroui_base::input::{IKeyboardDevice, KeyModifiers, KeyboardDevice};
use ferroui_base::platform::{ICursorFactory, IPlatformSettings, ManagedDispatcherImpl};
use ferroui_base::rendering::IRenderLoop;
use ferroui_base::threading::{Dispatcher, IDispatcherImpl};
use ferroui_base::FerroLocator;
use ferroui_controls::platform::{IMountedVolumeInfoProvider, IPlatformIconLoader, IScreenImpl, IWindowingPlatform};
use ferroui_freedesktop::{DBusPlatformSettings, LinuxMountedVolumeInfoProvider};
use ferroui_x11::x11_icon_loader::X11IconLoader;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::Arc;

pub struct WaylandPlatform;

impl WaylandPlatform {
    /// Starts the backend.
    ///
    /// # Panics
    /// Panics when the backend cannot be started (the exception of the reference).
    pub fn initialize(options: WaylandPlatformOptions) {
        if let Some(error) = Self::try_initialize(options) {
            error.throw();
        }
    }

    /// Starts the backend, or says why it cannot be started. Nothing is registered in the
    /// second case, and the dispatcher of the UI thread has no implementation yet, so
    /// another backend can still be started.
    pub fn try_initialize(options: WaylandPlatformOptions) -> Option<FerroWaylandException> {
        // In some cases we aren't allowed to open multiple connections (e. g. WAYLAND_SOCKET env
        // is used), so we can't do separate usability checks and bail before doing any initialization.
        // Instead we do full startup sequence with the worker on its thread and wait for it
        // to get to the globals, which is the point where we can make an informed decision
        // about wl_display being usable.
        let worker_options = options.for_worker();
        let connection = match WaylandWorker::probe(&worker_options) {
            Ok(connection) => connection,
            Err(error) => return Some(FerroWaylandException::with_inner("Unable to connect to Wayland display", error)),
        };

        // The dispatcher over GLib needs its libraries: asked before the worker is started,
        // so that a failure leaves nothing running.
        let dispatcher_impl: Rc<dyn IDispatcherImpl> = if options.use_g_lib_main_loop {
            match WaylandGlibDispatcher::new(options.external_g_lib_main_loop_exception_logger.clone()) {
                Ok(dispatcher_impl) => dispatcher_impl,
                Err(error) => return Some(error),
            }
        } else {
            Rc::new(ManagedDispatcherImpl::new(None))
        };

        let worker = match WaylandWorker::new() {
            Ok(worker) => worker,
            Err(error) => return Some(error),
        };

        // NOTE: This technically mutates the global state since it touches the dispatcher,
        // which designates the current thread as the UI one, but it's generally safe to do
        let screens = SnapshotScreensImpl::new();
        let screens_sink: Rc<dyn IWaylandOutputsSink> = screens.clone();
        let screens_proxy = WaylandOutputsSinkProxy::new(screens_sink, Dispatcher::ui_thread());

        let (init_sender, init_receiver) = channel();
        if let Err(error) = worker.start(worker_options, connection, Some(screens_proxy), init_sender) {
            return Some(error);
        }

        match init_receiver.recv() {
            Ok(None) => {}
            Ok(Some(init_error)) => return Some(init_error),
            Err(_) => return Some(FerroWaylandException::new("The Wayland worker ended before it bound its globals")),
        }

        Dispatcher::initialize_ui_thread_dispatcher(dispatcher_impl);

        // The reference creates the compositor of the framework with the worker, before the
        // worker is started. Here it is created once the display is known to be usable, so
        // that a fallback backend finds the media context and the dispatcher untouched.
        let client = WaylandWorkerClient::new(worker.clone());

        let keyboard = KeyboardDevice::new();
        let windowing_platform: Rc<dyn IWindowingPlatform> =
            Rc::new(WaylandTopLevelFactory::new(client.clone(), keyboard.clone()));
        let render_loop: Arc<dyn IRenderLoop> = worker.render_loop().clone();
        let keyboard_device: Rc<dyn IKeyboardDevice> = keyboard;
        let cursor_factory: Rc<dyn ICursorFactory> = Rc::new(WaylandCursorFactory::new(client.clone()));
        let platform_settings: Rc<dyn IPlatformSettings> = DBusPlatformSettings::new();
        let mounted_volumes: Rc<dyn IMountedVolumeInfoProvider> = Rc::new(LinuxMountedVolumeInfoProvider::new());
        let icon_loader: Rc<dyn IPlatformIconLoader> = Rc::new(X11IconLoader);
        let screens_impl: Rc<dyn IScreenImpl> = screens.clone();

        FerroLocator::current_mutable()
            .bind::<dyn IWindowingPlatform>()
            .to_constant(windowing_platform)
            .bind::<Arc<dyn IRenderLoop>>()
            .to_constant(Rc::new(render_loop))
            .bind_to_self(Rc::new(PlatformHotkeyConfiguration::new(KeyModifiers::CONTROL)))
            .bind_to_self(Rc::new(KeyGestureFormatInfo::new(Some(HashMap::new()), "Super", "Ctrl", "Alt", "Shift")))
            .bind::<dyn IKeyboardDevice>()
            .to_constant(keyboard_device)
            .bind::<dyn ICursorFactory>()
            .to_constant(cursor_factory)
            .bind::<dyn IPlatformSettings>()
            .to_constant(platform_settings)
            .bind::<dyn IMountedVolumeInfoProvider>()
            .to_constant(mounted_volumes)
            .bind::<dyn IPlatformIconLoader>()
            .to_constant(icon_loader)
            .bind::<dyn IScreenImpl>()
            .to_constant(screens_impl)
            // Not in the reference: the client and the screens under their own types, for
            // code that asks the backend about itself (the example does).
            .bind_to_self(client)
            .bind_to_self(screens);

        None
    }
}
