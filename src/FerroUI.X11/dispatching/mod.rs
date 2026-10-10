//! The event loop of the platform over the connection to the server.

pub mod g_lib_dispatcher_impl;
pub mod glib_dispatcher_impl_base;
pub mod i_x11_platform_dispatcher;
pub mod x11_event_dispatcher;
pub mod x11_platform_threading;

pub use g_lib_dispatcher_impl::GlibDispatcherImpl;
pub use glib_dispatcher_impl_base::GlibDispatcherImplBase;
pub use i_x11_platform_dispatcher::IX11PlatformDispatcher;
pub use x11_event_dispatcher::{EventHandler, IEventHook, X11EventDispatcher};
pub use x11_platform_threading::X11PlatformThreading;
