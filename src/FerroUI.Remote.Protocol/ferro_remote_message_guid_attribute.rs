use crate::guid::Guid;
use crate::metsys_bson::BsonClass;

/// The identifier of a message class on the wire: the header of every
/// message carries it, and the receiving end finds the class by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FerroRemoteMessageGuidAttribute {
    guid: Guid,
}

impl FerroRemoteMessageGuidAttribute {
    /// The attribute with the identifier of the given text (`Guid.Parse`). A
    /// text that is not an identifier fails the build of the constant.
    pub const fn new(guid: &str) -> FerroRemoteMessageGuidAttribute {
        FerroRemoteMessageGuidAttribute { guid: Guid::parse_const(guid) }
    }

    pub const fn guid(&self) -> Guid {
        self.guid
    }
}

/// A class that carries the attribute: the counterpart of
/// `GetCustomAttribute<FerroRemoteMessageGuidAttribute>()` on a type.
/// [`ferro_remote_message_guid!`](crate::ferro_remote_message_guid) puts the
/// attribute on a class.
pub trait FerroRemoteMessage: BsonClass {
    const ATTRIBUTE: FerroRemoteMessageGuidAttribute;
}

/// Puts the identifier attribute on a message class and gives the class the
/// identifier as the constant `GUID`:
///
/// ```ignore
/// ferro_remote_message_guid!(MeasureViewportMessage, "6E3C5310-E2B1-4C3D-8688-01183AA48C5B");
/// ```
///
/// The class also has to be in the table of its assembly
/// ([`Assembly`](crate::Assembly)) for a resolver to find it.
#[macro_export]
macro_rules! ferro_remote_message_guid {
    ($class:ident, $guid:literal) => {
        impl $crate::FerroRemoteMessage for $class {
            const ATTRIBUTE: $crate::FerroRemoteMessageGuidAttribute =
                $crate::FerroRemoteMessageGuidAttribute::new($guid);
        }

        impl $class {
            /// The identifier of the message class on the wire.
            pub const GUID: $crate::Guid = <$class as $crate::FerroRemoteMessage>::ATTRIBUTE.guid();
        }
    };
}
