//! The windowing platform of the backend (the port of
//! `WaylandTopLevelFactory.cs`).

use crate::server::wayland_worker_client::WaylandWorkerClient;
use crate::window_impl::WindowImpl;
use ferroui_base::input::KeyboardDevice;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::{ITopLevelImpl, ITrayIconImpl, IWindowIconImpl, IWindowImpl, IWindowingPlatform};
use ferroui_freedesktop::DBusTrayIconImpl;
use ferroui_x11::x11_icon_loader::X11IconData;
use std::rc::Rc;

pub struct WaylandTopLevelFactory {
    client: Rc<WaylandWorkerClient>,
    keyboard: Rc<KeyboardDevice>,
}

impl WaylandTopLevelFactory {
    /// The factory of a worker. `keyboard` is the keyboard device the platform registered,
    /// which the windows raise their key events with (the reference asks the locator for it
    /// and casts).
    pub fn new(client: Rc<WaylandWorkerClient>, keyboard: Rc<KeyboardDevice>) -> Self {
        Self { client, keyboard }
    }

    pub fn client(&self) -> &Rc<WaylandWorkerClient> {
        &self.client
    }
}

/// The data of an icon as the tray icon sends it: the items of `_NET_WM_ICON` (width, height,
/// pixels), which the icon loader of the X11 project, shared by both backends, produces.
fn icon_converter(icon: Option<&Rc<dyn IWindowIconImpl>>) -> Vec<u32> {
    let Some(icon_data) = icon.and_then(|icon| X11IconData::from_icon_impl(icon).ok()) else {
        return Vec::new();
    };
    icon_data.data().iter().map(|item| *item as u32).collect()
}

impl IWindowingPlatform for WaylandTopLevelFactory {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        WindowImpl::new(self.client.clone(), self.keyboard.clone())
    }

    /// # Panics
    /// Always (the `NotSupportedException` of the reference).
    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always (the `NotSupportedException` of the reference).
    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        // org.kde.StatusNotifierItem (works on KDE; on GNOME via the
        // AppIndicator extension). No XEmbed fallback — XEmbed is X11-only,
        // and there is no Wayland-native systray protocol. If the watcher
        // service is not on the bus we return None and the framework falls
        // back to its own no-op handling.
        let dbus_tray_icon = DBusTrayIconImpl::new();
        if !dbus_tray_icon.is_active() {
            IDisposable::dispose(&*dbus_tray_icon);
            return None;
        }

        dbus_tray_icon.set_icon_converter_delegate(Some(Rc::new(icon_converter)));
        Some(dbus_tray_icon)
    }

    fn get_windows_z_order(&self, _windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]) {
        // Z-order querying isn't supported on Wayland; zero the output so callers
        // get a deterministic result instead of whatever the span previously held.
        z_order.fill(0);
    }
}
