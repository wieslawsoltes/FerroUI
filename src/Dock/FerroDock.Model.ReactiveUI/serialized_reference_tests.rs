//! Source serialized references retain their actual models until explicitly cleared.
use crate::{
    controls::{Document, DocumentDock, RootDock},
    core::DockWindow,
};
use ferrodock_model::core::*;
use std::rc::Rc;
#[test]
fn source_owner_window_and_parent_window_fields_hold_their_original_references() {
    let root = RootDock::new();
    let doc = Document::new();
    let window = DockWindow::new();
    let source_window = DockWindow::new();
    let owner = DocumentDock::new();
    let parent = DockWindow::new();
    let weak_window = Rc::downgrade(&source_window);
    let weak_owner = Rc::downgrade(&owner);
    let weak_parent = Rc::downgrade(&parent);
    root.set_window(Some(source_window.clone()));
    doc.set_owner(Some(owner.clone()));
    window.set_parent_window(Some(parent.clone()));
    drop(source_window);
    drop(owner);
    drop(parent);
    assert!(Rc::ptr_eq(
        &root.window().unwrap(),
        &(weak_window.upgrade().unwrap() as Rc<dyn IDockWindow>)
    ));
    assert!(Rc::ptr_eq(
        &doc.owner().unwrap(),
        &(weak_owner.upgrade().unwrap() as Rc<dyn IDockable>)
    ));
    assert!(Rc::ptr_eq(
        &window.parent_window().unwrap(),
        &(weak_parent.upgrade().unwrap() as Rc<dyn IDockWindow>)
    ));
    root.set_window(None);
    doc.set_owner(None);
    window.set_parent_window(None);
    assert!(root.window().is_none() && doc.owner().is_none() && window.parent_window().is_none());
    assert!(
        weak_window.upgrade().is_none()
            && weak_owner.upgrade().is_none()
            && weak_parent.upgrade().is_none()
    );
}
