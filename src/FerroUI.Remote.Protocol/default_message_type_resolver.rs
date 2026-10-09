use std::any::TypeId;
use std::collections::HashMap;

use crate::assembly::{Assembly, ASSEMBLY};
use crate::error::Error;
use crate::guid::Guid;
use crate::i_message_type_resolver::IMessageTypeResolver;
use crate::metsys_bson::ClassType;

pub struct DefaultMessageTypeResolver {
    guids_to_types: HashMap<Guid, &'static ClassType>,
    types_to_guids: HashMap<TypeId, Guid>,
}

impl DefaultMessageTypeResolver {
    /// The resolver of the message classes of the given assemblies and of
    /// this library, which comes last: where two classes carry the same
    /// identifier, the identifier resolves to the class of the later
    /// assembly, and each class resolves to its identifier.
    ///
    /// The original enumerates the exported types of each assembly and reads
    /// the identifier attribute of each. Here an assembly is the table of
    /// its exported types ([`Assembly`]).
    pub fn new(assemblies: &[&'static Assembly]) -> DefaultMessageTypeResolver {
        let mut guids_to_types = HashMap::new();
        let mut types_to_guids = HashMap::new();
        for asm in assemblies.iter().copied().chain(std::iter::once(&ASSEMBLY)) {
            for t in asm.exported_types {
                if let Some(attr) = t.attribute {
                    guids_to_types.insert(attr.guid(), t.class);
                    types_to_guids.insert(t.class.id(), attr.guid());
                }
            }
        }
        DefaultMessageTypeResolver { guids_to_types, types_to_guids }
    }
}

impl IMessageTypeResolver for DefaultMessageTypeResolver {
    fn get_by_guid(&self, id: Guid) -> Result<&'static ClassType, Error> {
        self.guids_to_types
            .get(&id)
            .copied()
            .ok_or_else(|| Error::KeyNotFound(format!("The given key '{}' was not present in the dictionary.", id)))
    }

    fn get_guid(&self, ty: &'static ClassType) -> Result<Guid, Error> {
        self.types_to_guids.get(&ty.id()).copied().ok_or_else(|| {
            Error::KeyNotFound(format!("The given key '{}' was not present in the dictionary.", ty.name))
        })
    }
}
