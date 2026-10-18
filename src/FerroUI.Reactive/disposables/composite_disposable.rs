//! `CompositeDisposable`.

use super::ICancelable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The number of disposables above which removal finds a disposable through an index instead of a
/// linear search (`MaximumLinearSearchThreshold`; dotnet/reactive issue 2005: a `SelectMany` over
/// thousands of inner sequences made the removals of their completions quadratic).
const MAXIMUM_LINEAR_SEARCH_THRESHOLD: usize = 1024;

/// The number of slots above which the holes left by removals are compacted when they are more
/// than the disposables (`ShrinkThreshold`).
const SHRINK_THRESHOLD: usize = 64;

/// The identity of a disposable: the address of its value.
fn key(d: &Rc<dyn IDisposable>) -> usize {
    Rc::as_ptr(d) as *const () as usize
}

#[derive(Default)]
struct Inner {
    /// The disposables in the order they were added; a removed one leaves a hole.
    slots: Vec<Option<Rc<dyn IDisposable>>>,
    count: usize,
    /// The slots of each disposable, once the group holds more than the threshold.
    index: Option<HashMap<usize, Vec<usize>>>,
}

impl Inner {
    fn with_disposables(disposables: Vec<Rc<dyn IDisposable>>) -> Self {
        let count = disposables.len();
        let mut inner = Self { slots: disposables.into_iter().map(Some).collect(), count, index: None };
        if count > MAXIMUM_LINEAR_SEARCH_THRESHOLD {
            inner.rebuild_index();
        }
        inner
    }

    fn rebuild_index(&mut self) {
        let mut index: HashMap<usize, Vec<usize>> = HashMap::with_capacity(self.slots.len() + self.slots.len() / 4);
        for (position, slot) in self.slots.iter().enumerate() {
            if let Some(d) = slot {
                index.entry(key(d)).or_default().push(position);
            }
        }
        self.index = Some(index);
    }

    fn add(&mut self, item: Rc<dyn IDisposable>) {
        let position = self.slots.len();
        let item_key = key(&item);
        self.slots.push(Some(item));
        self.count += 1;
        match &mut self.index {
            Some(index) => index.entry(item_key).or_default().push(position),
            None if self.count > MAXIMUM_LINEAR_SEARCH_THRESHOLD => self.rebuild_index(),
            None => {}
        }
    }

    fn position_of(&self, item: &Rc<dyn IDisposable>) -> Option<usize> {
        match &self.index {
            Some(index) => index.get(&key(item)).and_then(|positions| positions.first().copied()),
            None => self.slots.iter().position(|slot| slot.as_ref().is_some_and(|d| same(d, item))),
        }
    }

    fn remove(&mut self, item: &Rc<dyn IDisposable>) -> Option<Rc<dyn IDisposable>> {
        let position = self.position_of(item)?;
        let removed = self.slots[position].take();
        self.count -= 1;
        if let Some(index) = &mut self.index {
            let item_key = key(item);
            if let Some(positions) = index.get_mut(&item_key) {
                positions.retain(|p| *p != position);
                if positions.is_empty() {
                    index.remove(&item_key);
                }
            }
        }
        if self.slots.len() > SHRINK_THRESHOLD && self.count < self.slots.len() / 2 {
            self.slots.retain(Option::is_some);
            if self.index.is_some() {
                self.rebuild_index();
            }
        }
        removed
    }

    fn take_all(&mut self) -> Vec<Rc<dyn IDisposable>> {
        self.count = 0;
        self.index = None;
        std::mem::take(&mut self.slots).into_iter().flatten().collect()
    }

    fn to_vec(&self) -> Vec<Rc<dyn IDisposable>> {
        self.slots.iter().flatten().cloned().collect()
    }
}

/// A group of disposables that are disposed together (`CompositeDisposable`).
///
/// Disposables compare by identity (`Remove`, `Contains`); a disposable added twice is held twice.
/// A disposable added after the group was disposed is disposed at once. The group disposes its
/// members outside of any borrow of its list, so that a member may add to or remove from the
/// group while it is disposed. Above a thousand disposables, removals find a disposable through an
/// index, as upstream switches to a dictionary; the members keep the order they were added in.
#[derive(Default)]
pub struct CompositeDisposable {
    inner: RefCell<Inner>,
    disposed: Cell<bool>,
}

impl CompositeDisposable {
    /// An empty group.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// An empty group with room for `capacity` disposables.
    pub fn with_capacity(capacity: usize) -> Rc<Self> {
        Rc::new(Self {
            inner: RefCell::new(Inner { slots: Vec::with_capacity(capacity), count: 0, index: None }),
            disposed: Cell::new(false),
        })
    }

    /// A group of the given disposables.
    pub fn from_disposables(disposables: impl IntoIterator<Item = Rc<dyn IDisposable>>) -> Rc<Self> {
        Rc::new(Self { inner: RefCell::new(Inner::with_disposables(disposables.into_iter().collect())), disposed: Cell::new(false) })
    }

    /// The number of disposables in the group.
    pub fn count(&self) -> usize {
        self.inner.borrow().count
    }

    /// Adds a disposable; disposes it at once when the group has been disposed.
    pub fn add(&self, item: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            item.dispose();
            return;
        }
        self.inner.borrow_mut().add(item);
    }

    /// Removes the disposable and disposes it; false when the group does not hold it.
    pub fn remove(&self, item: &Rc<dyn IDisposable>) -> bool {
        if self.disposed.get() {
            return false;
        }
        let removed = self.inner.borrow_mut().remove(item);
        match removed {
            Some(removed) => {
                removed.dispose();
                true
            }
            None => false,
        }
    }

    /// Disposes the disposables and removes them, without disposing the group (`Clear`).
    pub fn clear(&self) {
        if self.disposed.get() {
            return;
        }
        let disposables = self.inner.borrow_mut().take_all();
        for d in disposables {
            d.dispose();
        }
    }

    /// Whether the group holds the disposable.
    pub fn contains(&self, item: &Rc<dyn IDisposable>) -> bool {
        !self.disposed.get() && self.inner.borrow().position_of(item).is_some()
    }

    /// The disposables of the group, in the order they were added (`ToArray`, the enumerator).
    pub fn to_vec(&self) -> Vec<Rc<dyn IDisposable>> {
        if self.disposed.get() {
            return Vec::new();
        }
        self.inner.borrow().to_vec()
    }

    /// Copies the disposables into `array` from `index` (`CopyTo`). Panics, where upstream throws
    /// `ArgumentOutOfRangeException`, when `index` is not in the array or the disposables do not
    /// fit; a disposed group copies nothing.
    pub fn copy_to(&self, array: &mut [Option<Rc<dyn IDisposable>>], index: usize) {
        if index >= array.len() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'arrayIndex')");
        }
        if self.disposed.get() {
            return;
        }
        let disposables = self.to_vec();
        if array.len() - index < disposables.len() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'arrayIndex')");
        }
        for (slot, disposable) in array[index..].iter_mut().zip(disposables) {
            *slot = Some(disposable);
        }
    }

    /// Whether the group is read-only (`IsReadOnly`): never.
    pub fn is_read_only(&self) -> bool {
        false
    }
}

fn same(a: &Rc<dyn IDisposable>, b: &Rc<dyn IDisposable>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

impl IDisposable for CompositeDisposable {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let disposables = self.inner.borrow_mut().take_all();
        for d in disposables {
            d.dispose();
        }
    }
}

impl ICancelable for CompositeDisposable {
    fn is_disposed(&self) -> bool {
        self.disposed.get()
    }
}
