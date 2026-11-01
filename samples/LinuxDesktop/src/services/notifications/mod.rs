//! The freedesktop notification endpoint and its UI-thread state. Bus requests
//! share FerroUI's UI-thread bridge with exported tray menus. Only a managed
//! session advertises the service; a windowed preview never competes for it.
mod model;

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_freedesktop::{ui_thread_object::UiThreadHandle, DBusCallQueue};
pub(crate) use model::Notice;
use model::{Event, History, Request};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use zbus::{object_server::SignalEmitter, zvariant::OwnedValue, Connection};

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

// Stop admitting producers when the bus cannot drain outgoing signals. Already
// admitted notifications may still expire or close (at most History's capacity).
const SIGNAL_BATCH_LIMIT: usize = 128;

struct Alarm {
    deadline: Instant,
    cancel: Rc<dyn IDisposable>,
}

impl Drop for Alarm {
    fn drop(&mut self) {
        self.cancel.dispose();
    }
}

pub(crate) struct Service {
    model: RefCell<History>,
    timer: RefCell<Option<Alarm>>,
    pending_batches: Cell<usize>,
    changed: Box<dyn Fn()>,
    connection: RefCell<Option<Connection>>,
    bus_status: RefCell<String>,
    queue: DBusCallQueue,
}

impl Service {
    pub fn new(managed: bool, changed: impl Fn() + 'static) -> Rc<Self> {
        let service = Rc::new(Self {
            model: Default::default(),
            timer: Default::default(),
            pending_batches: Cell::new(0),
            connection: Default::default(),
            changed: Box::new(changed),
            bus_status: RefCell::new(
                if managed {
                    "Connecting to the session bus"
                } else {
                    "Notifications are delivered in the Ferro Desktop login session"
                }
                .into(),
            ),
            queue: DBusCallQueue::new(|error| {
                Box::pin(async move {
                    eprintln!("Notification signal: {error}");
                })
            }),
        });
        if managed {
            let handle = UiThreadHandle::register(Rc::new(Rc::downgrade(&service)));
            let object = Endpoint(handle.clone());
            let weak = Rc::downgrade(&service);
            let owner =
                ferroui_view::Owner::current().expect("notification endpoint has a shell owner");
            drop(
                Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                    let result = async {
                        zbus::connection::Builder::session()?
                            .serve_at(PATH, object)?
                            .name(NAME)?
                            .build()
                            .await
                    }
                    .await;
                    if let Some(service) = weak.upgrade().filter(|_| owner.is_alive()) {
                        match result {
                            Ok(connection) => {
                                service.connection.replace(Some(connection));
                                service.bus_status.borrow_mut().clear();
                            }
                            Err(e) => {
                                let message =
                                    format!("The notification service is unavailable: {e}");
                                eprintln!("Ferro Desktop: {message}");
                                service.bus_status.replace(message);
                            }
                        }
                        (service.changed)();
                    }
                }),
            );
            let weak = Rc::downgrade(&service);
            ferroui_view::on_cleanup(move || {
                handle.unregister();
                if let Some(service) = weak.upgrade() {
                    service.timer.take();
                    service.queue.fail_all();
                    service.connection.take();
                }
            });
        }
        service
    }

    pub fn page(&self, first: usize, count: usize) -> Vec<Notice> {
        self.model
            .borrow()
            .items()
            .skip(first)
            .take(count)
            .cloned()
            .collect()
    }
    pub fn hide_banner(self: &Rc<Self>) {
        if self.model.borrow_mut().hide_banner() {
            self.commit(Vec::new());
        }
    }
    pub fn len(&self) -> usize {
        self.model.borrow().len()
    }
    pub fn quiet(&self) -> bool {
        self.model.borrow().quiet
    }
    pub fn status(&self) -> String {
        self.bus_status.borrow().clone()
    }
    pub fn banner(&self) -> Option<Notice> {
        self.model.borrow().banner().cloned()
    }

    pub fn accepts_actions(&self) -> bool {
        self.pending_batches.get() < SIGNAL_BATCH_LIMIT
    }

    // An input-priority bus request can reach the UI before a normal-priority
    // timer that is already due. Expire its IDs before allowing any mutation.
    fn expire_due(self: &Rc<Self>, now: Instant) {
        if self
            .timer
            .borrow()
            .as_ref()
            .is_some_and(|t| t.deadline <= now)
        {
            let events = self.model.borrow_mut().expire(now);
            self.commit(events);
        }
    }

    fn notify(self: &Rc<Self>, replace: u32, req: Request) -> zbus::fdo::Result<u32> {
        let now = Instant::now();
        self.expire_due(now);
        if !self.accepts_actions() {
            return Err(zbus::fdo::Error::LimitsExceeded(
                "Notification signal delivery is congested".into(),
            ));
        }
        let (id, events) = self
            .model
            .borrow_mut()
            .notify(replace, req, now)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.into()))?;
        self.commit(events);
        Ok(id)
    }

    fn remove(self: &Rc<Self>, id: u32, reason: u32) -> bool {
        self.expire_due(Instant::now());
        let events = self.model.borrow_mut().remove(id, reason);
        if let Some(events) = events {
            self.commit(events);
            true
        } else {
            false
        }
    }
    pub fn dismiss(self: &Rc<Self>, id: u32) {
        self.remove(id, 2);
    }
    pub fn invoke(self: &Rc<Self>, id: u32, key: &str) {
        self.expire_due(Instant::now());
        if !self.accepts_actions() {
            return;
        }
        let events = self.model.borrow_mut().invoke(id, key);
        if !events.is_empty() {
            self.commit(events);
        }
    }
    pub fn clear(self: &Rc<Self>) {
        self.expire_due(Instant::now());
        let events = self.model.borrow_mut().clear();
        self.commit(events);
    }
    pub fn toggle_quiet(self: &Rc<Self>) {
        self.model.borrow_mut().toggle_quiet();
        self.commit(Vec::new());
    }

    fn commit(self: &Rc<Self>, events: Vec<Event>) {
        if !events.is_empty() {
            if let Some(connection) = self.connection.borrow().clone() {
                self.pending_batches.set(self.pending_batches.get() + 1);
                let weak = Rc::downgrade(self);
                self.queue.enqueue(move || async move {
                    let result = async {
                        let emitter = SignalEmitter::new(&connection, PATH)?;
                        for event in events {
                            match event {
                                Event::Action(id, key) => {
                                    Endpoint::action_invoked(&emitter, id, &key).await?
                                }
                                Event::Closed(id, reason) => {
                                    Endpoint::notification_closed(&emitter, id, reason).await?
                                }
                            }
                        }
                        Ok(())
                    }
                    .await;
                    if let Some(service) = weak.upgrade() {
                        let pending = service.pending_batches.get();
                        service.pending_batches.set(pending - 1);
                        if pending == SIGNAL_BATCH_LIMIT {
                            (service.changed)();
                        }
                    }
                    result
                });
            }
        }
        let next = self.model.borrow().next_deadline();
        if next != self.timer.borrow().as_ref().map(|alarm| alarm.deadline) {
            self.timer.take();
            if let Some(deadline) = next {
                let weak = Rc::downgrade(self);
                let cancel = DispatcherTimer::run_once(
                    move || {
                        if let Some(service) = weak.upgrade() {
                            // A timer can fire slightly early. Discard this one-shot
                            // before recomputing the actual remaining interval.
                            service.timer.take();
                            let events = service.model.borrow_mut().expire(Instant::now());
                            service.commit(events);
                        }
                    },
                    deadline.saturating_duration_since(Instant::now()),
                    DispatcherPriority::NORMAL,
                );
                self.timer.replace(Some(Alarm { deadline, cancel }));
            }
        }
        (self.changed)();
    }
}

struct Endpoint(UiThreadHandle<std::rc::Weak<Service>>);

#[zbus::interface(name = "org.freedesktop.Notifications", spawn = false)]
impl Endpoint {
    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body", "actions"]
    }

    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    fn get_server_information(&self) -> (&str, &str, &str, &str) {
        ("Ferro Desktop", "FerroUI", env!("CARGO_PKG_VERSION"), "1.3")
    }

    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: &str,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        // These are plain text, not markup. Bound retained memory and reject malformed
        // requests instead of silently assigning an identifier to unsupported actions.
        let _ = app_icon;
        if app_name.len() > 1024
            || summary.len() > 4096
            || body.len() > 16 * 1024
            || actions.len() % 2 != 0
            || actions.len() > 32
            || actions.iter().any(|a| a.len() > 1024)
        {
            return Err(zbus::fdo::Error::InvalidArgs(
                "Invalid size or unpaired notification action".into(),
            ));
        }
        let mut pairs = actions.into_iter();
        let request = Request {
            app: app_name,
            title: summary,
            body,
            actions: std::iter::from_fn(|| Some((pairs.next()?, pairs.next()?))).collect(),
            critical: hints.get("urgency").and_then(|v| u8::try_from(v).ok()) == Some(2),
            resident: hints
                .get("resident")
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false),
            transient: hints
                .get("transient")
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false),
            timeout_ms: expire_timeout,
        };
        self.0
            .call(move |weak| weak.upgrade().map(|s| s.notify(replaces_id, request)))
            .await
            .flatten()
            .ok_or_else(|| zbus::fdo::Error::Failed("Session ended".into()))?
    }

    async fn close_notification(&self, id: u32) -> zbus::fdo::Result<()> {
        let closed = self
            .0
            .call(move |weak| weak.upgrade().is_some_and(|s| s.remove(id, 3)))
            .await;
        if closed == Some(true) {
            Ok(())
        } else {
            Err(zbus::fdo::Error::InvalidArgs(
                "No active notification with this ID".into(),
            ))
        }
    }
    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

#[cfg(test)]
mod delivery_tests {
    use super::*;
    use ferroui_base::reactive::Disposable;
    use std::time::Duration;

    fn request(timeout_ms: i32) -> Request {
        Request {
            app: "Editor".into(),
            title: "Saved".into(),
            body: String::new(),
            actions: vec![],
            timeout_ms,
            critical: false,
            transient: false,
            resident: false,
        }
    }

    #[test]
    fn late_dispatch_cannot_replace_an_expired_id() {
        let service = Service::new(false, || {});
        let start = Instant::now() - Duration::from_secs(1);
        let (id, _) = service
            .model
            .borrow_mut()
            .notify(0, request(10), start)
            .unwrap();
        service.timer.replace(Some(Alarm {
            deadline: start + Duration::from_millis(10),
            cancel: Disposable::create(|| {}),
        }));
        assert!(matches!(
            service.notify(id, request(0)),
            Err(zbus::fdo::Error::InvalidArgs(_))
        ));
        assert!(service.timer.borrow().is_none());
        assert!(!service.page(0, 1)[0].active);
    }

    #[test]
    fn incoming_burst_keeps_the_earliest_alarm_and_leaves_room_for_close() {
        let service = Service::new(false, || {});
        let now = Instant::now();
        service.model.borrow_mut().toggle_quiet();
        let (id, _) = service
            .model
            .borrow_mut()
            .notify(0, request(60_000), now)
            .unwrap();
        let cancelled = Rc::new(Cell::new(false));
        let flag = cancelled.clone();
        service.timer.replace(Some(Alarm {
            deadline: now + Duration::from_secs(60),
            cancel: Disposable::create(move || flag.set(true)),
        }));
        service.notify(0, request(0)).unwrap();
        assert!(!cancelled.get(), "an unrelated arrival restarted the timer");
        service.pending_batches.set(SIGNAL_BATCH_LIMIT);
        assert!(matches!(
            service.notify(0, request(0)),
            Err(zbus::fdo::Error::LimitsExceeded(_))
        ));
        assert_eq!(service.len(), 2);
        assert!(service.remove(id, 3));
        assert!(cancelled.get());
    }
}
