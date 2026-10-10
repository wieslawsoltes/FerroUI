//! The port of `AtSpiAccessibilityWatcher.cs`: whether the session has
//! accessibility enabled (`org.a11y.Status` of `org.a11y.Bus` on the
//! session bus), and the changes of that.

use super::at_spi_constants::{BUS_NAME_A11Y, PATH_A11Y};
use crate::event::Event;
use crate::signal_watch::watch_stream;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use zbus::zvariant::{OwnedValue, Value};

const INTERFACE_STATUS: &str = "org.a11y.Status";

pub struct AtSpiAccessibilityWatcher {
    session_connection: RefCell<Option<zbus::Connection>>,
    properties_watcher: RefCell<Option<Rc<dyn IDisposable>>>,
    is_enabled: Cell<bool>,
    is_enabled_changed: Event<bool>,
}

impl Default for AtSpiAccessibilityWatcher {
    fn default() -> Self {
        Self {
            session_connection: RefCell::new(None),
            properties_watcher: RefCell::new(None),
            is_enabled: Cell::new(false),
            is_enabled_changed: Event::new(),
        }
    }
}

impl AtSpiAccessibilityWatcher {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled.get()
    }

    /// Raised with the new value when [`is_enabled`](Self::is_enabled) changed.
    pub fn is_enabled_changed(&self) -> &Event<bool> {
        &self.is_enabled_changed
    }

    pub async fn init_async(self: &Rc<Self>) {
        if let Err(e) = self.init_core().await {
            // D-Bus session bus unavailable or org.a11y.Bus not present.
            // Silently degrade - accessibility remains disabled.
            log(&format!("AT-SPI watcher unavailable; accessibility remains disabled: {e}"));
            self.is_enabled.set(false);
        }
    }

    async fn init_core(self: &Rc<Self>) -> zbus::Result<()> {
        let connection = zbus::blocking::connection::Builder::session()?.build()?.into_inner();
        self.init_on_connection(connection).await
    }

    /// The rest of `InitAsync`, on a session connection that exists.
    pub(crate) async fn init_on_connection(self: &Rc<Self>, connection: zbus::Connection) -> zbus::Result<()> {
        *self.session_connection.borrow_mut() = Some(connection.clone());
        let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
            .destination(BUS_NAME_A11Y)?
            .path(PATH_A11Y)?
            .build()
            .await?;
        let interface = zbus::names::InterfaceName::from_static_str_unchecked(INTERFACE_STATUS);

        match proxy.get_all(interface.clone()).await {
            Ok(properties) => self.is_enabled.set(enabled_of(&properties)),
            Err(e) => {
                log(&format!("AT-SPI status properties query failed, defaulting to disabled: {e}"));
                self.is_enabled.set(false);
            }
        }

        let changes = proxy.receive_properties_changed().await?;
        let weak = Rc::downgrade(self);
        let watcher = watch_stream(changes, move |signal| {
            let (Some(this), Ok(arguments)) = (weak.upgrade(), signal.args()) else { return };
            if arguments.interface_name.as_str() != INTERFACE_STATUS {
                return;
            }
            let changed: HashMap<String, OwnedValue> = arguments
                .changed_properties
                .iter()
                .filter_map(|(name, value)| Some((name.to_string(), OwnedValue::try_from(value).ok()?)))
                .collect();
            let enabled = enabled_of(&changed);
            if enabled == this.is_enabled.get() {
                return;
            }
            this.is_enabled.set(enabled);
            this.is_enabled_changed.invoke(enabled);
        });
        *self.properties_watcher.borrow_mut() = Some(watcher);
        Ok(())
    }

    pub fn dispose(&self) {
        if let Some(watcher) = self.properties_watcher.borrow_mut().take() {
            watcher.dispose();
        }
        *self.session_connection.borrow_mut() = None;
    }
}

/// `IsEnabled || ScreenReaderEnabled` of a set of properties; a property
/// the set does not have counts as false, as the generated property
/// structure of the reference has it.
pub(crate) fn enabled_of(properties: &HashMap<String, OwnedValue>) -> bool {
    let flag = |name: &str| matches!(properties.get(name).map(|value| &**value), Some(Value::Bool(true)));
    flag("IsEnabled") || flag("ScreenReaderEnabled")
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

    fn properties(entries: &[(&str, bool)]) -> HashMap<String, OwnedValue> {
        entries.iter().map(|(name, value)| (name.to_string(), OwnedValue::from(*value))).collect()
    }

    #[test]
    fn accessibility_is_enabled_by_either_property() {
        assert!(!enabled_of(&properties(&[])));
        assert!(!enabled_of(&properties(&[("IsEnabled", false), ("ScreenReaderEnabled", false)])));
        assert!(enabled_of(&properties(&[("IsEnabled", true)])));
        assert!(enabled_of(&properties(&[("ScreenReaderEnabled", true), ("IsEnabled", false)])));
        assert!(!enabled_of(&properties(&[("Other", true)])));
    }
}
