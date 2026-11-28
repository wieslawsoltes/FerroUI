//! Element capture bookkeeping, including direct Pointer API callers.
use super::*;
use std::{cell::Cell, rc::Rc};
#[test]
fn ferro_release_all_covers_direct_captures_and_duplicate_pointer_ids() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let element = InputElement::new();
    let a = Pointer::new(3, PointerType::Mouse, true);
    let b = Pointer::new(3, PointerType::Touch, false);
    a.capture(Some(&element));
    b.capture(Some(&element));
    assert_eq!(element.pointer_captures().len(), 2);
    element.release_pointer_captures();
    assert!(a.captured().is_none());
    assert!(b.captured().is_none());
    assert!(element.pointer_captures().is_empty());
}
#[test]
fn ferro_element_capture_reports_success_and_transfer_removes_old_membership() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let a = InputElement::new();
    let b = InputElement::new();
    let p: Rc<dyn IPointer> = Pointer::new(4, PointerType::Mouse, true);
    assert!(a.capture_pointer(&p));
    assert!(a.capture_pointer(&p));
    assert_eq!(a.pointer_captures().len(), 1);
    p.capture(Some(&b));
    assert!(a.pointer_captures().is_empty());
    assert_eq!(b.pointer_captures().len(), 1);
    p.capture(None);
    assert!(b.pointer_captures().is_empty());
}
#[test]
fn ferro_release_all_does_not_release_a_pointer_transferred_by_reentrant_handler() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let a = InputElement::new();
    let b = InputElement::new();
    let p = Pointer::new(5, PointerType::Mouse, true);
    let q = Pointer::new(6, PointerType::Touch, false);
    p.capture(Some(&a));
    q.capture(Some(&a));
    let target = b.clone();
    let other = q.clone();
    a.pointer_capture_lost(move |_, e| {
        if e.pointer().id() == 5 {
            other.capture(Some(&target));
        }
    });
    a.release_pointer_captures();
    assert!(p.captured().is_none());
    assert!(q.captured().unwrap().ptr_eq(&b));
    assert!(a.pointer_captures().is_empty());
    q.capture(None);
}
#[test]
fn ferro_capture_veto_does_not_publish_membership() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let a = InputElement::new();
    a.pointer_capture_changing(|_, e| e.set_handled(true));
    let p: Rc<dyn IPointer> = Pointer::new(7, PointerType::Touch, false);
    assert!(!a.capture_pointer(&p));
    assert!(a.pointer_captures().is_empty());
    assert!(p.captured().is_none());
}
#[test]
fn ferro_capture_tracking_does_not_keep_pointer_alive() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let a = InputElement::new();
    let p = Pointer::new(8, PointerType::Mouse, true);
    let weak = Rc::downgrade(&p);
    p.capture(Some(&a));
    drop(p);
    assert!(weak.upgrade().is_none());
    assert!(a.pointer_captures().is_empty());
    a.release_pointer_captures();
}
#[test]
fn ferro_capture_registry_tracks_platform_loss_and_disposal_before_handlers() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let a = InputElement::new();
    let p = Pointer::new(9, PointerType::Mouse, true);
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let weak = a.downgrade();
    a.pointer_capture_lost(move |_, _| {
        assert!(weak.upgrade().unwrap().pointer_captures().is_empty());
        c.set(c.get() + 1);
    });
    p.capture(Some(&a));
    p.platform_capture_lost();
    assert_eq!(count.get(), 1);
    p.capture(Some(&a));
    p.dispose();
    assert_eq!(count.get(), 2);
    assert!(a.pointer_captures().is_empty());
}

#[test]
fn ferro_release_all_respects_release_cancellation() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let element = InputElement::new();
    let pointer = Pointer::new(10, PointerType::Touch, false);
    pointer.capture(Some(&element));
    let token = element.pointer_capture_changing(|_, e| {
        if e.new_value().is_none() {
            e.set_handled(true);
        }
    });
    element.release_pointer_captures();
    assert_eq!(element.pointer_captures().len(), 1);
    element.remove_handler(InputElement::pointer_capture_changing_event(), token);
    element.release_pointer_captures();
    assert!(pointer.captured().is_none());
}
#[test]
fn ferro_release_snapshot_preserves_new_capture_installed_during_notification() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    let element = InputElement::new();
    let pointer = Pointer::new(11, PointerType::Touch, false);
    let added = Pointer::new(12, PointerType::Touch, false);
    pointer.capture(Some(&element));
    let weak = element.downgrade();
    let other = added.clone();
    let token = element.pointer_capture_lost(move |_, e| {
        if e.pointer().id() == 11 {
            other.capture(weak.upgrade().as_ref());
        }
    });
    element.release_pointer_captures();
    assert!(pointer.captured().is_none());
    assert!(added.captured().unwrap().ptr_eq(&element));
    element.remove_handler(InputElement::pointer_capture_lost_event(), token);
    element.release_pointer_captures();
    assert!(added.captured().is_none());
}
#[test]
fn ferro_element_capture_entry_supports_custom_pointer_implementations() {
    let _scope = crate::threading::Dispatcher::unit_test_scope();
    use std::cell::RefCell;
    struct Custom(RefCell<Option<crate::Ref<InputElement>>>);
    impl IPointer for Custom {
        fn id(&self) -> i32 {
            13
        }
        fn type_(&self) -> PointerType {
            PointerType::Touch
        }
        fn is_primary(&self) -> bool {
            true
        }
        fn capture(&self, target: Option<&crate::Ref<InputElement>>) {
            self.0.replace(target.cloned());
        }
        fn captured(&self) -> Option<crate::Ref<InputElement>> {
            self.0.borrow().clone()
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    let pointer: Rc<dyn IPointer> = Rc::new(Custom(RefCell::new(None)));
    let element = InputElement::new();
    assert!(element.capture_pointer(&pointer));
    assert_eq!(element.pointer_captures().len(), 1);
    element.release_pointer_captures();
    assert!(pointer.captured().is_none());
    assert!(element.pointer_captures().is_empty());
}
