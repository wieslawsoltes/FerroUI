//! The globals of a connection (the port of `WaylandGlobals.cs`).
//!
//! The reference constructs the object while its registry listener already
//! writes to it. Here the announcements of the first round trip are recorded
//! (`RegistryBootstrap`), the globals are made from the record, and the
//! registry listener writes to the globals from then on.
//!
//! The globals of stage 2 of `docs/porting/wayland-platform.md` (the data
//! device manager, fractional scale and viewporter, text input, the exporter
//! of `xdg-foreign`) and of stage 3 (`linux-dmabuf`) are not bound yet.

use super::rendering::wayland_egl_wsi_platform_graphics::WaylandEglWsiPlatformGraphics;
use super::wayland_cursor_manager::WaylandCursorManager;
use super::wayland_input_dispatcher::WaylandInputDispatcher;
use super::wayland_outputs_tracker::WaylandOutputsTracker;
use crate::server::interop::wayland_connection::WaylandConnection;
use crate::server::wayland_platform_graphics::IWaylandGraphics;
use crate::server::wayland_worker::WaylandWorkerState;
use crate::wayland_exception::FerroWaylandException;
use ferroui_base::logging::{LogEventLevel, Logger};
use std::collections::HashMap;
use std::rc::Rc;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;

const SEAT_MIN_VERSION: u32 = 5;
const SEAT_MAX_VERSION: u32 = 9;

/// What the registry announced before the globals of the connection exist.
#[derive(Default)]
pub struct RegistryBootstrap {
    /// Every announcement in its order: the registry name, the interface, the version.
    announced: Vec<(u32, String, u32)>,
    /// The names that were taken away again.
    removed: Vec<u32>,
}

pub struct WaylandGlobals {
    queue_handle: QueueHandle<WaylandWorkerState>,
    /// The number of the connection the globals belong to.
    pub connection_id: u64,
    pub registry: WlRegistry,
    known_globals: HashMap<String, (u32, u32)>,
    pub outputs: WaylandOutputsTracker,
    pub wl_shm: WlShm,
    pub shm_formats: Vec<wl_shm::Format>,
    pub wl_compositor: WlCompositor,
    pub xdg_wm_base: XdgWmBase,
    pub xdg_output_manager: Option<ZxdgOutputManagerV1>,
    pub xdg_decoration_manager: Option<ZxdgDecorationManagerV1>,
    pub input_dispatcher: WaylandInputDispatcher,
    pub cursor_manager: WaylandCursorManager,
    /// The application identifier of the top-levels (an addition, see the options).
    pub app_id: Option<String>,
}

impl WaylandGlobals {
    /// Binds the globals of a connection and stores them in the state of the worker.
    pub fn create(connection: &mut WaylandConnection, state: &mut WaylandWorkerState) -> Result<(), FerroWaylandException> {
        let queue_handle = connection.queue_handle().clone();
        state.registry_bootstrap = Some(RegistryBootstrap::default());
        let registry = connection.display().display().get_registry(&queue_handle, ());

        // Use a roundtrip to ensure we get all existing globals before our own event loop runs
        let first = connection.roundtrip(state);
        let bootstrap = state.registry_bootstrap.take().unwrap_or_default();
        first?;

        let mut known_globals: HashMap<String, (u32, u32)> = HashMap::new();
        for (name, interface, version) in &bootstrap.announced {
            if !bootstrap.removed.contains(name) {
                known_globals.insert(interface.clone(), (*name, *version));
            }
        }

        let wl_shm: WlShm = Self::bind_required(&registry, &known_globals, 1, 1, &queue_handle)?;
        let wl_compositor: WlCompositor = Self::bind_required(&registry, &known_globals, 4, 6, &queue_handle)?;
        let xdg_wm_base: XdgWmBase = Self::bind_required(&registry, &known_globals, 3, 4, &queue_handle)?;

        let cursor_manager = WaylandCursorManager::new(connection.display(), &wl_shm, &wl_compositor, &queue_handle)?;

        // Require v3 of zxdg_output_manager_v1 so wl_output.done acts as
        // the unified terminator (xdg_output.done is deprecated since
        // v3). Simplifies init-batch gating — same pragmatism as the
        // wl_output v2 minimum.
        let xdg_output_manager: Option<ZxdgOutputManagerV1> = Self::bind(&registry, &known_globals, 3, 3, &queue_handle)?;

        // ForceDrawnDecorations is a test/debug option: skip binding the
        // SSD manager so toplevels behave as if the compositor never
        // advertised support, exercising the CSD code path.
        let xdg_decoration_manager: Option<ZxdgDecorationManagerV1> = if state.options.force_drawn_decorations {
            None
        } else {
            Self::bind(&registry, &known_globals, 1, 1, &queue_handle)?
        };

        let mut globals = WaylandGlobals {
            queue_handle: queue_handle.clone(),
            connection_id: connection.id(),
            registry,
            known_globals,
            outputs: WaylandOutputsTracker::new(state.outputs_sink.clone()),
            wl_shm,
            shm_formats: Vec::new(),
            wl_compositor,
            xdg_wm_base,
            xdg_output_manager: xdg_output_manager.clone(),
            xdg_decoration_manager,
            input_dispatcher: WaylandInputDispatcher::new(),
            cursor_manager,
            app_id: state.options.app_id.clone(),
        };

        // The outputs and the seats the registry announced, in the order it announced them
        // (the registry listener of the reference binds them as they are announced).
        for (name, interface, version) in &bootstrap.announced {
            if !bootstrap.removed.contains(name) {
                globals.on_global(*name, interface, *version);
            }
        }

        if let Some(manager) = xdg_output_manager {
            globals.outputs.attach_xdg_output_manager(manager, &queue_handle);
        }

        // Seats are bound reactively via the registry listener and managed by the input dispatcher
        globals.input_dispatcher.on_initial_globals_bound();

        state.globals = Some(globals);

        // Collect events from bound globals
        connection.roundtrip(state)?;

        // GPU swapchain selection. Default is WSI (wl_egl_window +
        // eglSwapBuffers) — required for NVIDIA and most resilient to driver
        // quirks. The dmabuf path (`use_dmabuf_swapchain`) is stage 3.
        let use_dmabuf = state.options.use_dmabuf_swapchain.unwrap_or(false);
        if use_dmabuf {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "Wayland") {
                logger.log(
                    None,
                    "The dmabuf swapchain is not built yet (stage 3 of docs/porting/wayland-platform.md): \
                     rendering goes through a wl_egl_window",
                );
            }
        }
        let gpu = WaylandEglWsiPlatformGraphics::try_create(connection, &state.options.gl_profiles)
            .map(|graphics| graphics as Rc<dyn IWaylandGraphics>);
        state.worker.platform_graphics().initialize(gpu);

        // TODO: sanity checks
        Ok(())
    }

    /// The handle objects of this connection are created with.
    pub fn queue_handle(&self) -> &QueueHandle<WaylandWorkerState> {
        &self.queue_handle
    }

    fn on_global(&mut self, name: u32, interface: &str, version: u32) {
        self.known_globals.insert(interface.to_string(), (name, version));
        if interface == wayland_client::protocol::wl_output::WlOutput::interface().name {
            let queue_handle = self.queue_handle.clone();
            self.outputs.add_global(&self.registry, name, version, &queue_handle);
        } else if interface == wayland_client::protocol::wl_seat::WlSeat::interface().name {
            self.on_seat_global(name, version);
        }
    }

    fn on_seat_global(&mut self, global_name: u32, version: u32) {
        let bind_version = version.min(SEAT_MAX_VERSION);
        if bind_version < SEAT_MIN_VERSION {
            return;
        }
        let queue_handle = self.queue_handle.clone();
        self.input_dispatcher.on_seat_added(global_name, &self.registry, bind_version, &queue_handle);
    }

    fn bind<I>(
        registry: &WlRegistry,
        known_globals: &HashMap<String, (u32, u32)>,
        min_version: u32,
        max_version: u32,
        queue_handle: &QueueHandle<WaylandWorkerState>,
    ) -> Result<Option<I>, FerroWaylandException>
    where
        I: Proxy + 'static,
        WaylandWorkerState: Dispatch<I, ()>,
    {
        let interface = I::interface();
        if interface.version < max_version {
            return Err(FerroWaylandException::new(format!(
                "{} v{max_version} is not supported by current bindings",
                interface.name
            )));
        }

        let Some((name, version)) = known_globals.get(interface.name) else {
            return Ok(None);
        };

        if *version < min_version {
            return Ok(None);
        }

        Ok(Some(registry.bind(*name, (*version).min(max_version), queue_handle, ())))
    }

    fn bind_required<I>(
        registry: &WlRegistry,
        known_globals: &HashMap<String, (u32, u32)>,
        min_version: u32,
        max_version: u32,
        queue_handle: &QueueHandle<WaylandWorkerState>,
    ) -> Result<I, FerroWaylandException>
    where
        I: Proxy + 'static,
        WaylandWorkerState: Dispatch<I, ()>,
    {
        Self::bind(registry, known_globals, min_version, max_version, queue_handle)?.ok_or_else(|| {
            FerroWaylandException::new(format!(
                "Required wayland global {} (>={min_version}) not found",
                I::interface().name
            ))
        })
    }

    /// Releases what the globals hold: the connection is lost or closed.
    pub fn dispose(mut self, state: &WaylandWorkerState) {
        self.input_dispatcher.dispose();
        self.cursor_manager.dispose();
        self.outputs.dispose();
        state.worker.platform_graphics().reset();
    }
}

impl Dispatch<WlRegistry, ()> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        _proxy: &WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                if let Some(bootstrap) = &mut state.registry_bootstrap {
                    bootstrap.announced.push((name, interface, version));
                } else if let Some(globals) = &mut state.globals {
                    globals.on_global(name, &interface, version);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(bootstrap) = &mut state.registry_bootstrap {
                    bootstrap.removed.push(name);
                } else if let Some(globals) = &mut state.globals {
                    globals.input_dispatcher.on_seat_removed(name);
                    if globals.outputs.on_global_removed(name) {
                        // The surfaces that were on the output are not told that they left it.
                        for top_level in state.top_levels.values_mut() {
                            top_level.shell_mut().surface_mut().forget_output(name);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlShm, ()> for WaylandWorkerState {
    fn event(state: &mut Self, _: &WlShm, event: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_shm::Event::Format { format: WEnum::Value(format) } = event {
            if let Some(globals) = &mut state.globals {
                globals.shm_formats.push(format);
            }
        }
    }
}

impl Dispatch<WlCompositor, ()> for WaylandWorkerState {
    fn event(_: &mut Self, _: &WlCompositor, _: <WlCompositor as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<XdgWmBase, ()> for WaylandWorkerState {
    fn event(_: &mut Self, proxy: &XdgWmBase, event: xdg_wm_base::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<ZxdgDecorationManagerV1, ()> for WaylandWorkerState {
    fn event(
        _: &mut Self,
        _: &ZxdgDecorationManagerV1,
        _: <ZxdgDecorationManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
