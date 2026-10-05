//! The reference has no dedicated test class for `AccessText`; these tests
//! pin down the marker handling its users rely on.

use crate::primitives::AccessText;
use crate::test_support::test_scope;
use ferroui_base::Size;

#[test]
fn access_key_is_the_character_after_the_first_underscore() {
    let _scope = test_scope();
    let target = AccessText::new();

    assert_eq!(target.access_key(), None);

    target.set_text(Some("_File"));
    assert_eq!(target.access_key().as_deref(), Some("F"));

    target.set_text(Some("E_xit"));
    assert_eq!(target.access_key().as_deref(), Some("x"));

    target.set_text(Some("Exit_"));
    assert_eq!(target.access_key(), None);

    target.set_text(Some("Exit"));
    assert_eq!(target.access_key(), None);

    target.set_text(None);
    assert_eq!(target.access_key(), None);
}

#[test]
fn access_key_is_a_whole_code_point() {
    let _scope = test_scope();
    let target = AccessText::new();

    target.set_text(Some("_\u{1F600}x"));

    assert_eq!(target.access_key().as_deref(), Some("\u{1F600}"));
}

#[test]
fn remove_access_key_marker_removes_the_first_single_marker_and_unescapes_double_markers() {
    assert_eq!(AccessText::remove_access_key_marker(None), None);
    assert_eq!(AccessText::remove_access_key_marker(Some("")).as_deref(), Some(""));
    assert_eq!(AccessText::remove_access_key_marker(Some("_File")).as_deref(), Some("File"));
    assert_eq!(AccessText::remove_access_key_marker(Some("E_xit")).as_deref(), Some("Exit"));
    assert_eq!(AccessText::remove_access_key_marker(Some("a__b")).as_deref(), Some("a_b"));
    assert_eq!(AccessText::remove_access_key_marker(Some("a__b_c")).as_deref(), Some("a_bc"));
    assert_eq!(AccessText::remove_access_key_marker(Some("_a_b")).as_deref(), Some("a_b"));
    // A trailing marker is kept.
    assert_eq!(AccessText::remove_access_key_marker(Some("Exit_")).as_deref(), Some("Exit_"));
}

#[test]
fn text_layout_does_not_contain_the_marker() {
    let _scope = test_scope();
    let target = AccessText::new();
    target.set_text(Some("_File"));

    target.measure(Size::new(f64::INFINITY, f64::INFINITY));

    // Four glyphs of 6 at the default font size of 12.
    assert_eq!(target.text_layout().width_including_trailing_whitespace(), 24.0);
    assert_eq!(target.desired_size().width, 24.0);
}
