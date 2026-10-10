//! The tray icon of a FreeDesktop session (the port of
//! `DBusTrayIconImpl.cs`): an `org.kde.StatusNotifierItem` on a connection
//! of its own, registered with `org.kde.StatusNotifierWatcher`, with its
//! menu exported by [`DBusMenuExporter`].

use crate::dbus_helper::DBusHelper;
use crate::dbus_menu_exporter::{DBusMenuExporter, DBusMenuExporterImpl};
use crate::event::Event;
use crate::signal_watch::CancellationFlag;
use crate::ui_thread_object::UiThreadHandle;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::platform::{INativeMenuExporter, ITrayIconImpl, IWindowIconImpl};
use futures_util::future::{LocalBoxFuture, Shared};
use futures_util::{FutureExt, StreamExt};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use zbus::object_server::SignalEmitter;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;
use zbus::Connection;

/// `org.kde.StatusNotifierWatcher` (the proxy the reference generates from
/// `DBusXml/org.kde.StatusNotifierWatcher.xml`), with the member that is
/// called.
#[zbus::proxy(interface = "org.kde.StatusNotifierWatcher", gen_blocking = false, assume_defaults = false)]
pub trait StatusNotifierWatcher {
    fn register_status_notifier_item(&self, service: &str) -> zbus::Result<()>;
}

const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";

/// A pixmap as the item offers it: width, height, and the pixels as
/// bytes in the order alpha, red, green, blue.
pub type DBusPixmap = (i32, i32, Vec<u8>);

/// Turns an icon of the platform into the data of `_NET_WM_ICON` (width,
/// height, pixels); empty for an icon the platform does not know.
pub type IconConverter = Rc<dyn Fn(Option<&Rc<dyn IWindowIconImpl>>) -> Vec<u32>>;

static TRAY_ICON_INSTANCE_ID: AtomicI32 = AtomicI32::new(0);

fn log_error(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "DBUS") {
        logger.log(None, message);
    }
}

/// A request for the name of the item, which several callers wait for.
type NameTask = Shared<LocalBoxFuture<'static, Result<(), String>>>;

pub struct DBusTrayIconImpl {
    connection: Option<Connection>,
    watch_cts: RefCell<Option<CancellationFlag>>,
    status_notifier_item_dbus_obj: Option<Rc<StatusNotifierItemDbusObj>>,
    /// Whether the proxy of the watcher exists (`_statusNotifierWatcher`
    /// is not null); the proxy itself is made where it is called.
    status_notifier_watcher: Cell<bool>,
    icon: RefCell<DBusPixmap>,

    sys_tray_service_name: RefCell<Option<String>>,
    // This task shows if the name request is in progress, complete, or failed.
    sys_tray_service_name_request: RefCell<Option<(u64, NameTask)>>,
    // The bus keeps the name until the release is complete. A request before that fails.
    sys_tray_service_name_release: RefCell<Option<(u64, Shared<LocalBoxFuture<'static, ()>>)>>,
    next_task_id: Cell<u64>,
    tooltip_text: RefCell<Option<String>>,
    is_disposed: Cell<bool>,
    item_exported: Cell<bool>,
    service_connected: Cell<bool>,
    is_visible: Cell<bool>,

    is_active: Cell<bool>,
    menu_exporter: Option<Rc<DBusMenuExporterImpl>>,
    on_clicked: RefCell<Option<Rc<dyn Fn()>>>,
    icon_converter_delegate: RefCell<Option<IconConverter>>,
    this: Weak<DBusTrayIconImpl>,
}

impl DBusTrayIconImpl {
    /// `EmptyPixmap`.
    pub fn empty_pixmap() -> DBusPixmap {
        (1, 1, vec![255, 0, 0, 0])
    }

    /// A tray icon on a new connection to the session bus. Without a bus
    /// the icon is not active ([`is_active`](Self::is_active)).
    pub fn new() -> Rc<Self> {
        let connection = DBusHelper::try_create_new_connection(None);
        if connection.is_none() {
            log_error("Unable to get a dbus connection for system tray icons.");
        }
        Self::with_connection(connection)
    }

    /// The same on a connection that was made already (`None`: no bus).
    pub fn with_connection(connection: Option<Connection>) -> Rc<Self> {
        let parts = connection.as_ref().map(|connection| {
            let dbus_menu_path = DBusMenuExporter::generate_dbus_menu_obj_path();
            let menu_exporter = DBusMenuExporter::try_create_detached_native_menu(&dbus_menu_path, connection.clone());
            let item = StatusNotifierItemDbusObj::new(connection.clone(), &dbus_menu_path);
            (menu_exporter, item)
        });
        let is_active = parts.is_some();
        let (menu_exporter, item) = parts.unzip();

        let tray_icon = Rc::new_cyclic(|this: &Weak<DBusTrayIconImpl>| DBusTrayIconImpl {
            connection,
            watch_cts: RefCell::new(None),
            status_notifier_item_dbus_obj: item,
            status_notifier_watcher: Cell::new(false),
            icon: RefCell::new((0, 0, Vec::new())),
            sys_tray_service_name: RefCell::new(None),
            sys_tray_service_name_request: RefCell::new(None),
            sys_tray_service_name_release: RefCell::new(None),
            next_task_id: Cell::new(1),
            tooltip_text: RefCell::new(None),
            is_disposed: Cell::new(false),
            item_exported: Cell::new(false),
            service_connected: Cell::new(false),
            is_visible: Cell::new(true),
            is_active: Cell::new(is_active),
            menu_exporter,
            on_clicked: RefCell::new(None),
            icon_converter_delegate: RefCell::new(None),
            this: this.clone(),
        });

        if let Some(item) = &tray_icon.status_notifier_item_dbus_obj {
            let this = tray_icon.this.clone();
            item.activation_delegate.subscribe(move |()| {
                let on_clicked = this.upgrade().and_then(|this| this.on_clicked.borrow().clone());
                if let Some(on_clicked) = on_clicked {
                    on_clicked();
                }
            });
            tray_icon.watch_async();
        }

        tray_icon
    }

    pub fn is_active(&self) -> bool {
        self.is_active.get()
    }

    pub fn icon_converter_delegate(&self) -> Option<IconConverter> {
        self.icon_converter_delegate.borrow().clone()
    }

    pub fn set_icon_converter_delegate(&self, value: Option<IconConverter>) {
        *self.icon_converter_delegate.borrow_mut() = value;
    }

    fn next_task_id(&self) -> u64 {
        let id = self.next_task_id.get();
        self.next_task_id.set(id + 1);
        id
    }

    /// Follows the owner of the name of the watcher: the current one
    /// first, then every change (the reference waits, in turns, for the
    /// owner it knows to go and for an owner to come; both waits end with
    /// the next change, the signal `NameOwnerChanged` of the bus).
    fn watch_async(&self) {
        let Some(connection) = self.connection.clone() else {
            return;
        };
        let watch_cts = CancellationFlag::new();
        *self.watch_cts.borrow_mut() = Some(watch_cts.clone());
        let this = self.this.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let on_owner_changed = |owner: Option<String>| {
                if let Some(this) = this.upgrade() {
                    this.on_owner_changed(owner.as_deref());
                }
            };
            let watch = async {
                let dbus = zbus::fdo::DBusProxy::new(&connection).await?;
                // Subscribed before the current owner is asked for, so
                // that a change in between is not lost.
                let mut changes = dbus.receive_name_owner_changed_with_args(&[(0, WATCHER_NAME)]).await?;
                let owner = match dbus.get_name_owner(zbus::names::BusName::try_from(WATCHER_NAME)?).await {
                    Ok(owner) => Some(owner.to_string()),
                    Err(zbus::fdo::Error::NameHasNoOwner(_)) => None,
                    Err(e) => return Err(zbus::Error::from(e)),
                };
                on_owner_changed(owner);
                while !watch_cts.is_cancellation_requested() {
                    let Some(Some(signal)) = watch_cts.run(changes.next()).await else {
                        break;
                    };
                    let args = signal.args()?;
                    on_owner_changed(args.new_owner().as_ref().map(|owner| owner.to_string()));
                }
                Ok::<(), zbus::Error>(())
            };

            if let Some(Err(e)) = watch_cts.run(watch).await {
                let disposed = this.upgrade().is_none_or(|this| this.is_disposed.get());
                if !disposed {
                    log_error(&format!("Interface 'org.kde.StatusNotifierWatcher' is unavailable.\n{e}"));
                }
            }
        }));
    }

    fn on_owner_changed(&self, new_owner: Option<&str>) {
        if self.is_disposed.get() || self.connection.is_none() {
            return;
        }

        if new_owner.is_some() {
            if !self.service_connected.get() {
                self.service_connected.set(true);
                self.status_notifier_watcher.set(true);
            }

            // A new watcher can take the name with no gap. It does not know the item, so register again.
            if self.is_visible.get() {
                self.create_tray_icon();
            }
        } else {
            if self.service_connected.get() {
                self.destroy_tray_icon();
                self.service_connected.set(false);
            }

            // Get the name before a watcher comes. A new watcher scans the bus, and it adds a second
            // item if it finds the object before the connection owns the name.
            if self.is_visible.get() {
                self.create_tray_icon();
            }
        }
    }

    fn create_tray_icon(&self) {
        let (Some(connection), Some(item), Some(this)) =
            (self.connection.clone(), self.status_notifier_item_dbus_obj.clone(), self.this.upgrade())
        else {
            return;
        };
        if self.is_disposed.get() {
            return;
        }

        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            // The request this call waited for, and whether it completed.
            let mut request: Option<(u64, bool)> = None;

            let result: Result<(), String> = async {
                loop {
                    let Some((id, release)) = this.sys_tray_service_name_release.borrow().clone() else {
                        break;
                    };
                    release.await;
                    let same = this.sys_tray_service_name_release.borrow().as_ref().is_some_and(|(now, _)| *now == id);
                    if same {
                        *this.sys_tray_service_name_release.borrow_mut() = None;
                    }
                    if !this.should_own_name() {
                        return Ok(());
                    }
                }

                if this.sys_tray_service_name_request.borrow().is_none() {
                    // Keep the name after a hide. A new id shows a new item to the host.
                    if this.sys_tray_service_name.borrow().is_none() {
                        let pid = std::process::id();
                        let tid = TRAY_ICON_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
                        *this.sys_tray_service_name.borrow_mut() = Some(format!("org.kde.StatusNotifierItem-{pid}-{tid}"));
                    }

                    let name = this.sys_tray_service_name.borrow().clone().unwrap_or_default();
                    let task = this.request_tray_service_name_async(connection.clone(), name);
                    *this.sys_tray_service_name_request.borrow_mut() = Some((this.next_task_id(), task));
                }

                let Some((id, task)) = this.sys_tray_service_name_request.borrow().clone() else {
                    return Ok(());
                };
                request = Some((id, false));
                task.await?;
                request = Some((id, true));

                // A hide while the bus answers queues the release after this line.
                let same = this.sys_tray_service_name_request.borrow().as_ref().is_some_and(|(now, _)| *now == id);
                if !this.should_show_tray_icon() || !this.status_notifier_watcher.get() || !same {
                    return Ok(());
                }

                // A host that scans the bus adds a second item if the object is exported before the
                // connection owns the name. Two calls can reach this line, and a second export throws.
                if !this.item_exported.get() {
                    this.item_exported.set(true);
                    if let Err(e) = item.export().await {
                        this.item_exported.set(false);
                        return Err(e.to_string());
                    }
                }

                let Some(name) = this.sys_tray_service_name.borrow().clone() else {
                    return Ok(());
                };

                let watcher = async {
                    StatusNotifierWatcherProxy::builder(&connection)
                        .destination(WATCHER_NAME)?
                        .path(WATCHER_PATH)?
                        .cache_properties(CacheProperties::No)
                        .build()
                        .await
                }
                .await
                .map_err(|e| e.to_string())?;
                watcher.register_status_notifier_item(&name).await.map_err(|e| e.to_string())?;

                if !this.should_show_tray_icon() {
                    return Ok(());
                }

                item.set_title_and_tooltip(this.tooltip_text.borrow().as_deref());
                item.set_icon(this.icon.borrow().clone());
                Ok(())
            }
            .await;

            if let Err(e) = result {
                // Clear only this request, and only if it did not complete. The next call then asks for
                // the name again.
                if let Some((id, false)) = request {
                    let mut current = this.sys_tray_service_name_request.borrow_mut();
                    if current.as_ref().is_some_and(|(now, _)| *now == id) {
                        *current = None;
                    }
                }

                if !this.is_disposed.get() {
                    log_error(&format!("Unable to register the system tray icon.\n{e}"));
                }
            }
        }));
    }

    fn request_tray_service_name_async(&self, connection: Connection, name: String) -> NameTask {
        let this = self.this.clone();
        let task: LocalBoxFuture<'static, Result<(), String>> = Box::pin(async move {
            match connection.request_name(name.as_str()).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    // Use a new name next time, also when a hide cleared this request.
                    if let Some(this) = this.upgrade() {
                        let mut current = this.sys_tray_service_name.borrow_mut();
                        if current.as_deref() == Some(name.as_str()) {
                            *current = None;
                        }
                    }
                    Err(e.to_string())
                }
            }
        });
        task.shared()
    }

    // create_tray_icon reads these conditions again after each await.
    fn should_own_name(&self) -> bool {
        !self.is_disposed.get() && self.is_visible.get()
    }

    fn should_show_tray_icon(&self) -> bool {
        self.should_own_name() && self.service_connected.get()
    }

    fn destroy_tray_icon(&self) {
        let (Some(_), Some(item)) = (&self.connection, &self.status_notifier_item_dbus_obj) else {
            return;
        };
        if !self.item_exported.get() {
            return;
        }

        item.unexport();
        self.item_exported.set(false);
    }

    // A host removes the item when the name goes away. Keep the name when only the watcher stops.
    fn release_tray_service_name(&self) {
        let (Some(connection), Some(name)) = (self.connection.clone(), self.sys_tray_service_name.borrow().clone()) else {
            return;
        };
        let Some((_, request)) = self.sys_tray_service_name_request.borrow_mut().take() else {
            return;
        };

        let this = self.this.clone();
        let release: LocalBoxFuture<'static, ()> = Box::pin(async move {
            // Wait for the request. A release before the connection owns the name has no effect.
            if request.await.is_err() {
                // create_tray_icon logs this failure. There is no name to release.
                return;
            }

            if let Err(e) = connection.release_name(name.as_str()).await {
                if !this.upgrade().is_none_or(|this| this.is_disposed.get()) {
                    log_error(&format!("Unable to release the system tray icon name.\n{e}"));
                }
            }
        });
        let release = release.shared();
        *self.sys_tray_service_name_release.borrow_mut() = Some((self.next_task_id(), release.clone()));
        // The reference starts the release with the call; here it is a
        // task of the dispatcher, which whoever waits for it shares.
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || release));
    }
}

impl IDisposable for DBusTrayIconImpl {
    fn dispose(&self) {
        self.is_active.set(false);
        self.destroy_tray_icon();
        self.release_tray_service_name();
        if let Some(menu_exporter) = &self.menu_exporter {
            menu_exporter.dispose();
        }
        if let Some(watch_cts) = &*self.watch_cts.borrow() {
            watch_cts.cancel();
        }
        if let Some(item) = &self.status_notifier_item_dbus_obj {
            item.dispose();
        }
        self.is_disposed.set(true);
    }
}

impl ITrayIconImpl for DBusTrayIconImpl {
    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
        let Some(icon_converter_delegate) = self.icon_converter_delegate.borrow().clone() else {
            return;
        };
        if self.is_disposed.get() {
            return;
        }

        let Some(icon) = icon else {
            if let Some(item) = &self.status_notifier_item_dbus_obj {
                item.set_icon(Self::empty_pixmap());
            }
            return;
        };

        let x11_icon_data = icon_converter_delegate(Some(&icon));

        let Some(pixmap) = pixmap_from_icon_data(&x11_icon_data) else {
            return;
        };

        *self.icon.borrow_mut() = pixmap.clone();
        if let Some(item) = &self.status_notifier_item_dbus_obj {
            item.set_icon(pixmap);
        }
    }

    fn set_is_visible(&self, visible: bool) {
        if self.is_disposed.get() || visible == self.is_visible.get() {
            return;
        }

        // create_tray_icon reads is_visible. Set it first.
        self.is_visible.set(visible);

        if visible {
            self.create_tray_icon();
        } else {
            self.destroy_tray_icon();
            // Release the name also when the watcher is away. A hidden icon must not own a name.
            self.release_tray_service_name();
        }
    }

    fn set_tool_tip_text(&self, text: Option<&str>) {
        let Some(text) = text else {
            return;
        };
        if self.is_disposed.get() {
            return;
        }
        *self.tooltip_text.borrow_mut() = Some(text.to_string());
        if let Some(item) = &self.status_notifier_item_dbus_obj {
            item.set_title_and_tooltip(Some(text));
        }
    }

    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        self.menu_exporter.clone().map(|exporter| exporter as Rc<dyn INativeMenuExporter>)
    }

    fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
        self.on_clicked.borrow().clone()
    }

    fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>) {
        *self.on_clicked.borrow_mut() = value;
    }
}

/// The pixmap of the data of `_NET_WM_ICON`: the width, the height, and
/// one item per pixel (alpha in the highest byte), as bytes in network
/// order. `None` for data without a size; pixels the data does not have
/// are transparent.
pub(crate) fn pixmap_from_icon_data(x11_icon_data: &[u32]) -> Option<DBusPixmap> {
    if x11_icon_data.len() < 2 {
        return None;
    }

    let w = x11_icon_data[0] as i32;
    let h = x11_icon_data[1] as i32;

    let pix_length = usize::try_from(w).ok()?.checked_mul(usize::try_from(h).ok()?)?;
    let mut pix_byte_array = Vec::with_capacity(pix_length.checked_mul(4)?);

    for i in 0..pix_length {
        let raw_pixel = x11_icon_data.get(i + 2).copied().unwrap_or(0);
        pix_byte_array.extend_from_slice(&raw_pixel.to_be_bytes());
    }

    Some((w, h, pix_byte_array))
}

// The item is active, is more important that the item will be shown in some way to the user.
const STATUS_ACTIVE: &str = "Active";

/// What the properties of the item answer with. Written by the UI thread,
/// read by the thread of the connection.
#[derive(Default)]
struct ItemState {
    category: String,
    id: String,
    title: String,
    icon_pixmap: Vec<DBusPixmap>,
}

/// DBus Object used for setting system tray icons
/// (`StatusNotifierItemDbusObj`): the part of the UI thread.
///
/// Useful guide: https://web.archive.org/web/20210818173850/https://www.notmart.org/misc/statusnotifieritem/statusnotifieritem.html
pub(crate) struct StatusNotifierItemDbusObj {
    connection: Connection,
    menu: OwnedObjectPath,
    state: Arc<Mutex<ItemState>>,
    pub(crate) activation_delegate: Event,
    handle: RefCell<Option<UiThreadHandle<StatusNotifierItemDbusObj>>>,
    this: Weak<StatusNotifierItemDbusObj>,
}

pub(crate) const STATUS_NOTIFIER_ITEM_PATH: &str = "/StatusNotifierItem";

impl StatusNotifierItemDbusObj {
    fn new(connection: Connection, dbus_menu_path: &str) -> Rc<Self> {
        let menu = OwnedObjectPath::try_from(dbus_menu_path).expect("a generated menu path is an object path");
        Rc::new_cyclic(|this| StatusNotifierItemDbusObj {
            connection,
            menu,
            state: Arc::new(Mutex::new(ItemState::default())),
            activation_delegate: Event::new(),
            handle: RefCell::new(None),
            this: this.clone(),
        })
    }

    /// `Connection.AddMethodHandler(this)`.
    async fn export(&self) -> zbus::Result<()> {
        let Some(this) = self.this.upgrade() else {
            return Ok(());
        };
        let handle = self.handle.borrow_mut().get_or_insert_with(|| UiThreadHandle::register(this)).clone();
        let object = StatusNotifierItemObject { item: handle, menu: self.menu.clone(), state: self.state.clone() };
        self.connection.object_server().at(STATUS_NOTIFIER_ITEM_PATH, object).await.map(|_| ())
    }

    /// `Connection.RemoveMethodHandler(Path)`.
    fn unexport(&self) {
        let connection = self.connection.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let _ = connection.object_server().remove::<StatusNotifierItemObject, _>(STATUS_NOTIFIER_ITEM_PATH).await;
        }));
    }

    fn dispose(&self) {
        if let Some(handle) = self.handle.borrow_mut().take() {
            handle.unregister();
        }
    }

    fn invalidate_all(&self) {
        let connection = self.connection.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let Ok(emitter) = SignalEmitter::new(&connection, STATUS_NOTIFIER_ITEM_PATH) else {
                return;
            };
            let _ = StatusNotifierItemObject::new_title(&emitter).await;
            let _ = StatusNotifierItemObject::new_icon(&emitter).await;
            let _ = StatusNotifierItemObject::new_attention_icon(&emitter).await;
            let _ = StatusNotifierItemObject::new_overlay_icon(&emitter).await;
            let _ = StatusNotifierItemObject::new_tool_tip(&emitter).await;
            let _ = StatusNotifierItemObject::new_status(&emitter, STATUS_ACTIVE).await;
        }));
    }

    fn set_icon(&self, dbus_pixmap: DBusPixmap) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).icon_pixmap = vec![dbus_pixmap];
        self.invalidate_all();
    }

    fn set_title_and_tooltip(&self, text: Option<&str>) {
        let Some(text) = text else {
            return;
        };

        {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.id = text.to_string();
            state.category = "ApplicationStatus".to_string();
            state.title = text.to_string();
        }
        self.invalidate_all();
    }
}

/// The object the connection exports for the item.
pub(crate) struct StatusNotifierItemObject {
    item: UiThreadHandle<StatusNotifierItemDbusObj>,
    menu: OwnedObjectPath,
    state: Arc<Mutex<ItemState>>,
}

impl StatusNotifierItemObject {
    fn state(&self) -> std::sync::MutexGuard<'_, ItemState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The tool tip of an item: an icon name, icon pixmaps, a title and a text.
type ToolTip = (String, Vec<DBusPixmap>, String, String);

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItemObject {
    #[zbus(property)]
    fn category(&self) -> String {
        self.state().category.clone()
    }

    #[zbus(property)]
    fn id(&self) -> String {
        self.state().id.clone()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        self.state().title.clone()
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        STATUS_ACTIVE
    }

    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        self.menu.clone()
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<DBusPixmap> {
        self.state().icon_pixmap.clone()
    }

    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Vec<DBusPixmap> {
        Vec::new()
    }

    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> Vec<DBusPixmap> {
        Vec::new()
    }

    #[zbus(property)]
    fn attention_movie_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (String::new(), Vec::new(), String::new(), String::new())
    }

    fn context_menu(&self, _x: i32, _y: i32) {}

    async fn activate(&self, _x: i32, _y: i32) {
        self.item.call(|item| item.activation_delegate.raise()).await;
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {}

    fn scroll(&self, _delta: i32, _orientation: &str) {}

    #[zbus(signal)]
    async fn new_title(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_attention_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_overlay_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_tool_tip(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn new_status(emitter: &SignalEmitter<'_>, status: &str) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests;
