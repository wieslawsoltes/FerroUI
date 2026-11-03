//! Suggestions use Rust value equality and Rc for explicit sharing.
use ferroboard_core::text::{Suggestion, SuggestionKind, Utf16Text};
use std::rc::Rc;
#[test]
fn value_equality_preserves_candidate_fields_and_distinguishes_shared_ownership() {
    let original = Rc::new(Suggestion::new_nullable(
        Some(Utf16Text::from_units(vec![0xd800, 97])),
        SuggestionKind(99),
        f64::from_bits(0x7ff0000000000001),
        true,
    ));
    let alias = original.clone();
    assert!(Rc::ptr_eq(&original, &alias));
    assert_eq!(original, alias);
    let copy = Rc::new((*original).clone());
    assert!(!Rc::ptr_eq(&original, &copy));
    copy.set_confidence(f64::from_bits(0xfff8000000000000));
    assert_eq!(original, copy);
    copy.set_is_removable(true);
    assert_ne!(original, copy);
    assert_ne!(Some(original), None);
}
