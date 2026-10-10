//! ferroui-freedesktop
//!
//! The services of a FreeDesktop session that the Linux platforms of
//! FerroUI share: the input methods that are reached over D-Bus (IBus,
//! Fcitx), and, in later stages, the portals, the menu exporter and the
//! tray icon. `docs/porting/x11-platform.md` has the design (section 3 for
//! D-Bus) and the stages.
//!
//! D-Bus is spoken through `zbus`. A connection reads its socket on a
//! thread of its own; every call and every signal is awaited as a task of
//! the dispatcher of the UI thread, so the code of this crate runs on the
//! UI thread like the continuations of the reference.
//!
//! The crate builds on every Unix system and has only its event type
//! elsewhere.

pub mod event;

#[cfg(unix)]
pub mod dbus_call_queue;
#[cfg(unix)]
pub mod dbus_helper;
#[cfg(unix)]
pub mod dbus_ime;
#[cfg(unix)]
pub mod ix11_input_method;
#[cfg(unix)]
pub mod signal_watch;

#[cfg(unix)]
pub use dbus_call_queue::{DBusCallError, DBusCallQueue, DBusResult};
#[cfg(unix)]
pub use dbus_helper::DBusHelper;
pub use event::Event;
#[cfg(unix)]
pub use ix11_input_method::{IX11InputMethodControl, IX11InputMethodFactory, X11InputMethodForwardedKey};

#[cfg(all(test, unix))]
mod test_support;
