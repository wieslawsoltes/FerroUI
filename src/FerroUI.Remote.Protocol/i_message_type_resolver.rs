use uuid::Uuid;

use crate::error::Error;
use crate::metsys_bson::ClassType;

/// Maps the identifier a message carries on the wire to its class and back.
/// An identifier or a class the resolver does not know is a
/// `KeyNotFoundException` in the original and [`Error::KeyNotFound`] here.
///
/// Since 0.5 the identifiers are [`Uuid`]s, where they were the port type
/// `Guid`: an implementation takes and returns `Uuid`, and a caller with a
/// `Guid` converts it with `Uuid::from`.
pub trait IMessageTypeResolver: Send + Sync {
    fn get_by_guid(&self, id: Uuid) -> Result<&'static ClassType, Error>;
    fn get_guid(&self, ty: &'static ClassType) -> Result<Uuid, Error>;
}
