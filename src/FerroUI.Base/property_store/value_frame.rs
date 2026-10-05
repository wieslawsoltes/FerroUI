use super::{FramePriority, FrameType, IValueEntry};
use crate::data::BindingPriority;
use crate::{FerroObject, FerroProperty, Ref, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// State shared by all value frame kinds.
pub(crate) struct ValueFrameBase {
    entries: RefCell<Vec<Rc<dyn IValueEntry>>>,
    owner: RefCell<Option<WeakRef<FerroObject>>>,
    is_shared: Cell<bool>,
    priority: BindingPriority,
    frame_priority: FramePriority,
}

impl ValueFrameBase {
    pub fn new(priority: BindingPriority, type_: FrameType) -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
            owner: RefCell::new(None),
            is_shared: Cell::new(false),
            priority,
            frame_priority: FramePriority::new(priority, type_),
        }
    }

    /// Creates the state for a frame that is shared between several stores
    /// and therefore has no single owner.
    pub fn new_shared(priority: BindingPriority, type_: FrameType) -> Self {
        Self { is_shared: Cell::new(true), ..Self::new(priority, type_) }
    }

    /// Whether the frame is shared between several stores.
    #[inline]
    pub fn is_shared(&self) -> bool {
        self.is_shared.get()
    }

    /// Marks the frame as shared between several stores: it no longer has a
    /// single owner.
    pub fn make_shared(&self) {
        self.is_shared.set(true);
        *self.owner.borrow_mut() = None;
    }

    #[inline]
    pub fn entry_count(&self) -> usize {
        self.entries.borrow().len()
    }

    #[inline]
    pub fn priority(&self) -> BindingPriority {
        self.priority
    }

    #[inline]
    pub fn frame_priority(&self) -> FramePriority {
        self.frame_priority
    }

    /// The object whose value store owns the frame.
    pub fn owner(&self) -> Option<Ref<FerroObject>> {
        assert!(!self.is_shared.get(), "Cannot get owner for shared ValueFrame");
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn set_owner(&self, owner: Option<WeakRef<FerroObject>>) {
        if self.is_shared.get() {
            return;
        }
        let mut current = self.owner.borrow_mut();
        if current.is_some() && owner.is_some() {
            panic!("ValueFrame already has an owner.");
        }
        *current = owner;
    }

    pub fn contains(&self, property: &FerroProperty) -> bool {
        self.entries.borrow().iter().any(|e| e.property().id() == property.id())
    }

    pub fn get_entry(&self, index: usize) -> Rc<dyn IValueEntry> {
        self.entries.borrow()[index].clone()
    }

    pub fn find_entry(&self, property: &FerroProperty) -> Option<Rc<dyn IValueEntry>> {
        self.entries.borrow().iter().find(|e| e.property().id() == property.id()).cloned()
    }

    pub fn add(&self, value: Rc<dyn IValueEntry>) {
        debug_assert!(!value.property().is_direct());
        self.entries.borrow_mut().push(value);
    }

    pub fn remove(&self, property: &FerroProperty) {
        debug_assert!(!property.is_direct());
        let mut entries = self.entries.borrow_mut();
        if let Some(index) = entries.iter().position(|e| e.property().id() == property.id()) {
            entries.remove(index);
        }
    }

    pub fn unsubscribe_all(&self) {
        let entries: Vec<_> = self.entries.borrow().clone();
        for entry in entries {
            entry.unsubscribe();
        }
    }
}

/// A set of values for properties that share a priority and an activation
/// state: an immediate (non-local) value set on an object, or an instance of a
/// style or theme.
pub(crate) trait ValueFrame: Any {
    fn base(&self) -> &ValueFrameBase;

    /// Returns whether the frame is active and whether that state has changed
    /// since it was last read.
    fn get_is_active(&self) -> (bool, bool);

    /// Releases the frame's entries.
    fn dispose(&self) {
        self.base().unsubscribe_all();
    }

    fn as_any(&self) -> &dyn Any;
}

impl dyn ValueFrame {
    pub fn is_active(&self) -> bool {
        self.get_is_active().0
    }

    /// Looks up the entry for `property`. Returns the entry if present and the
    /// frame is active, along with whether the active state has changed.
    pub fn try_get_entry_if_active(&self, property: &FerroProperty) -> (Option<Rc<dyn IValueEntry>>, bool) {
        match self.base().find_entry(property) {
            Some(entry) => {
                let (active, changed) = self.get_is_active();
                (active.then_some(entry), changed)
            }
            None => (None, false),
        }
    }

    /// Called by a binding entry of this frame when its source completes.
    pub fn on_binding_completed(self: &Rc<Self>, property: &'static FerroProperty) {
        self.base().remove(property);
        if let Some(owner) = self.base().owner() {
            owner.values().on_value_entry_removed(&owner, self, property);
        }
    }
}

/// Identity comparison of two value frames.
#[inline]
pub(crate) fn frame_ptr_eq(a: &Rc<dyn ValueFrame>, b: &Rc<dyn ValueFrame>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
