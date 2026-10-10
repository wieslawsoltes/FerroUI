//! The dispatcher of the UI thread over the main loop of GLib (the port of
//! `WaylandGlibDispatcher.cs`).

use crate::wayland_exception::FerroWaylandException;
use ferroui_x11::dispatching::glib_dispatcher_impl_base::GlibDispatcherImplBase;
use ferroui_x11::interop::glib::Glib;
use std::any::Any;
use std::rc::Rc;

/// GLib (GMainLoop) based UI-thread dispatcher for the Wayland backend, enabled via
/// `WaylandPlatformOptions::use_g_lib_main_loop`. It lets the framework share a GLib main loop
/// with GLib/GTK based libraries on the UI thread. Unlike X11 it attaches no platform event
/// source of its own: the Wayland connection is owned and pumped by the worker thread, which
/// posts input/events back via the dispatcher, so the base class' signaling/timer/background
/// machinery is all the UI thread needs.
///
/// The class of the reference is an empty subclass of the base; here it is the function that
/// makes the base.
pub struct WaylandGlibDispatcher;

impl WaylandGlibDispatcher {
    /// The dispatcher, or the error of GLib when its libraries cannot be opened.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        external_exception_logger: Option<Rc<dyn Fn(&(dyn Any + Send))>>,
    ) -> Result<Rc<GlibDispatcherImplBase>, FerroWaylandException> {
        let glib = Glib::try_get().map_err(|error| {
            FerroWaylandException::with_inner("WaylandPlatformOptions::use_g_lib_main_loop needs GLib", error)
        })?;
        Ok(GlibDispatcherImplBase::new(glib, external_exception_logger))
    }
}
