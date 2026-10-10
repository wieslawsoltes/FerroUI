//! The menu of a window or of a tray icon, exported over D-Bus (the port
//! of `DBusMenuExporter.cs`): the interface `com.canonical.dbusmenu`, and
//! for the menu of a window its registration with
//! `com.canonical.AppMenu.Registrar`.
//!
//! The exported object ([`DBusMenuObject`]) lives with the connection and
//! is called on its thread; it answers from the exporter of the UI thread
//! through a [`UiThreadHandle`].

use crate::dbus_helper::DBusHelper;
use crate::event::Event;
use crate::ui_thread_object::UiThreadHandle;
use ferroui_base::collections::CollectionChangedHandler;
use ferroui_base::input::KeyModifiers;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::platform::{INativeMenuExporter, IPlatformIconLoader, ITopLevelNativeMenuExporter};
use ferroui_controls::{MenuItemToggleType, NativeMenu, NativeMenuItem, NativeMenuItemBase, NativeMenuItemSeparator};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use zbus::object_server::SignalEmitter;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{ObjectPath, StructureBuilder, Value};
use zbus::Connection;

/// `com.canonical.AppMenu.Registrar` (the proxy the reference generates
/// from `DBusXml/com.canonical.AppMenu.Registrar.xml`), with the members
/// that are called.
#[zbus::proxy(interface = "com.canonical.AppMenu.Registrar", gen_blocking = false, assume_defaults = false)]
pub trait Registrar {
    fn register_window(&self, window_id: u32, menu_object_path: &ObjectPath<'_>) -> zbus::Result<()>;

    fn unregister_window(&self, window_id: u32) -> zbus::Result<()>;
}

const REGISTRAR_NAME: &str = "com.canonical.AppMenu.Registrar";
const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";

/// The properties of an item, by name.
pub(crate) type MenuProperties = HashMap<String, Value<'static>>;

/// The layout of an item: its identifier, its properties and its
/// children, each a variant of a layout (`(ia{sv}av)`).
pub(crate) type MenuLayout = (i32, MenuProperties, Vec<Value<'static>>);

pub struct DBusMenuExporter;

impl DBusMenuExporter {
    /// The exporter of the menu of the window `xid`, on the connection of
    /// the process; `None` without a session bus
    /// (`TryCreateTopLevelNativeMenu`).
    pub fn try_create_top_level_native_menu(xid: usize) -> Option<Rc<dyn ITopLevelNativeMenuExporter>> {
        let conn = DBusHelper::default_connection()?;
        Some(DBusMenuExporterImpl::new_top_level(conn, xid))
    }

    /// The exporter of a menu that belongs to no window, at `path` of
    /// `current_connection` (`TryCreateDetachedNativeMenu`).
    pub fn try_create_detached_native_menu(path: &str, current_connection: Connection) -> Rc<DBusMenuExporterImpl> {
        DBusMenuExporterImpl::new_detached(current_connection, path)
    }

    /// A new object path for a menu (`GenerateDBusMenuObjPath`).
    pub fn generate_dbus_menu_obj_path() -> String {
        format!("/org/ferroui/dbusmenu/{:032x}", random_u128())
    }
}

/// 128 bits that differ from call to call and from process to process
/// (the reference takes a new GUID): two hashes with keys the standard
/// library draws at random, over a counter of the process.
fn random_u128() -> u128 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let half = || {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
        hasher.write_u32(std::process::id());
        hasher.finish()
    };
    (u128::from(half()) << 64) | u128::from(half())
}

/// The object a connection exports for a menu.
pub(crate) struct DBusMenuObject {
    exporter: UiThreadHandle<DBusMenuExporterImpl>,
}

fn gone() -> zbus::fdo::Error {
    zbus::fdo::Error::UnknownObject("The menu is not exported any more".to_string())
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl DBusMenuObject {
    #[zbus(property)]
    fn version(&self) -> u32 {
        4
    }

    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> zbus::fdo::Result<(u32, MenuLayout)> {
        self.exporter
            .call(move |exporter| exporter.get_layout_call(parent_id, recursion_depth, &property_names))
            .await
            .ok_or_else(gone)
    }

    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<String>,
    ) -> zbus::fdo::Result<Vec<(i32, MenuProperties)>> {
        self.exporter
            .call(move |exporter| {
                ids.iter().map(|id| (*id, get_properties(&exporter.get_menu(*id), &property_names))).collect()
            })
            .await
            .ok_or_else(gone)
    }

    async fn get_property(&self, id: i32, name: String) -> zbus::fdo::Result<Value<'static>> {
        self.exporter
            .call(move |exporter| get_property(&exporter.get_menu(id), &name).unwrap_or(Value::I32(0)))
            .await
            .ok_or_else(gone)
    }

    async fn event(&self, id: i32, event_id: String, _data: Value<'_>, _timestamp: u32) -> zbus::fdo::Result<()> {
        self.exporter.call(move |exporter| exporter.handle_event(id, &event_id)).await.ok_or_else(gone)
    }

    async fn event_group(&self, events: Vec<(i32, String, Value<'_>, u32)>) -> zbus::fdo::Result<Vec<i32>> {
        let events: Vec<(i32, String)> = events.into_iter().map(|(id, event_id, _, _)| (id, event_id)).collect();
        self.exporter
            .call(move |exporter| {
                for (id, event_id) in &events {
                    exporter.handle_event(*id, event_id);
                }
                Vec::new()
            })
            .await
            .ok_or_else(gone)
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    #[zbus(signal)]
    async fn layout_updated(emitter: &SignalEmitter<'_>, revision: u32, parent: i32) -> zbus::Result<()>;
}

/// An item and the menu it opens, as the calls address them
/// (`(NativeMenuItemBase? item, NativeMenu? menu)`).
type ItemAndMenu = (Option<Ref<NativeMenuItemBase>>, Option<Ref<NativeMenu>>);

/// `DBusMenuExporterImpl`.
pub struct DBusMenuExporterImpl {
    connection: Connection,
    path: String,
    ids_to_items: RefCell<HashMap<i32, Ref<NativeMenuItemBase>>>,
    items_to_ids: RefCell<HashMap<Ref<NativeMenuItemBase>, i32>>,
    /// The subscriptions to the property changes of the items that have
    /// an identifier.
    item_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The menus of items whose collection of items is listened to.
    menus: RefCell<Vec<(Ref<NativeMenu>, u64)>>,
    xid: u32,
    app_menu: bool,
    registrar: RefCell<Option<RegistrarProxy<'static>>>,
    menu: RefCell<Option<(Ref<NativeMenu>, u64)>>,
    disposed: Cell<bool>,
    revision: Cell<u32>,
    reset_queued: Cell<bool>,
    next_id: Cell<i32>,
    is_native_menu_exported: Cell<bool>,
    on_is_native_menu_exported_changed: Event,
    /// How the exported object reaches this one, while it is exported.
    handle: RefCell<Option<UiThreadHandle<DBusMenuExporterImpl>>>,
    this: Weak<DBusMenuExporterImpl>,
}

impl DBusMenuExporterImpl {
    fn new(connection: Connection, path: String, xid: u32, app_menu: bool) -> Rc<Self> {
        let exporter = Rc::new_cyclic(|this| DBusMenuExporterImpl {
            connection,
            path,
            ids_to_items: RefCell::new(HashMap::new()),
            items_to_ids: RefCell::new(HashMap::new()),
            item_subscriptions: RefCell::new(Vec::new()),
            menus: RefCell::new(Vec::new()),
            xid,
            app_menu,
            registrar: RefCell::new(None),
            menu: RefCell::new(None),
            disposed: Cell::new(false),
            revision: Cell::new(1),
            reset_queued: Cell::new(false),
            next_id: Cell::new(1),
            is_native_menu_exported: Cell::new(false),
            on_is_native_menu_exported_changed: Event::new(),
            handle: RefCell::new(None),
            this: this.clone(),
        });
        exporter.set_native_menu(Some(NativeMenu::new()));
        exporter.initialize_async();
        exporter
    }

    /// The exporter of the menu of the window `xid`.
    pub fn new_top_level(connection: Connection, xid: usize) -> Rc<Self> {
        // The identifier of a window has 32 bits on the wire.
        Self::new(connection, DBusMenuExporter::generate_dbus_menu_obj_path(), xid as u32, true)
    }

    /// The exporter of a menu at `path`, which no window owns.
    pub fn new_detached(connection: Connection, path: &str) -> Rc<Self> {
        Self::new(connection, path.to_string(), 0, false)
    }

    /// The object path of the menu.
    pub fn path(&self) -> &str {
        &self.path
    }

    fn get_layout_call(&self, parent_id: i32, recursion_depth: i32, property_names: &[String]) -> (u32, MenuLayout) {
        let menu = self.get_menu(parent_id);
        let layout = self.get_layout(menu.0.as_ref(), menu.1.as_ref(), recursion_depth, property_names);
        if !self.is_native_menu_exported.get() {
            self.is_native_menu_exported.set(true);
            self.on_is_native_menu_exported_changed.raise();
        }

        (self.revision.get(), layout)
    }

    fn initialize_async(&self) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            if this.disposed.get() {
                return;
            }

            // Connection.AddMethodHandler(this)
            let handle = UiThreadHandle::register(this.clone());
            *this.handle.borrow_mut() = Some(handle.clone());
            let object = DBusMenuObject { exporter: handle };
            if let Err(e) = this.connection.object_server().at(this.path.as_str(), object).await {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::PLATFORM) {
                    logger.log(None, &format!("DBusMenu: the menu could not be exported: {e}"));
                }
                return;
            }
            if !this.app_menu {
                return;
            }

            let Ok(path) = ObjectPath::try_from(this.path.as_str()) else {
                return;
            };
            let registrar = async {
                RegistrarProxy::builder(&this.connection)
                    .destination(REGISTRAR_NAME)?
                    .path(REGISTRAR_PATH)?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await
            }
            .await;
            let Ok(registrar) = registrar else {
                return;
            };
            *this.registrar.borrow_mut() = Some(registrar.clone());
            if !this.disposed.get() && registrar.register_window(this.xid, &path).await.is_err() {
                // It's not really important if this code succeeds,
                // and it's not important to know if it succeeds
                // since even if we register the window it's not guaranteed that
                // menu will be actually exported
                *this.registrar.borrow_mut() = None;
            }
        }));
    }

    /*
         This is basic initial implementation, so we don't actually track anything and
         just reset the whole layout on *ANY* change

         This is not how it should work and will prevent us from implementing various features,
         but that's the fastest way to get things working, so...
     */
    fn do_layout_reset(&self) {
        self.reset_queued.set(false);
        let subscriptions = std::mem::take(&mut *self.item_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let menus = std::mem::take(&mut *self.menus.borrow_mut());
        for (menu, token) in menus {
            menu.items().remove_collection_changed(token);
        }
        self.ids_to_items.borrow_mut().clear();
        self.items_to_ids.borrow_mut().clear();
        self.revision.set(self.revision.get().wrapping_add(1));

        let (connection, path, revision) = (self.connection.clone(), self.path.clone(), self.revision.get());
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let emitted = async {
                let emitter = SignalEmitter::new(&connection, path.as_str())?;
                DBusMenuObject::layout_updated(&emitter, revision, 0).await
            }
            .await;
            if let Err(e) = emitted {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::PLATFORM) {
                    logger.log(None, &format!("DBusMenu: LayoutUpdated could not be sent: {e}"));
                }
            }
        }));
    }

    fn queue_reset(&self) {
        if self.reset_queued.get() {
            return;
        }
        self.reset_queued.set(true);
        let this = self.this.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = this.upgrade() {
                    this.do_layout_reset();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn items_changed_handler(&self) -> Rc<CollectionChangedHandler<Ref<NativeMenuItemBase>>> {
        let this = self.this.clone();
        Rc::new(move |_| {
            if let Some(this) = this.upgrade() {
                this.queue_reset();
            }
        })
    }

    fn get_menu(&self, id: i32) -> ItemAndMenu {
        if id == 0 {
            return (None, self.menu.borrow().as_ref().map(|(menu, _)| menu.clone()));
        }
        let item = self.ids_to_items.borrow().get(&id).cloned();
        let menu = item.as_ref().and_then(menu_of);
        (item, menu)
    }

    fn ensure_subscribed(&self, menu: Option<Ref<NativeMenu>>) {
        let Some(menu) = menu else {
            return;
        };
        if self.menus.borrow().iter().any(|(known, _)| *known == menu) {
            return;
        }
        let token = menu.items().add_collection_changed(self.items_changed_handler());
        self.menus.borrow_mut().push((menu, token));
    }

    fn get_id(&self, item: &Ref<NativeMenuItemBase>) -> i32 {
        if let Some(id) = self.items_to_ids.borrow().get(item) {
            return *id;
        }
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        self.ids_to_items.borrow_mut().insert(id, item.clone());
        self.items_to_ids.borrow_mut().insert(item.clone(), id);
        let this = self.this.clone();
        let subscription = item.property_changed(move |_| {
            if let Some(this) = this.upgrade() {
                this.queue_reset();
            }
        });
        self.item_subscriptions.borrow_mut().push(subscription);
        self.ensure_subscribed(menu_of(item));
        id
    }

    fn get_layout(
        &self,
        item: Option<&Ref<NativeMenuItemBase>>,
        menu: Option<&Ref<NativeMenu>>,
        depth: i32,
        property_names: &[String],
    ) -> MenuLayout {
        let id = item.map_or(0, |item| self.get_id(item));
        let props = get_properties(&(item.cloned(), menu.cloned()), property_names);
        let mut children = Vec::new();
        if let Some(menu) = menu.filter(|_| depth != 0) {
            for ch in menu.items().snapshot().iter() {
                let layout =
                    self.get_layout(Some(ch), menu_of(ch).as_ref(), if depth == -1 { -1 } else { depth - 1 }, property_names);
                let child = StructureBuilder::new().add_field(layout.0).add_field(layout.1).add_field(layout.2).build();
                if let Ok(child) = child {
                    children.push(Value::Structure(child));
                }
            }
        }

        (id, props, children)
    }

    fn handle_event(&self, id: i32, event_id: &str) {
        if event_id == "clicked" {
            let item = self.get_menu(id).0.and_then(|item| item.cast::<NativeMenuItem>());
            if let Some(item) = item.filter(|item| item.is_enabled()) {
                item.to_exporter_events_bridge().raise_clicked();
            }
        }
    }
}

/// The menu an item opens (`(item as NativeMenuItem)?.Menu`).
fn menu_of(item: &Ref<NativeMenuItemBase>) -> Option<Ref<NativeMenu>> {
    item.cast::<NativeMenuItem>().and_then(|item| item.menu())
}

const ALL_PROPERTIES: [&str; 9] =
    ["type", "label", "enabled", "visible", "shortcut", "toggle-type", "children-display", "toggle-state", "icon-data"];

fn get_property(i: &ItemAndMenu, name: &str) -> Option<Value<'static>> {
    let (it, menu) = i;
    let it = it.as_ref()?;

    if it.is::<NativeMenuItemSeparator>() {
        if name == "type" {
            return Some(Value::from("separator"));
        }
    } else if let Some(item) = it.cast::<NativeMenuItem>() {
        if name == "type" {
            return None;
        }
        if name == "label" {
            return Some(Value::from(item.header().unwrap_or_else(|| "<null>".to_string())));
        }
        if name == "enabled" {
            if item.menu().is_some_and(|menu| menu.items().snapshot().is_empty()) {
                return Some(Value::Bool(false));
            }
            if !item.is_enabled() {
                return Some(Value::Bool(false));
            }
            return None;
        }

        if name == "visible" {
            return Some(Value::Bool(item.is_visible()));
        }

        if name == "shortcut" {
            let gesture = item.gesture()?;
            if gesture.key_modifiers().is_empty() {
                return None;
            }
            let mut lst: Vec<String> = Vec::new();
            let modifiers = gesture.key_modifiers();
            if modifiers.contains(KeyModifiers::CONTROL) {
                lst.push("Control".to_string());
            }
            if modifiers.contains(KeyModifiers::ALT) {
                lst.push("Alt".to_string());
            }
            if modifiers.contains(KeyModifiers::SHIFT) {
                lst.push("Shift".to_string());
            }
            if modifiers.contains(KeyModifiers::META) {
                lst.push("Super".to_string());
            }
            lst.push(gesture.key().to_string());
            return Some(Value::new(vec![lst]));
        }

        if name == "toggle-type" {
            if item.toggle_type() == MenuItemToggleType::CheckBox {
                return Some(Value::from("checkmark"));
            }
            if item.toggle_type() == MenuItemToggleType::Radio {
                return Some(Value::from("radio"));
            }
        }

        if name == "toggle-state" && item.toggle_type() != MenuItemToggleType::None {
            return Some(Value::I32(i32::from(item.is_checked())));
        }

        if name == "icon-data" {
            if let Some(icon) = item.icon() {
                let loader = FerroLocator::current().get_service::<dyn IPlatformIconLoader>();

                if let Some(loader) = loader {
                    let icon = loader.load_icon_from_bitmap(icon.platform_impl().item());
                    let mut ms = Vec::new();
                    // An icon that cannot be encoded is no icon.
                    if icon.save(&mut ms).is_ok() {
                        return Some(Value::new(ms));
                    }
                }
            }
        }

        if name == "children-display" {
            if menu.is_some() {
                return Some(Value::from("submenu"));
            }
            return None;
        }
    }

    None
}

fn get_properties(i: &ItemAndMenu, names: &[String]) -> MenuProperties {
    let mut properties = HashMap::new();
    let mut add = |n: &str| {
        if let Some(v) = get_property(i, n) {
            properties.insert(n.to_string(), v);
        }
    };
    if names.is_empty() {
        ALL_PROPERTIES.iter().for_each(|n| add(n));
    } else {
        names.iter().for_each(|n| add(n));
    }

    properties
}

impl IDisposable for DBusMenuExporterImpl {
    fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.disposed.set(true);
        let handle = self.handle.borrow_mut().take();
        if let Some(handle) = &handle {
            handle.unregister();
        }
        let registrar = self.registrar.borrow().clone();
        let (connection, path, xid) = (self.connection.clone(), self.path.clone(), self.xid);
        // Fire and forget
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            if let Some(registrar) = registrar {
                if let Err(e) = registrar.unregister_window(xid).await {
                    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::PLATFORM) {
                        logger.log(None, &format!("DBusMenu UnregisterWindowAsync failed: {e}"));
                    }
                }
            }
            // Connection.RemoveMethodHandler(Path)
            if handle.is_some() {
                let _ = connection.object_server().remove::<DBusMenuObject, _>(path.as_str()).await;
            }
        }));
    }
}

impl INativeMenuExporter for DBusMenuExporterImpl {
    fn set_native_menu(&self, menu: Option<Ref<NativeMenu>>) {
        let menu = menu.unwrap_or_else(NativeMenu::new);

        if let Some((old, token)) = self.menu.borrow_mut().take() {
            old.items().remove_collection_changed(token);
        }
        let token = menu.items().add_collection_changed(self.items_changed_handler());
        *self.menu.borrow_mut() = Some((menu, token));

        self.do_layout_reset();
    }
}

impl ITopLevelNativeMenuExporter for DBusMenuExporterImpl {
    fn is_native_menu_exported(&self) -> bool {
        self.is_native_menu_exported.get()
    }

    fn on_is_native_menu_exported_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.on_is_native_menu_exported_changed.subscribe(move |()| handler());
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.on_is_native_menu_exported_changed.unsubscribe(token);
            }
        })
    }
}

#[cfg(test)]
mod tests;
