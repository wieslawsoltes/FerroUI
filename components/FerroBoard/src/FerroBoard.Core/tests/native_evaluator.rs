//! Native locale values, snapshots and shared default mutation.
use ferroboard_core::{keys::*, layouts::*, text::TextCulture};
#[test]
fn native_evaluator_alias_copy_and_weak_ownership() {
    let value = KeyEvaluator::default();
    let alias = value.clone();
    let weak = value.downgrade();
    let snapshot = KeyEvaluator::from_values(value.values());
    value.set_mode(KeyboardMode::Symbols);
    value.set_culture_override(Some(TextCulture::get("tr-TR").unwrap()));
    assert_eq!(alias.mode(), KeyboardMode::Symbols);
    assert_eq!(alias.culture(), Some(TextCulture::get("tr-TR").unwrap()));
    assert_eq!(snapshot.mode(), KeyboardMode::Characters);
    assert!(snapshot.culture_override().is_none());
    assert!(weak.upgrade().unwrap().ptr_eq(&value));
    drop(alias);
    drop(value);
    assert!(weak.upgrade().is_none());
}
#[test]
fn native_locale_profiles_are_copyable_canonical_values() {
    let a = TextCulture::get("tr-TR").unwrap();
    let b = TextCulture::get("TR-tr").unwrap();
    assert_eq!(a, b);
    assert_ne!(a, TextCulture::invariant());
    assert_eq!(a.to_upper(&"idi".into()).to_utf8().unwrap(), "İDİ");
    assert!(TextCulture::get("invalid locale !!!").is_err());
}
