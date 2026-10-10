//! The port of `AtSpiValueHandler.cs`: `org.a11y.atspi.Value`, which is
//! properties only.

use crate::at_spi::at_spi_constants::VALUE_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::VALUE;
use crate::at_spi::dbus::interface::{DBusInterface, InterfaceDescription};
use ferroui_controls::automation::provider::IRangeValueProvider;
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct AtSpiValueHandler {
    node: Weak<AtSpiNode>,
}

impl AtSpiValueHandler {
    pub(crate) fn new(_server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { node }
    }

    fn provider(&self) -> Option<Rc<dyn IRangeValueProvider>> {
        self.node.upgrade()?.peer().get_provider::<dyn IRangeValueProvider>()
    }

    fn set_current_value(&self, value: f64) {
        let Some(p) = self.provider() else { return };
        let clamped = clamp_value(p.minimum(), p.maximum(), value);
        // A provider that refuses (its element is disabled) leaves the value as it is.
        let _ = p.set_value(clamped);
    }
}

/// `Math.Max(minimum, Math.Min(maximum, value))`.
pub(crate) fn clamp_value(minimum: f64, maximum: f64, value: f64) -> f64 {
    minimum.max(maximum.min(value))
}

impl DBusInterface for AtSpiValueHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &VALUE
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        let provider = self.provider();
        let number = |read: fn(&dyn IRangeValueProvider) -> f64| Value::from(provider.as_deref().map_or(0.0, read));
        Some(match name {
            "version" => Value::from(VALUE_VERSION),
            "MinimumValue" => number(|p| p.minimum()),
            "MaximumValue" => number(|p| p.maximum()),
            "MinimumIncrement" => number(|p| p.small_change()),
            "CurrentValue" => number(|p| p.value()),
            "Text" => Value::from(String::new()),
            _ => return None,
        })
    }

    fn set_property(&self, name: &str, value: &Value<'_>) -> bool {
        match (name, value) {
            ("CurrentValue", Value::F64(value)) => {
                self.set_current_value(*value);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn a_value_is_clamped_to_the_range() {
        assert_eq!(clamp_value(0.0, 10.0, 5.0), 5.0);
        assert_eq!(clamp_value(0.0, 10.0, -1.0), 0.0);
        assert_eq!(clamp_value(0.0, 10.0, 11.0), 10.0);
        // A range that is upside down gives its minimum, as the reference.
        assert_eq!(clamp_value(10.0, 0.0, 5.0), 10.0);
    }
}
