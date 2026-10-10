//! The interfaces of Fcitx 4 (`org.fcitx.Fcitx.InputMethod`,
//! `.InputContext`) and of Fcitx 5 (`.InputMethod1`, `.InputContext1`):
//! the proxies the reference generates from the four descriptions in
//! `DBusXml/`, with the members the input method calls and the signals it
//! listens to.

use zbus::zvariant::OwnedObjectPath;

#[zbus::proxy(interface = "org.fcitx.Fcitx.InputMethod", gen_blocking = false, assume_defaults = false)]
pub trait InputMethod {
    /// Gives the identifier of the context, whether it is enabled, and
    /// two trigger keys with their states.
    #[zbus(name = "CreateICv3")]
    fn create_icv3(&self, appname: &str, pid: i32) -> zbus::Result<(i32, bool, u32, u32, u32, u32)>;
}

#[zbus::proxy(interface = "org.fcitx.Fcitx.InputMethod1", gen_blocking = false, assume_defaults = false)]
pub trait InputMethod1 {
    /// Gives the path of the context and its identifier.
    fn create_input_context(&self, arg0: &[(&str, &str)]) -> zbus::Result<(OwnedObjectPath, Vec<u8>)>;
}

pub use context::InputContextProxy;
pub use context1::InputContext1Proxy;

/// The proxy and the signals of the input context: a module of its own, because the two versions
/// have signals of the same names.
pub mod context {
    #[zbus::proxy(interface = "org.fcitx.Fcitx.InputContext", gen_blocking = false, assume_defaults = false)]
    pub trait InputContext {
        fn focus_in(&self) -> zbus::Result<()>;

        fn focus_out(&self) -> zbus::Result<()>;

        fn reset(&self) -> zbus::Result<()>;

        fn set_cursor_rect(&self, x: i32, y: i32, w: i32, h: i32) -> zbus::Result<()>;

        fn set_capacity(&self, caps: u32) -> zbus::Result<()>;

        #[zbus(name = "DestroyIC")]
        fn destroy_ic(&self) -> zbus::Result<()>;

        fn process_key_event(&self, keyval: u32, keycode: u32, state: u32, type_: i32, time: u32) -> zbus::Result<i32>;

        #[zbus(signal)]
        fn commit_string(&self, str: &str) -> zbus::Result<()>;

        #[zbus(signal)]
        fn update_formatted_preedit(&self, str: Vec<(String, i32)>, cursorpos: i32) -> zbus::Result<()>;

        #[zbus(signal)]
        fn forward_key(&self, keyval: u32, state: u32, type_: i32) -> zbus::Result<()>;
    }
}

/// The proxy and the signals of the input context: a module of its own, because the two versions
/// have signals of the same names.
pub mod context1 {
    #[zbus::proxy(interface = "org.fcitx.Fcitx.InputContext1", gen_blocking = false, assume_defaults = false)]
    pub trait InputContext1 {
        fn focus_in(&self) -> zbus::Result<()>;

        fn focus_out(&self) -> zbus::Result<()>;

        fn reset(&self) -> zbus::Result<()>;

        fn set_cursor_rect(&self, x: i32, y: i32, w: i32, h: i32) -> zbus::Result<()>;

        fn set_capability(&self, caps: u64) -> zbus::Result<()>;

        #[zbus(name = "DestroyIC")]
        fn destroy_ic(&self) -> zbus::Result<()>;

        fn process_key_event(&self, keyval: u32, keycode: u32, state: u32, type_: bool, time: u32) -> zbus::Result<bool>;

        #[zbus(signal)]
        fn commit_string(&self, str: &str) -> zbus::Result<()>;

        #[zbus(signal)]
        fn update_formatted_preedit(&self, str: Vec<(String, i32)>, cursorpos: i32) -> zbus::Result<()>;

        #[zbus(signal)]
        fn forward_key(&self, keyval: u32, state: u32, type_: bool) -> zbus::Result<()>;
    }
}
