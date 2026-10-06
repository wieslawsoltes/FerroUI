use crate::reactive::{IDisposable, IObservable, IObserver, LightweightObservable, LightweightObservableBase};
use crate::{Ref, TypeInfo, Visual};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Tracks an ancestor of a visual in the visual tree.
pub struct VisualLocator;

impl VisualLocator {
    /// An observable of the ancestor at `ancestor_level` (counting only
    /// ancestors of `ancestor_type`, when given) of `relative_to`, or `None`
    /// while `relative_to` is not attached to a visual tree. It publishes
    /// the ancestor again whenever `relative_to` is attached or detached.
    pub fn track(
        relative_to: &Visual,
        ancestor_level: usize,
        ancestor_type: Option<&'static TypeInfo>,
    ) -> Rc<dyn IObservable<Option<Ref<Visual>>>> {
        VisualTracker::new(relative_to.to_ref(), ancestor_level, ancestor_type)
    }
}

struct VisualTracker {
    this: Weak<VisualTracker>,
    base: LightweightObservableBase<Option<Ref<Visual>>>,
    relative_to: Ref<Visual>,
    ancestor_level: usize,
    ancestor_type: Option<&'static TypeInfo>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl VisualTracker {
    fn new(relative_to: Ref<Visual>, ancestor_level: usize, ancestor_type: Option<&'static TypeInfo>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: LightweightObservableBase::new(),
            relative_to,
            ancestor_level,
            ancestor_type,
            subscriptions: RefCell::new(Vec::new()),
        })
    }

    fn attached_detached(&self) {
        self.base.publish_next(self.get_result());
    }

    fn get_result(&self) -> Option<Ref<Visual>> {
        if self.relative_to.is_attached_to_visual_tree() {
            self.relative_to
                .get_visual_ancestors()
                .filter(|x| self.ancestor_type.is_none_or(|t| t.is_assignable_from(x.get_type())))
                .nth(self.ancestor_level)
        } else {
            None
        }
    }
}

impl LightweightObservable<Option<Ref<Visual>>> for VisualTracker {
    fn observable_base(&self) -> &LightweightObservableBase<Option<Ref<Visual>>> {
        &self.base
    }

    fn initialize(&self) {
        let this = self.this.clone();
        let attached = self.relative_to.attached_to_visual_tree(move |_| {
            if let Some(this) = this.upgrade() {
                this.attached_detached();
            }
        });
        let this = self.this.clone();
        let detached = self.relative_to.detached_from_visual_tree(move |_| {
            if let Some(this) = this.upgrade() {
                this.attached_detached();
            }
        });
        self.subscriptions.borrow_mut().extend([attached, detached]);
    }

    fn deinitialize(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn subscribed(&self, observer: &Rc<dyn IObserver<Option<Ref<Visual>>>>, _first: bool) {
        observer.on_next(self.get_result());
    }
}

impl IObservable<Option<Ref<Visual>>> for VisualTracker {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<Ref<Visual>>>>) -> Rc<dyn IDisposable> {
        let this = self.this.upgrade().expect("the tracker is alive while it is subscribed to");
        LightweightObservableBase::subscribe(&this, observer)
    }
}
