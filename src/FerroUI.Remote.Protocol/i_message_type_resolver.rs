use crate::error::Error;
use crate::guid::Guid;
use crate::metsys_bson::ClassType;

/// Maps the identifier a message carries on the wire to its class and back.
/// An identifier or a class the resolver does not know is a
/// `KeyNotFoundException` in the original and [`Error::KeyNotFound`] here.
pub trait IMessageTypeResolver: Send + Sync {
    fn get_by_guid(&self, id: Guid) -> Result<&'static ClassType, Error>;
    fn get_guid(&self, ty: &'static ClassType) -> Result<Guid, Error>;
}
