use super::{IServerObject, ServerCompositor};
use crate::rendering::composition::transport::BatchStreamReader;
use std::cell::RefCell;
use std::rc::Rc;

/// A server-side list container capable of receiving changes from the UI
/// thread. Right now it's quite dumb since it always receives the full
/// list.
///
/// Embedded by the generated property block of a list class; the items are
/// the server objects the batch names.
#[derive(Default)]
pub struct ServerList {
    list: RefCell<Vec<Rc<dyn IServerObject>>>,
}

impl ServerList {
    pub fn new() -> Self {
        Self::default()
    }

    /// A snapshot of the items.
    pub fn list(&self) -> Vec<Rc<dyn IServerObject>> {
        self.list.borrow().clone()
    }

    /// Drops the items.
    pub fn clear(&self) {
        self.list.borrow_mut().clear();
    }

    pub fn count(&self) -> usize {
        self.list.borrow().len()
    }

    /// The items viewed as their concrete type. Panics if an item is of
    /// another type.
    pub fn items<T: IServerObject>(&self) -> Vec<Rc<T>> {
        self.list
            .borrow()
            .iter()
            .map(|item| match item.clone().into_any_rc().downcast::<T>() {
                Ok(item) => item,
                Err(_) => panic!("a server list holds an item of another type"),
            })
            .collect()
    }

    /// Reads the list part of the changes of the object. The changes of the
    /// base class follow.
    pub fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, compositor: Option<&Rc<ServerCompositor>>) {
        if reader.read::<u8>() == 1 {
            let mut list = self.list.borrow_mut();
            list.clear();
            let count = reader.read::<i32>();
            for _ in 0..count {
                let Some(id) = reader.read_server_object() else {
                    panic!("a batch lists a null item in a server list");
                };
                match compositor.and_then(|c| c.get_object(id)) {
                    Some(object) => list.push(object),
                    None => panic!("a batch refers to server object {id:?}, which does not exist"),
                }
            }
        }
    }
}
