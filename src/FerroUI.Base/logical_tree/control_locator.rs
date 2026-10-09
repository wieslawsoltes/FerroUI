use crate::reactive::{IDisposable, IObservable, IObserver, LightweightObservable, LightweightObservableBase};
use crate::{Ref, StyledElement, TypeInfo, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Tracks an ancestor of an element in the logical tree.
pub struct ControlLocator;

impl ControlLocator {
    /// An observable of the ancestor at `ancestor_level` (counting only
    /// ancestors of `ancestor_type`, when given) of `relative_to` in the
    /// logical tree. It publishes the ancestor again when `relative_to` is
    /// attached to a logical tree, and `None` when it is detached.
    // Deviation (DEVIATIONS.md, Logical tree): upstream takes an `int` level; a negative level
    // cannot be passed here.
    pub fn track(
        relative_to: &StyledElement,
        ancestor_level: usize,
        ancestor_type: Option<&'static TypeInfo>,
    ) -> Rc<dyn IObservable<Option<Ref<StyledElement>>>> {
        ControlTracker::new(relative_to.to_ref(), ancestor_level, ancestor_type)
    }
}

struct ControlTracker {
    this: Weak<ControlTracker>,
    base: LightweightObservableBase<Option<Ref<StyledElement>>>,
    /// The element and the ancestor found are held weakly (DEVIATIONS.md,
    /// Logical tree): the tracker belongs to a binding of the element, and
    /// the ancestor owns the element. Held strongly, as the managed original
    /// holds both, the element keeps itself and its ancestor alive.
    relative_to: WeakRef<StyledElement>,
    ancestor_level: usize,
    ancestor_type: Option<&'static TypeInfo>,
    value: RefCell<Option<WeakRef<StyledElement>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ControlTracker {
    fn new(relative_to: Ref<StyledElement>, ancestor_level: usize, ancestor_type: Option<&'static TypeInfo>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: LightweightObservableBase::new(),
            relative_to: relative_to.downgrade(),
            ancestor_level,
            ancestor_type,
            value: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        })
    }

    fn attached(&self) {
        self.update();
        let value = self.value();
        self.base.publish_next(value);
    }

    fn value(&self) -> Option<Ref<StyledElement>> {
        self.value.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn detached(&self) {
        *self.value.borrow_mut() = None;
        self.base.publish_next(None);
    }

    fn update(&self) {
        let value = self.relative_to.upgrade().and_then(|relative_to| {
            relative_to
                .get_logical_ancestors()
                .filter(|x| self.ancestor_type.is_none_or(|t| t.is_assignable_from(x.get_type())))
                .nth(self.ancestor_level)
        });
        *self.value.borrow_mut() = value.as_ref().map(Ref::downgrade);
    }
}

impl LightweightObservable<Option<Ref<StyledElement>>> for ControlTracker {
    fn observable_base(&self) -> &LightweightObservableBase<Option<Ref<StyledElement>>> {
        &self.base
    }

    fn initialize(&self) {
        self.update();
        let Some(relative_to) = self.relative_to.upgrade() else { return };
        let this = self.this.clone();
        let attached = relative_to.attached_to_logical_tree(move |_| {
            if let Some(this) = this.upgrade() {
                this.attached();
            }
        });
        let this = self.this.clone();
        let detached = relative_to.detached_from_logical_tree(move |_| {
            if let Some(this) = this.upgrade() {
                this.detached();
            }
        });
        self.subscriptions.borrow_mut().extend([attached, detached]);
    }

    fn deinitialize(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        *self.value.borrow_mut() = None;
    }

    fn subscribed(&self, observer: &Rc<dyn IObserver<Option<Ref<StyledElement>>>>, _first: bool) {
        let value = self.value();
        observer.on_next(value);
    }
}

impl IObservable<Option<Ref<StyledElement>>> for ControlTracker {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<Ref<StyledElement>>>>) -> Rc<dyn IDisposable> {
        let this = self.this.upgrade().expect("the tracker is alive while it is subscribed to");
        LightweightObservableBase::subscribe(&this, observer)
    }
}
