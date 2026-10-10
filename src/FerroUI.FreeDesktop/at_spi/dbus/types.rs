//! The types of `DBusXml/Types.xml`, which the reference generates as
//! structures with a conversion to the D-Bus structure they travel as.
//! Here each has its wire form as a tuple, which is what the replies are
//! made from.

use crate::at_spi::at_spi_constants::NULL_PATH;
use std::collections::HashMap;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Structure, Value};

/// `(so)`: the name of a connection and the path of an object of it.
pub(crate) type ObjectReferenceWire = (String, OwnedObjectPath);

/// `(iiii)`.
pub(crate) type AtSpiRect = (i32, i32, i32, i32);

/// `(sss)`: the localized name, the description and the key binding of an action.
pub(crate) type AtSpiAction = (String, String, String);

/// `(ua(so))`: a relation type and its targets.
pub(crate) type AtSpiRelationEntry = (u32, Vec<ObjectReferenceWire>);

/// `a{ss}`.
pub(crate) type AtSpiAttributeSet = HashMap<String, String>;

/// `(iisv)`.
pub(crate) type AtSpiTextRange = (i32, i32, String, OwnedValue);

/// `(ss)`: the bus name of a listener and the event it registered for.
pub(crate) type AtSpiEventListener = (String, String);

/// `((so)(so)(so)iiassusau)`.
pub(crate) type AtSpiAccessibleCacheItem = (
    ObjectReferenceWire,
    ObjectReferenceWire,
    ObjectReferenceWire,
    i32,
    i32,
    Vec<String>,
    String,
    u32,
    String,
    Vec<u32>,
);

/// `(aiia{ss}iaiiasib)`.
pub(crate) type AtSpiMatchRuleWire =
    (Vec<i32>, i32, HashMap<String, String>, i32, Vec<i32>, i32, Vec<String>, i32, bool);

/// `AtSpiObjectReference`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AtSpiObjectReference {
    pub(crate) service: String,
    pub(crate) path: String,
}

impl AtSpiObjectReference {
    pub(crate) fn new(service: impl Into<String>, path: impl Into<String>) -> Self {
        Self { service: service.into(), path: path.into() }
    }

    fn object_path(&self) -> OwnedObjectPath {
        // The paths of references are constants and allocated node paths.
        match ObjectPath::try_from(self.path.clone()) {
            Ok(path) => path.into(),
            Err(_) => ObjectPath::from_static_str_unchecked(NULL_PATH).into(),
        }
    }

    /// The reference as it travels.
    pub(crate) fn to_wire(&self) -> ObjectReferenceWire {
        (self.service.clone(), self.object_path())
    }

    /// The reference as the value of a variant (`ToDbusStruct`).
    pub(crate) fn to_dbus_struct(&self) -> Value<'static> {
        Value::Structure(Structure::from((self.service.clone(), self.object_path())))
    }
}

/// `AtSpiMatchRule`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AtSpiMatchRule {
    pub(crate) states: Vec<i32>,
    pub(crate) state_match_type: i32,
    pub(crate) attributes: HashMap<String, String>,
    pub(crate) attribute_match_type: i32,
    pub(crate) roles: Vec<i32>,
    pub(crate) role_match_type: i32,
    pub(crate) interfaces: Vec<String>,
    pub(crate) interface_match_type: i32,
    pub(crate) invert: bool,
}

impl From<AtSpiMatchRuleWire> for AtSpiMatchRule {
    fn from(wire: AtSpiMatchRuleWire) -> Self {
        Self {
            states: wire.0,
            state_match_type: wire.1,
            attributes: wire.2,
            attribute_match_type: wire.3,
            roles: wire.4,
            role_match_type: wire.5,
            interfaces: wire.6,
            interface_match_type: wire.7,
            invert: wire.8,
        }
    }
}
