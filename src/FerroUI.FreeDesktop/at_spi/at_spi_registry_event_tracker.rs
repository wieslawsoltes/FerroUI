//! The port of `AtSpiRegistryEventTracker.cs`: whether any client of the
//! registry listens to the events of objects. While none does, the
//! server emits nothing.

use super::at_spi_constants::{BUS_NAME_REGISTRY, REGISTRY_PATH};
use super::dbus::proxies::OrgA11yAtspiRegistryProxy;
use crate::signal_watch::watch_stream;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use zbus::proxy::CacheProperties;

pub(crate) struct AtSpiRegistryEventTracker {
    connection: zbus::Connection,
    registered_events: RefCell<BTreeSet<String>>,
    registry_registered_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    registry_deregistered_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    has_event_listeners: Cell<bool>,
    disposed: Cell<bool>,
}

impl AtSpiRegistryEventTracker {
    pub(crate) fn new(connection: zbus::Connection) -> Rc<Self> {
        Rc::new(Self {
            connection,
            registered_events: RefCell::new(BTreeSet::new()),
            registry_registered_subscription: RefCell::new(None),
            registry_deregistered_subscription: RefCell::new(None),
            has_event_listeners: Cell::new(true),
            disposed: Cell::new(false),
        })
    }

    pub(crate) fn has_event_listeners(&self) -> bool {
        self.has_event_listeners.get()
    }

    pub(crate) async fn initialize_async(self: &Rc<Self>) {
        if let Err(e) = self.initialize_core().await {
            // Registry event tracking unavailable - remain chatty.
            log(&format!("AT-SPI registry event tracker unavailable; remaining chatty: {e}"));
            self.has_event_listeners.set(true);
        }
    }

    async fn initialize_core(self: &Rc<Self>) -> zbus::Result<()> {
        let registry_proxy = OrgA11yAtspiRegistryProxy::builder(&self.connection)
            .destination(BUS_NAME_REGISTRY)?
            .path(REGISTRY_PATH)?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;

        // Seed from current registrations
        let events = registry_proxy.get_registered_events().await?;
        if self.disposed.get() {
            return Ok(());
        }
        {
            let mut registered = self.registered_events.borrow_mut();
            registered.clear();
            registered.extend(events.into_iter().map(|(_bus, event_name)| event_name));
        }
        self.update_has_event_listeners();

        // The signals of the registry. The proxy takes them from whoever
        // owns the name of the registry and follows a change of the
        // owner, which the reference does itself with a watch of the
        // owner (docs/porting/DEVIATIONS.md).
        self.subscribe_to_registry_signals(&registry_proxy).await;
        Ok(())
    }

    pub(crate) fn dispose(&self) {
        self.disposed.set(true);
        if let Some(subscription) = self.registry_registered_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.registry_deregistered_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.registered_events.borrow_mut().clear();
    }

    async fn subscribe_to_registry_signals(self: &Rc<Self>, registry_proxy: &OrgA11yAtspiRegistryProxy<'static>) {
        let registered = registry_proxy.receive_event_listener_registered().await;
        let deregistered = registry_proxy.receive_event_listener_deregistered().await;
        let (registered, deregistered) = match (registered, deregistered) {
            (Ok(registered), Ok(deregistered)) => (registered, deregistered),
            (Err(e), _) | (_, Err(e)) => {
                log(&format!("AT-SPI registry signal subscription failed; remaining chatty: {e}"));
                self.has_event_listeners.set(true);
                return;
            }
        };
        if self.disposed.get() {
            return;
        }

        let weak = Rc::downgrade(self);
        *self.registry_registered_subscription.borrow_mut() = Some(watch_stream(registered, move |signal| {
            let (Some(this), Ok(arguments)) = (weak.upgrade(), signal.args()) else { return };
            this.on_registry_event_listener_registered(&arguments.event);
        }));
        let weak = Rc::downgrade(self);
        *self.registry_deregistered_subscription.borrow_mut() = Some(watch_stream(deregistered, move |signal| {
            let (Some(this), Ok(arguments)) = (weak.upgrade(), signal.args()) else { return };
            this.on_registry_event_listener_deregistered(&arguments.event);
        }));
    }

    fn on_registry_event_listener_registered(&self, event: &str) {
        self.registered_events.borrow_mut().insert(event.to_string());
        self.update_has_event_listeners();
    }

    fn on_registry_event_listener_deregistered(&self, event: &str) {
        self.registered_events.borrow_mut().remove(event);
        self.update_has_event_listeners();
    }

    fn update_has_event_listeners(&self) {
        let any = self.registered_events.borrow().iter().any(|event| is_object_event_class(event));
        self.has_event_listeners.set(any);
    }
}

/// Whether a registered event name asks for the events this server
/// emits: everything, or the classes `object:`, `window:` and `focus:`.
pub(crate) fn is_object_event_class(event_name: &str) -> bool {
    if event_name.trim().is_empty() {
        return false;
    }
    if event_name == "*" {
        return true;
    }

    let lower = event_name.to_ascii_lowercase();
    lower.starts_with("object:") || lower.starts_with("window:") || lower.starts_with("focus:")
}

fn log(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::FREE_DESKTOP_PLATFORM) {
        logger.log(None, message);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn the_event_filter_takes_the_classes_of_objects_windows_and_focus() {
        assert!(is_object_event_class("*"));
        assert!(is_object_event_class("object:state-changed:focused"));
        assert!(is_object_event_class("Object:children-changed"));
        assert!(is_object_event_class("window:activate"));
        assert!(is_object_event_class("focus:"));
        assert!(!is_object_event_class(""));
        assert!(!is_object_event_class("   "));
        assert!(!is_object_event_class("mouse:abs"));
        assert!(!is_object_event_class("keyboard:modifiers"));
        assert!(!is_object_event_class("objects"));
        assert!(!is_object_event_class("**"));
    }
}
