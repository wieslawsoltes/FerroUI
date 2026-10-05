use super::ICompositionAnimationBase;
use crate::rendering::composition::{AsCompositionObject, CompositionObject, Compositor};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn same(a: &Rc<dyn ICompositionAnimationBase>, b: &Rc<dyn ICompositionAnimationBase>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// A collection of animations triggered when a condition is met.
///
/// Implicit animations let you drive animations by specifying trigger conditions rather than requiring the manual definition of animation behavior.
/// They help decouple animation start logic from core app logic. You define animations and the events that should trigger these animations.
/// Currently the only available trigger is animated property change.
///
/// When expression is used in ImplicitAnimationCollection a special keyword `this.FinalValue` will represent
/// the final value of the animated property that was changed
///
/// Upstream the class is an `IDictionary<string, ICompositionAnimationBase>`;
/// here the dictionary members are methods of the class. Entries are kept
/// in insertion order, which is the enumeration order of a dictionary that
/// has not had entries removed.
pub struct ImplicitAnimationCollection {
    object: CompositionObject,
    inner: RefCell<Vec<(String, Rc<dyn ICompositionAnimationBase>)>>,
}

impl ImplicitAnimationCollection {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<ImplicitAnimationCollection> {
        Rc::new(ImplicitAnimationCollection {
            object: CompositionObject::new(compositor, None),
            inner: RefCell::new(Vec::new()),
        })
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.inner.borrow().iter().position(|(k, _)| k == key)
    }

    /// The entries of the collection (`GetEnumerator`).
    pub fn items(&self) -> Vec<(String, Rc<dyn ICompositionAnimationBase>)> {
        self.inner.borrow().clone()
    }

    /// `ICollection<KeyValuePair<..>>.Add`.
    pub fn add_pair(&self, item: (String, Rc<dyn ICompositionAnimationBase>)) {
        self.add(&item.0, item.1)
    }

    pub fn clear(&self) {
        self.inner.borrow_mut().clear()
    }

    /// `ICollection<KeyValuePair<..>>.Contains`: whether the key is present
    /// with that very animation.
    pub fn contains_pair(&self, key: &str, value: &Rc<dyn ICompositionAnimationBase>) -> bool {
        self.inner.borrow().iter().any(|(k, v)| k == key && same(v, value))
    }

    /// `ICollection<KeyValuePair<..>>.CopyTo`. Panics when the entries do
    /// not fit in `array` from `array_index` on.
    pub fn copy_to(&self, array: &mut [Option<(String, Rc<dyn ICompositionAnimationBase>)>], array_index: usize) {
        let inner = self.inner.borrow();
        if array_index > array.len() || array.len() - array_index < inner.len() {
            panic!("Destination array is not long enough to copy all the items in the collection. Check array index and length.");
        }
        for (offset, item) in inner.iter().enumerate() {
            array[array_index + offset] = Some(item.clone());
        }
    }

    /// `ICollection<KeyValuePair<..>>.Remove`: removes the key only when it
    /// is present with that very animation.
    pub fn remove_pair(&self, key: &str, value: &Rc<dyn ICompositionAnimationBase>) -> bool {
        if !self.contains_pair(key, value) {
            return false;
        }
        self.remove(key)
    }

    pub fn count(&self) -> usize {
        self.inner.borrow().len()
    }

    /// `ICollection<KeyValuePair<..>>.IsReadOnly`.
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Adds an entry. Panics when the key is already present.
    pub fn add(&self, key: &str, value: Rc<dyn ICompositionAnimationBase>) {
        if self.index_of(key).is_some() {
            panic!("An item with the same key has already been added. Key: {key}");
        }
        self.inner.borrow_mut().push((key.to_owned(), value));
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.index_of(key).is_some()
    }

    pub fn remove(&self, key: &str) -> bool {
        match self.index_of(key) {
            Some(index) => {
                self.inner.borrow_mut().remove(index);
                true
            }
            None => false,
        }
    }

    pub fn try_get_value(&self, key: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.inner.borrow().iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    /// `this[key]`. Panics when the key is not present.
    pub fn get(&self, key: &str) -> Rc<dyn ICompositionAnimationBase> {
        match self.try_get_value(key) {
            Some(value) => value,
            None => panic!("The given key '{key}' was not present in the dictionary."),
        }
    }

    /// `this[key] = value`: adds the entry or replaces its animation.
    pub fn set(&self, key: &str, value: Rc<dyn ICompositionAnimationBase>) {
        match self.index_of(key) {
            Some(index) => self.inner.borrow_mut()[index].1 = value,
            None => self.inner.borrow_mut().push((key.to_owned(), value)),
        }
    }

    pub fn keys(&self) -> Vec<String> {
        self.inner.borrow().iter().map(|(k, _)| k.clone()).collect()
    }

    pub fn values(&self) -> Vec<Rc<dyn ICompositionAnimationBase>> {
        self.inner.borrow().iter().map(|(_, v)| v.clone()).collect()
    }

    // UWP compat
    pub fn size(&self) -> u32 {
        self.count() as u32
    }

    pub fn get_view(&self) -> HashMap<String, Rc<dyn ICompositionAnimationBase>> {
        self.inner.borrow().iter().cloned().collect()
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    pub fn insert(&self, key: &str, animation: Rc<dyn ICompositionAnimationBase>) {
        self.add(key, animation)
    }

    pub fn lookup(&self, key: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.try_get_value(key)
    }
}

impl AsCompositionObject for ImplicitAnimationCollection {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "ImplicitAnimationCollection"
    }
}
