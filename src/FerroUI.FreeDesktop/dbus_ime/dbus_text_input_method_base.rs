//! What the input methods over D-Bus share (the port of
//! `DBusTextInputMethodBase.cs`): watching the names of the input method
//! services, connecting to the first that is there, the queue of calls,
//! and the state of focus, capabilities and the cursor rectangle that is
//! reported when it changes.
//!
//! The reference is an abstract class its input methods derive from. Here
//! the base is the object (it implements the two interfaces of an input
//! method) and holds the part that differs as a
//! [`DBusTextInputMethodCore`]; that part reaches the protected members
//! of the base through a weak reference to it.

use crate::dbus_call_queue::{DBusCallError, DBusCallQueue, DBusResult};
use crate::event::Event;
use crate::ix11_input_method::{IX11InputMethodControl, IX11InputMethodFactory, X11InputMethodForwardedKey};
use crate::signal_watch::CancellationFlag;
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelPoint, PixelRect, PixelVector, Rect};
use ferroui_controls::Application;
use futures_util::StreamExt;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::rc::{Rc, Weak};
use zbus::Connection;

/// Creates an input method over D-Bus per window
/// (`DBusInputMethodFactory<T>`).
pub struct DBusInputMethodFactory {
    factory: Box<dyn Fn(usize) -> Rc<DBusTextInputMethodBase>>,
}

impl DBusInputMethodFactory {
    pub fn new(factory: impl Fn(usize) -> Rc<DBusTextInputMethodBase> + 'static) -> Self {
        Self { factory: Box::new(factory) }
    }
}

impl IX11InputMethodFactory for DBusInputMethodFactory {
    fn create_client(&self, xid: usize) -> (Rc<dyn ITextInputMethodImpl>, Rc<dyn IX11InputMethodControl>) {
        let im = (self.factory)(xid);
        (im.clone(), im)
    }
}

/// The members an input method over D-Bus implements (the abstract and
/// virtual members of `DBusTextInputMethodBase`). The futures are awaited
/// on the UI thread.
pub trait DBusTextInputMethodCore {
    /// Creates the input context at the service `name`. `Ok(false)` lets
    /// the base try the next service that is online.
    fn connect(&self, name: String) -> LocalBoxFuture<DBusResult<bool>>;

    fn disconnect_async(&self) -> LocalBoxFuture<DBusResult> {
        Box::pin(async { Ok(()) })
    }

    fn on_disconnected(&self) {}

    /// The part of `Reset` an input method adds to the one of the base.
    fn reset(&self) {}

    fn set_cursor_rect_core(&self, rect: PixelRect) -> LocalBoxFuture<DBusResult>;

    fn set_active_core(&self, active: bool) -> LocalBoxFuture<DBusResult>;

    fn set_capabilities_core(
        &self,
        _supports_preedit: bool,
        _supports_surrounding_text: bool,
    ) -> LocalBoxFuture<DBusResult> {
        Box::pin(async { Ok(()) })
    }

    fn reset_context_core(&self) -> LocalBoxFuture<DBusResult>;

    fn handle_key_core(
        &self,
        args: Rc<dyn IRawInputEventArgs>,
        key_val: i32,
        key_code: i32,
    ) -> LocalBoxFuture<DBusResult<bool>>;

    fn set_options(&self, options: &TextInputOptions);
}

pub struct DBusTextInputMethodBase {
    weak_self: Weak<DBusTextInputMethodBase>,
    core: Rc<dyn DBusTextInputMethodCore>,
    disposables: RefCell<Vec<Rc<dyn IDisposable>>>,
    online_names_queue: RefCell<VecDeque<String>>,
    connection: Connection,
    known_names: Vec<String>,
    watch_cts: RefCell<Option<CancellationFlag>>,
    connecting: Cell<bool>,
    current_name: RefCell<Option<String>>,
    queue: DBusCallQueue,
    window_active: Cell<bool>,
    ime_active: Cell<Option<bool>>,
    logical_rect: Cell<Rect>,
    last_reported_rect: Cell<Option<PixelRect>>,
    scaling: Cell<f64>,
    window_position: Cell<PixelPoint>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    on_commit: Event<String>,
    on_forward: Event<X11InputMethodForwardedKey>,
}

/// Runs an `async void` of the reference: a task of the UI dispatcher
/// nobody waits for.
fn spawn(future: impl Future<Output = ()> + 'static) {
    drop(Dispatcher::ui_thread().invoke_async_task_local(move || future));
}

fn log_error(area: &'static str, message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, area) {
        logger.log(None, message);
    }
}

impl DBusTextInputMethodBase {
    /// An input method that watches `known_names` on `connection`.
    /// `core` gets the base it belongs to and gives the part that differs
    /// between the input methods.
    ///
    /// # Panics
    /// Panics when called from a thread other than the UI thread.
    pub fn new(
        connection: Connection,
        known_names: &[&str],
        core: impl FnOnce(Weak<DBusTextInputMethodBase>) -> Rc<dyn DBusTextInputMethodCore>,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak: &Weak<Self>| {
            let queue_owner = weak.clone();
            Self {
                weak_self: weak.clone(),
                core: core(weak.clone()),
                disposables: RefCell::new(Vec::new()),
                online_names_queue: RefCell::new(VecDeque::new()),
                connection,
                known_names: known_names.iter().map(|name| name.to_string()).collect(),
                watch_cts: RefCell::new(None),
                connecting: Cell::new(false),
                current_name: RefCell::new(None),
                queue: DBusCallQueue::new(move |e| {
                    let owner = queue_owner.upgrade();
                    Box::pin(async move {
                        if let Some(owner) = owner {
                            owner.queue_on_error_async(e).await;
                        }
                    })
                }),
                window_active: Cell::new(false),
                ime_active: Cell::new(None),
                logical_rect: Cell::new(Rect::default()),
                last_reported_rect: Cell::new(None),
                scaling: Cell::new(1.0),
                window_position: Cell::new(PixelPoint::default()),
                client: RefCell::new(None),
                on_commit: Event::new(),
                on_forward: Event::new(),
            }
        });
        this.watch_async();
        this
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn is_connected(&self) -> bool {
        self.current_name.borrow().is_some()
    }

    pub fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    pub fn is_active(&self) -> bool {
        self.client.borrow().is_some()
    }

    fn watch_async(&self) {
        let cancellation_token = CancellationFlag::new();
        *self.watch_cts.borrow_mut() = Some(cancellation_token.clone());
        for name in &self.known_names {
            spawn(Self::watch_name_async(
                self.weak_self.clone(),
                self.connection.clone(),
                name.clone(),
                cancellation_token.clone(),
            ));
        }
    }

    /// Follows the owner of a name on the bus: the current one first, then
    /// every change. The reference asks its library for a watcher and
    /// waits for the owner it knows to go or for an owner to come; both
    /// waits end with the next change of the owner, which is the signal
    /// `NameOwnerChanged` of the bus for that name.
    async fn watch_name_async(
        this: Weak<Self>,
        connection: Connection,
        name: String,
        cancellation_token: CancellationFlag,
    ) {
        let on_owner_changed = |owner: Option<String>| {
            if let Some(this) = this.upgrade() {
                this.on_owner_changed(&name, owner.as_deref());
            }
        };

        let watch = async {
            let dbus = zbus::fdo::DBusProxy::new(&connection).await?;
            // Subscribed before the current owner is asked for, so that a
            // change in between is not lost.
            let mut changes = dbus.receive_name_owner_changed_with_args(&[(0, name.as_str())]).await?;
            let owner = match zbus::names::BusName::try_from(name.as_str()) {
                Ok(bus_name) => match dbus.get_name_owner(bus_name).await {
                    Ok(owner) => Some(owner.to_string()),
                    Err(zbus::fdo::Error::NameHasNoOwner(_)) => None,
                    Err(e) => return Err(zbus::Error::from(e)),
                },
                Err(e) => return Err(zbus::Error::from(e)),
            };
            on_owner_changed(owner);

            while !cancellation_token.is_cancellation_requested() {
                let Some(Some(signal)) = cancellation_token.run(changes.next()).await else {
                    break;
                };
                let args = signal.args()?;
                let owner = args.new_owner().as_ref().map(|owner| owner.to_string());
                on_owner_changed(owner);
            }
            Ok::<(), zbus::Error>(())
        };

        if let Some(Err(e)) = cancellation_token.run(watch).await {
            if !cancellation_token.is_cancellation_requested() {
                log_error(LogArea::FREE_DESKTOP_PLATFORM, &format!("WatchNameOwner for '{name}' failed: {e}"));
            }
        }
    }

    /// The name of the application an input method is told
    /// (`GetAppName`): the name of the application object, else the name
    /// of the program.
    pub fn get_app_name(&self) -> String {
        app_name()
    }

    fn on_owner_changed(&self, service_name: &str, new_owner: Option<&str>) {
        if new_owner.is_some() && self.current_name.borrow().is_none() {
            self.online_names_queue.borrow_mut().push_back(service_name.to_string());
            if !self.connecting.get() {
                self.connecting.set(true);
                let Some(this) = self.weak_self.upgrade() else {
                    return;
                };
                spawn(async move {
                    // The `finally` of the reference.
                    struct ResetConnecting(Rc<DBusTextInputMethodBase>);
                    impl Drop for ResetConnecting {
                        fn drop(&mut self) {
                            self.0.connecting.set(false);
                        }
                    }
                    let guard = ResetConnecting(this);
                    let this = &guard.0;

                    loop {
                        let Some(name) = this.online_names_queue.borrow_mut().pop_front() else {
                            break;
                        };
                        match this.core.connect(name.clone()).await {
                            Ok(true) => {
                                this.online_names_queue.borrow_mut().clear();
                                *this.current_name.borrow_mut() = Some(name);
                                return;
                            }
                            Ok(false) => {}
                            Err(ex) => {
                                log_error("IME", &format!("Unable to create IME input context:\n{ex}"));
                            }
                        }
                    }
                });
            }
        }

        // IME has crashed
        if new_owner.is_none() && self.current_name.borrow().as_deref() == Some(service_name) {
            *self.current_name.borrow_mut() = None;
            self.dispose_disposables();

            self.core.on_disconnected();
            self.reset_state();
        }
    }

    fn dispose_disposables(&self) {
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());
        for s in disposables {
            s.dispose();
        }
    }

    /// `Reset` of the reference: the part of the input method, then the
    /// part of the base.
    fn reset_state(&self) {
        self.core.reset();
        self.last_reported_rect.set(None);
        self.ime_active.set(None);
    }

    async fn queue_on_error_async(&self, e: DBusCallError) {
        log_error("IME", &format!("Error:\n{e}"));
        if let Err(ex) = self.core.disconnect_async().await {
            log_error("IME", &format!("Error while destroying the context:\n{ex}"));
        }
        self.core.on_disconnected();
        *self.current_name.borrow_mut() = None;
    }

    /// Queues a call behind the calls already queued (`Enqueue`).
    pub fn enqueue<F>(&self, cb: impl FnOnce() -> F + 'static)
    where
        F: Future<Output = DBusResult> + 'static,
    {
        self.queue.enqueue(cb);
    }

    /// Keeps a subscription until the input method disconnects
    /// (`AddDisposable`).
    pub fn add_disposable(&self, d: Rc<dyn IDisposable>) {
        self.disposables.borrow_mut().push(d);
    }

    pub fn fire_commit(&self, s: String) {
        self.on_commit.invoke(s);
    }

    pub fn fire_forward(&self, k: X11InputMethodForwardedKey) {
        self.on_forward.invoke(k);
    }

    fn update_active(&self) {
        let this = self.weak_self.clone();
        self.queue.enqueue(move || async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            if !this.is_connected() {
                return Ok(());
            }

            let active = this.window_active.get() && this.is_active();
            if Some(active) != this.ime_active.get() {
                this.ime_active.set(Some(active));
                this.core.set_active_core(active).await?;
            }
            Ok(())
        });
    }

    fn update_capabilities(&self, supports_preedit: bool, supports_surrounding_text: bool) {
        let this = self.weak_self.clone();
        self.queue.enqueue(move || async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            if !this.is_connected() {
                return Ok(());
            }

            this.core.set_capabilities_core(supports_preedit, supports_surrounding_text).await
        });
    }

    fn update_cursor_rect(&self) {
        let this = self.weak_self.clone();
        self.queue.enqueue(move || async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            if !this.is_connected() {
                return Ok(());
            }
            let cursor_rect = PixelRect::from_rect(this.logical_rect.get(), this.scaling.get());
            let position = this.window_position.get();
            let cursor_rect = cursor_rect.translate(PixelVector::new(position.x, position.y));
            if Some(cursor_rect) != this.last_reported_rect.get() {
                this.last_reported_rect.set(Some(cursor_rect));
                this.core.set_cursor_rect_core(cursor_rect).await?;
            }
            Ok(())
        });
    }
}

/// See [`DBusTextInputMethodBase::get_app_name`].
pub(crate) fn app_name() -> String {
    Application::current()
        .and_then(|application| application.name())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.file_stem().map(|name| name.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "FerroUI".to_string())
}

impl IX11InputMethodControl for DBusTextInputMethodBase {
    fn set_window_active(&self, active: bool) {
        self.window_active.set(active);
        self.update_active();
    }

    fn is_enabled(&self) -> bool {
        self.is_connected() && self.ime_active.get() == Some(true)
    }

    fn handle_event_async(
        &self,
        args: Rc<dyn IRawInputEventArgs>,
        key_val: i32,
        key_code: i32,
    ) -> LocalBoxFuture<bool> {
        let core = self.core.clone();
        let handled = self.queue.enqueue_async_with_result(move || core.handle_key_core(args, key_val, key_code));
        let this = self.weak_self.clone();
        Box::pin(async move {
            match handled.await {
                Ok(handled) => handled,
                // Disconnected
                Err(DBusCallError::Canceled) => false,
                // Error, disconnect
                Err(e) => {
                    if let Some(this) = this.upgrade() {
                        this.queue_on_error_async(e).await;
                    }
                    false
                }
            }
        })
    }

    fn commit(&self) -> &Event<String> {
        &self.on_commit
    }

    fn forward_key(&self) -> &Event<X11InputMethodForwardedKey> {
        &self.on_forward
    }

    fn update_window_info(&self, position: PixelPoint, scaling: f64) {
        self.window_position.set(position);
        self.scaling.set(scaling);
        self.update_cursor_rect();
    }

    fn dispose(&self) {
        if let Some(watch_cts) = self.watch_cts.borrow().as_ref() {
            watch_cts.cancel();
        }
        self.dispose_disposables();
        if !self.is_connected() {
            return;
        }
        let Some(this) = self.weak_self.upgrade() else {
            return;
        };
        spawn(async move {
            if let Err(ex) = this.core.disconnect_async().await {
                log_error("IME", &format!("Error while destroying the context:\n{ex}"));
            }

            *this.current_name.borrow_mut() = None;
        });
    }
}

impl ITextInputMethodImpl for DBusTextInputMethodBase {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        let supports_preedit = client.as_ref().is_some_and(|client| client.supports_preedit());
        let supports_surrounding_text = client.as_ref().is_some_and(|client| client.supports_surrounding_text());
        *self.client.borrow_mut() = client;
        self.update_active();
        self.update_capabilities(supports_preedit, supports_surrounding_text);
    }

    fn set_cursor_rect(&self, rect: Rect) {
        self.logical_rect.set(rect);
        self.update_cursor_rect();
    }

    fn set_options(&self, options: &TextInputOptions) {
        self.core.set_options(options);
    }

    fn reset(&self) {
        let this = self.weak_self.clone();
        self.queue.enqueue(move || async move {
            let Some(this) = this.upgrade() else {
                return Ok(());
            };
            this.reset_state();
            if !this.is_connected() {
                return Ok(());
            }
            this.core.reset_context_core().await
        });
    }
}
