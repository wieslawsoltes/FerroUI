//! The interfaces of the IBus portal (the proxies the reference generates
//! from `DBusXml/org.freedesktop.IBus.Portal.xml`), with the members the
//! input method calls and the signals it listens to.

use zbus::zvariant::{OwnedObjectPath, Value};

#[zbus::proxy(interface = "org.freedesktop.IBus.Portal", gen_blocking = false, assume_defaults = false)]
pub trait Portal {
    fn create_input_context(&self, client_name: &str) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(interface = "org.freedesktop.IBus.InputContext", gen_blocking = false, assume_defaults = false)]
pub trait InputContext {
    fn process_key_event(&self, keyval: u32, keycode: u32, state: u32) -> zbus::Result<bool>;

    fn set_cursor_location(&self, x: i32, y: i32, w: i32, h: i32) -> zbus::Result<()>;

    fn focus_in(&self) -> zbus::Result<()>;

    fn focus_out(&self) -> zbus::Result<()>;

    fn reset(&self) -> zbus::Result<()>;

    fn set_capabilities(&self, caps: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn commit_text(&self, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    fn forward_key_event(&self, keyval: u32, keycode: u32, state: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn update_preedit_text(&self, text: Value<'_>, cursor_pos: u32, visible: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn show_preedit_text(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn hide_preedit_text(&self) -> zbus::Result<()>;
}

#[zbus::proxy(interface = "org.freedesktop.IBus.Service", gen_blocking = false, assume_defaults = false)]
pub trait Service {
    fn destroy(&self) -> zbus::Result<()>;
}
