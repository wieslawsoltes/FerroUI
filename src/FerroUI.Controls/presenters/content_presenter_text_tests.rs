//! The text property tests of the content presenter tests of the reference,
//! which were deferred until the text controls existed.

use crate::documents::TextElement;
use crate::presenters::ContentPresenter;
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::TextBlock;

#[test]
fn content_presenter_letter_spacing_default_value_is_zero() {
    let presenter = ContentPresenter::new();
    assert_eq!(presenter.letter_spacing(), 0.0);
}

#[test]
fn content_presenter_letter_spacing_can_be_set_and_retrieved() {
    let presenter = ContentPresenter::new();
    presenter.set_letter_spacing(3.5);
    assert_eq!(presenter.letter_spacing(), 3.5);
}

#[test]
fn content_presenter_letter_spacing_can_be_negative() {
    let presenter = ContentPresenter::new();
    presenter.set_letter_spacing(-2.0);
    assert_eq!(presenter.letter_spacing(), -2.0);
}

#[test]
fn content_presenter_letter_spacing_propagates_to_text_block_child() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_content(boxed_str("Test Content"));
    presenter.set_letter_spacing(4.0);
    let _root = TestRoot::with_child(&presenter);

    presenter.update_child();

    let text_block = presenter.child().and_then(|child| child.cast::<TextBlock>()).unwrap();
    assert_eq!(text_block.letter_spacing(), 4.0);
}

#[test]
fn content_presenter_letter_spacing_updates_text_block_when_changed() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_content(boxed_str("Test Content"));
    presenter.set_letter_spacing(1.0);
    let _root = TestRoot::with_child(&presenter);

    presenter.update_child();
    let text_block = presenter.child().and_then(|child| child.cast::<TextBlock>());

    presenter.set_letter_spacing(6.0);

    assert_eq!(text_block.unwrap().letter_spacing(), 6.0);
}

#[test]
fn content_presenter_letter_spacing_property_inherits_from_text_block() {
    // Verify that the letter spacing of the content presenter uses the text
    // element letter spacing definition.
    assert!(std::ptr::eq(
        TextElement::letter_spacing_property().as_property(),
        ContentPresenter::letter_spacing_property().as_property()
    ));
}

/// The string content of the reference tests: the child is a text block
/// that shows the content.
#[test]
fn setting_content_to_string_should_create_text_block() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    let _root = TestRoot::with_child(&presenter);

    presenter.set_content(boxed_str("Foo"));
    presenter.update_child();

    let text_block = presenter.child().and_then(|child| child.cast::<TextBlock>()).unwrap();
    assert_eq!(text_block.text().as_deref(), Some("Foo"));
}

#[test]
fn recognizes_access_key_should_create_access_text() {
    let _scope = test_scope();
    let presenter = ContentPresenter::new();
    presenter.set_recognizes_access_key(true);
    let _root = TestRoot::with_child(&presenter);

    presenter.set_content(boxed_str("_Foo"));
    presenter.update_child();

    let access_text = presenter.child().and_then(|child| child.cast::<crate::primitives::AccessText>()).unwrap();
    assert_eq!(access_text.text().as_deref(), Some("_Foo"));
    assert_eq!(access_text.access_key().as_deref(), Some("F"));
}
