use super::BatchStreamWriter;
use crate::rendering::composition::ICompositionObject;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The owner of a list: registers itself for serialization when the list
/// changes.
pub trait IRegisterForSerialization {
    fn register_for_serialization(&self);
}

/// A helper used from generated UI-thread-side collections of composition
/// objects.
///
/// The owner is passed to each mutating member instead of being held.
pub struct ServerListProxyHelper<TClient: ICompositionObject + ?Sized> {
    changed: Cell<bool>,
    list: RefCell<Vec<Rc<TClient>>>,
}

impl<TClient: ICompositionObject + ?Sized> Default for ServerListProxyHelper<TClient> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TClient: ICompositionObject + ?Sized> ServerListProxyHelper<TClient> {
    pub fn new() -> Self {
        Self { changed: Cell::new(false), list: RefCell::new(Vec::new()) }
    }

    /// A snapshot of the items, for enumeration.
    pub fn items(&self) -> Vec<Rc<TClient>> {
        self.list.borrow().clone()
    }

    pub fn add(&self, parent: &dyn IRegisterForSerialization, item: Rc<TClient>) {
        let count = self.list.borrow().len();
        self.insert(parent, count, item)
    }

    pub fn clear(&self, parent: &dyn IRegisterForSerialization) {
        self.list.borrow_mut().clear();
        self.changed.set(true);
        parent.register_for_serialization();
    }

    pub fn contains(&self, item: &Rc<TClient>) -> bool {
        self.index_of(item).is_some()
    }

    /// Copies the items into `array`, starting at `array_index`. Panics if
    /// they do not fit.
    pub fn copy_to(&self, array: &mut [Option<Rc<TClient>>], array_index: usize) {
        for (offset, item) in self.list.borrow().iter().enumerate() {
            array[array_index + offset] = Some(item.clone());
        }
    }

    pub fn remove(&self, parent: &dyn IRegisterForSerialization, item: &Rc<TClient>) -> bool {
        let Some(idx) = self.index_of(item) else { return false };
        self.remove_at(parent, idx);
        true
    }

    pub fn count(&self) -> usize {
        self.list.borrow().len()
    }

    pub fn is_read_only(&self) -> bool {
        false
    }

    /// The index of an item, compared by reference.
    pub fn index_of(&self, item: &Rc<TClient>) -> Option<usize> {
        self.list.borrow().iter().position(|i| std::ptr::addr_eq(Rc::as_ptr(i), Rc::as_ptr(item)))
    }

    /// Panics if `index` is greater than the count.
    pub fn insert(&self, parent: &dyn IRegisterForSerialization, index: usize, item: Rc<TClient>) {
        self.list.borrow_mut().insert(index, item);
        self.changed.set(true);
        parent.register_for_serialization();
    }

    /// Panics if `index` is out of range.
    pub fn remove_at(&self, parent: &dyn IRegisterForSerialization, index: usize) {
        self.list.borrow_mut().remove(index);
        self.changed.set(true);
        parent.register_for_serialization();
    }

    /// Panics if `index` is out of range.
    pub fn get(&self, index: usize) -> Rc<TClient> {
        self.list.borrow()[index].clone()
    }

    /// Panics if `index` is out of range.
    pub fn set(&self, parent: &dyn IRegisterForSerialization, index: usize, value: Rc<TClient>) {
        self.list.borrow_mut()[index] = value;
        self.changed.set(true);
        parent.register_for_serialization();
    }

    pub fn serialize(&self, writer: &mut BatchStreamWriter<'_>) {
        writer.write(u8::from(self.changed.get()));
        if self.changed.get() {
            let list = self.list.borrow();
            writer.write(list.len() as i32);
            for el in list.iter() {
                writer.write_server_object(Some(el.server()));
            }
        }
        self.changed.set(false);
    }
}
