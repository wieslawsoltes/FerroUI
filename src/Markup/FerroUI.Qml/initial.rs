//! Install initial values and bindings into fresh slots before completion.
use super::*;
#[derive(Clone, Debug)]
pub enum InitialProperty {
    Value(Value),
    Binding(ExternalValue),
}
impl Instance {
    pub fn create_object_with_properties(
        &mut self,
        factory: ObjectId,
        parent: Option<ObjectId>,
        properties: BTreeMap<String, InitialProperty>,
    ) -> Result<ObjectId, Error> {
        self.transaction(|tx| tx.create_with_properties(factory, parent, properties))
    }
}
impl crate::transaction::Transaction<'_> {
    pub(crate) fn apply_initial(
        &mut self,
        root: ObjectId,
        first: usize,
        mut assigned: BTreeSet<SlotId>,
        props: BTreeMap<String, InitialProperty>,
    ) -> Result<(), Error> {
        for (name, entry) in props {
            let source = self.model.public_key(root, &name)?;
            let target = self.model.write_target(source)?;
            if target < first {
                return Err(Error::new(
                    "initial properties cannot modify objects outside the new component",
                ));
            }
            match entry {
                InitialProperty::Value(value) => {
                    self.model.validate_handles(&value)?;
                    let value = self.model.typed(&self.model.properties[target].ty, value)?;
                    let slot = &mut self.model.slots[target];
                    slot.value = value;
                    slot.binding = None;
                    slot.dirty = false;
                }
                InitialProperty::Binding(callback) => {
                    if callback.instance() != self.model.script_instance() {
                        return Err(Error::new("foreign initial property binding"));
                    }
                    let receiver = self.model.properties[source].owner;
                    self.model.slots[target].binding = Some(InstalledBinding::Guest {
                        callback,
                        receiver,
                        scope: self.declaration_scope(),
                    });
                    self.model.slots[target].dirty = true;
                }
            }
            assigned.insert(target);
        }
        for (at, property) in self.model.properties[first..].iter().enumerate() {
            if property.required && !assigned.contains(&(first + at)) {
                return Err(Error::new(format!(
                    "required property '{}' was not initialized",
                    property.name
                )));
            }
        }
        for slot in assigned {
            self.model.slots[slot].assigned = true;
        }
        Ok(())
    }
}
