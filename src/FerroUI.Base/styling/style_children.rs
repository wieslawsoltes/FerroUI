use super::{IStyle, StyleBase};
use crate::WeakRef;
use std::cell::RefCell;
use std::rc::Rc;

/// The collection of child styles of a style.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone)]
pub struct StyleChildren(Rc<StyleChildrenData>);

struct StyleChildrenData {
    owner: WeakRef<StyleBase>,
    items: RefCell<Rc<Vec<Rc<dyn IStyle>>>>,
}

fn as_style_base(item: &Rc<dyn IStyle>) -> Option<&StyleBase> {
    item.as_object().and_then(|o| o.downcast_ref::<StyleBase>())
}

impl PartialEq for StyleChildren {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for StyleChildren {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StyleChildren({})", self.count())
    }
}

impl StyleChildren {
    pub(crate) fn new(owner: WeakRef<StyleBase>) -> Self {
        Self(Rc::new(StyleChildrenData { owner, items: RefCell::new(Rc::new(Vec::new())) }))
    }

    /// The number of child styles.
    #[inline]
    pub fn count(&self) -> usize {
        self.0.items.borrow().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.items.borrow().is_empty()
    }

    /// The child style at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> Rc<dyn IStyle> {
        self.0.items.borrow()[index].clone()
    }

    /// A snapshot of the child styles.
    #[inline]
    pub fn snapshot(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.0.items.borrow().clone()
    }

    /// Adds a child style.
    pub fn add(&self, item: impl Into<Rc<dyn IStyle>>) {
        self.insert(self.count(), item)
    }

    /// Inserts a child style.
    pub fn insert(&self, index: usize, item: impl Into<Rc<dyn IStyle>>) {
        let item = item.into();
        let owner = self.0.owner.upgrade();
        if let Some(style) = as_style_base(&item) {
            style.set_parent(owner.as_ref());
        }
        Rc::make_mut(&mut *self.0.items.borrow_mut()).insert(index, item);
    }

    /// Removes the child style at `index`.
    pub fn remove_at(&self, index: usize) {
        let item = self.get(index);
        if let Some(style) = as_style_base(&item) {
            style.set_parent(None);
        }
        if let Some(host) = self.0.owner.upgrade().and_then(|o| o.owner()) {
            if let Some(provider) = item.as_resource_provider() {
                provider.remove_owner(&host);
            }
        }
        Rc::make_mut(&mut *self.0.items.borrow_mut()).remove(index);
    }

    /// Removes a child style. Returns whether it was present.
    pub fn remove(&self, item: impl Into<Rc<dyn IStyle>>) -> bool {
        let item = item.into();
        let index = self.0.items.borrow().iter().position(|i| super::style_ptr_eq(i, &item));
        match index {
            Some(index) => {
                self.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Replaces the child style at `index`.
    pub fn set(&self, index: usize, item: impl Into<Rc<dyn IStyle>>) {
        let item = item.into();
        let owner = self.0.owner.upgrade();
        if let Some(style) = as_style_base(&item) {
            style.set_parent(owner.as_ref());
        }
        Rc::make_mut(&mut *self.0.items.borrow_mut())[index] = item.clone();
        if let Some(host) = owner.and_then(|o| o.owner()) {
            if let Some(provider) = item.as_resource_provider() {
                provider.add_owner(&host);
            }
        }
    }

    /// Removes all child styles.
    pub fn clear(&self) {
        *self.0.items.borrow_mut() = Rc::new(Vec::new());
    }
}
