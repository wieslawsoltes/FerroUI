//! Core array and shared-item contracts, without UI dependencies.
use std::{cell::RefCell, rc::Rc};
/// Shared fixed-length items. The alias uses ordinary Rust reference ownership.
pub type RibbonArray<T> = Rc<RibbonFixedItems<T>>;

/// Fixed-length storage for search results, KeyTips and shared model values.
/// This carries no array runtime or separate object/view allocation.
pub struct RibbonFixedItems<T> {
    values: RefCell<Box<[T]>>,
}
impl<T> PartialEq for RibbonFixedItems<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<T> Eq for RibbonFixedItems<T> {}
impl<T> RibbonFixedItems<T> {
    pub fn new(values: Vec<T>) -> Rc<Self> {
        Rc::new(Self {
            values: RefCell::new(values.into_boxed_slice()),
        })
    }
    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn set(&self, index: usize, value: T) {
        let old = std::mem::replace(&mut self.values.borrow_mut()[index], value);
        drop(old);
    }
}
impl<T: Clone> RibbonFixedItems<T> {
    pub fn get(&self, index: usize) -> T {
        self.values.borrow()[index].clone()
    }
    pub fn iter(self: &Rc<Self>) -> impl Iterator<Item = T> + use<T> {
        let length = self.len();
        let owner = (length > 0).then(|| self.clone());
        (0..length).map(move |i| {
            owner
                .as_ref()
                .expect("nonempty fixed-item iteration")
                .get(i)
        })
    }
    pub fn to_vec(&self) -> Vec<T> {
        self.values.borrow().to_vec()
    }
    pub fn contains(self: &Rc<Self>, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.iter().any(|v| v == *value)
    }
}
#[path = "shared_items.rs"]
mod shared_items;
pub use shared_items::RibbonItems;
/// Shared non-notifying item storage. The alias is an ordinary `Rc`.
pub type RibbonList<T> = Rc<RibbonItems<T>>;

/// Read-only access to ribbon items. Enumeration is separate from Count and
/// indexing: downstream implementations may expose live or failing sequences.
pub trait RibbonReadOnlyListImpl<T>: std::any::Any {
    fn count(&self) -> i32;
    fn get(&self, index: i32) -> T;
    fn iter_owned(self: Rc<Self>) -> crate::model::RibbonIterator<T>;
}
/// Coerce an existing `Rc` to this view to retain its allocation and mutations.
/// Weak ownership, identity and downcasting use the standard `Rc` APIs.
pub type RibbonReadOnlyList<T> = Rc<dyn RibbonReadOnlyListImpl<T>>;
impl<T: 'static> PartialEq for dyn RibbonReadOnlyListImpl<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
impl<T: 'static> dyn RibbonReadOnlyListImpl<T> {
    pub fn iter(self: &Rc<Self>) -> crate::model::RibbonIterator<T> {
        self.clone().iter_owned()
    }
}
impl<T: Clone + 'static> RibbonReadOnlyListImpl<T> for RibbonFixedItems<T> {
    fn count(&self) -> i32 {
        i32::try_from(self.len()).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> T {
        self.get(index as usize)
    }
    fn iter_owned(self: Rc<Self>) -> crate::model::RibbonIterator<T> {
        Box::new(RibbonFixedItems::iter(&self))
    }
}
impl<T: Clone + 'static> RibbonReadOnlyListImpl<T> for RibbonItems<T> {
    fn count(&self) -> i32 {
        i32::try_from(self.len()).expect("source Int32 Count")
    }
    fn get(&self, index: i32) -> T {
        self.get(index as usize)
    }
    fn iter_owned(self: Rc<Self>) -> crate::model::RibbonIterator<T> {
        Box::new(RibbonItems::iter(&self))
    }
}
