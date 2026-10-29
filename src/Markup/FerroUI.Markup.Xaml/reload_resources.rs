//! Rebind non-control DynamicResource consumers to a preserved component root.
use crate::data::HeldAnchor;
use ferroui_base::{controls::ResourceHostRef, Ref, StyledElement};
use std::{cell::RefCell, rc::{Rc, Weak}};

pub(crate) trait AnchorOwner {
    fn anchor(&self) -> Option<HeldAnchor>;
    fn replace_anchor(&self, anchor: Option<HeldAnchor>);
}
type Captures = Rc<RefCell<Vec<Weak<dyn AnchorOwner>>>>;
ferroui_base::ferro_thread_local! {
    static CAPTURES: RefCell<Option<Captures>> = const { RefCell::new(None) };
}
pub(crate) fn record(owner: Rc<dyn AnchorOwner>) {
    CAPTURES.with(|slot| {
        if let Some(captures) = slot.borrow().as_ref() {
            let weak = Rc::downgrade(&owner);
            let mut captures = captures.borrow_mut();
            if !captures.iter().any(|old| Weak::ptr_eq(old, &weak)) {
                captures.push(weak);
            }
        }
    });
}

/// Captures candidate extensions (including style setter values) and their
/// already-created expressions without retaining their targets or owners.
pub struct DynamicResourceReloadScope {
    captures: Captures,
    previous: Option<Captures>,
}
impl Default for DynamicResourceReloadScope {
    fn default() -> Self { Self::new() }
}
impl DynamicResourceReloadScope {
    pub fn new() -> Self {
        let captures = Rc::new(RefCell::new(Vec::new()));
        let previous = CAPTURES.with(|slot| slot.replace(Some(captures.clone())));
        Self { captures, previous }
    }
    pub fn prepare_rebase(
        &self, candidate: &Ref<StyledElement>, root: &Ref<StyledElement>,
    ) -> DynamicResourceRebase {
        let entries = self.captures.borrow().iter().filter_map(Weak::upgrade)
            .filter_map(|owner| {
                let anchor = owner.anchor();
                let matches = match &anchor {
                    Some(HeldAnchor::Element(element)) => element.points_to(candidate),
                    Some(HeldAnchor::Host(host)) => matches!(
                        host.upgrade(), Some(ResourceHostRef::Element(element)) if element.ptr_eq(candidate)
                    ),
                    _ => false,
                };
                matches.then(|| (owner, anchor))
            }).collect();
        DynamicResourceRebase {
            entries, root: root.downgrade(), committed: false,
        }
    }
}
impl Drop for DynamicResourceReloadScope {
    fn drop(&mut self) {
        CAPTURES.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

/// Transfers existing subscriptions on commit and restores them on rollback.
/// Extensions saved in setters use the new anchor for future expressions too.
pub struct DynamicResourceRebase {
    entries: Vec<(Rc<dyn AnchorOwner>, Option<HeldAnchor>)>,
    root: ferroui_base::WeakRef<StyledElement>,
    committed: bool,
}
impl DynamicResourceRebase {
    pub fn commit(&mut self) {
        self.committed = true;
        for (owner, _) in &self.entries {
            owner.replace_anchor(Some(HeldAnchor::Element(self.root.clone())));
        }
    }
    pub fn rollback(&mut self) {
        if self.committed {
            for (owner, previous) in self.entries.iter().rev() {
                owner.replace_anchor(previous.clone());
            }
            self.committed = false;
        }
    }
}
