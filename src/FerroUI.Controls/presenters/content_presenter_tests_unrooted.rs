//! Port of `ContentPresenterTests_Unrooted.cs`: tests for content controls
//! that are not attached to a logical tree.

use crate::presenters::ContentPresenter;
use crate::templates::FuncDataTemplate;
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{Border, Canvas, Control, Decorator, TextBlock};

#[test]
fn setting_content_to_control_should_not_set_child_unless_update_child_called() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    assert!(target.child().is_none());

    target.apply_template();
    assert!(target.child().is_none());

    target.update_child();
    assert_eq!(target.child().unwrap(), child);
}

#[test]
fn setting_content_to_string_should_not_create_text_block_unless_update_child_called() {
    let _scope = test_scope();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().is_none());

    target.apply_template();
    assert!(target.child().is_none());

    target.update_child();
    assert!(target.child().unwrap().is::<TextBlock>());
    assert_eq!(target.child().unwrap().cast::<TextBlock>().unwrap().text().as_deref(), Some("Foo"));
}

#[test]
fn clearing_control_content_should_remove_child_immediately() {
    let _scope = test_scope();
    let target = ContentPresenter::new();
    let child = Border::new();

    target.set_content(Some(Control::boxed(&child)));
    target.update_child();
    assert_eq!(target.child().unwrap(), child);

    target.set_content(None);
    assert!(target.child().is_none());
}

#[test]
fn clearing_string_content_should_remove_child_immediately() {
    let _scope = test_scope();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    target.update_child();
    assert!(target.child().unwrap().is::<TextBlock>());

    target.set_content(None);
    assert!(target.child().is_none());
}

#[test]
fn adding_to_logical_tree_should_reevaluate_data_templates() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let target = ContentPresenter::new();

    target.set_content(boxed_str("Foo"));
    assert!(target.child().is_none());

    root.set_child(&target);
    target.apply_template();
    assert!(target.child().unwrap().is::<TextBlock>());

    root.set_child(None);
    let root = TestRoot::new();
    root.data_templates().add(FuncDataTemplate::for_type::<String>(|_, _| Some(Decorator::new().upcast()), false));

    root.set_child(&target);
    target.apply_template();
    assert!(target.child().unwrap().is::<Decorator>());
}

#[test]
fn should_reset_inheritance_parent_when_child_removed() {
    let _scope = test_scope();
    let logical_parent = Canvas::new();
    let child = TextBlock::new();
    let target = ContentPresenter::new();

    child.set_parent(&logical_parent);
    target.set_content(Some(Control::boxed(&child)));
    target.update_child();
    target.set_content(None);
    target.update_child();

    // The inheritance parent is exposed via the styling parent.
    let styling_parent = child.styling_parent().and_then(|parent| parent.as_element().cloned());
    assert_eq!(styling_parent.unwrap(), logical_parent);
}
