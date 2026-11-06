//! One process-isolated test: native locale values can configure the shared default safely.
use ferroboard_core::{
    layouts::{IKeyEvaluator, KeyEvaluator},
    text::TextCulture,
};
#[test]
fn shared_default_locale_updates_are_visible_across_joined_workers() {
    struct Restore(Option<TextCulture>);
    impl Drop for Restore {
        fn drop(&mut self) {
            KeyEvaluator::default_reference().set_culture_override(self.0);
        }
    }
    let value = KeyEvaluator::default_reference();
    let _restore = Restore(value.culture_override());
    let turkish = TextCulture::get("tr-TR").unwrap();
    value.set_culture_override(Some(turkish));
    let read = std::thread::spawn(|| KeyEvaluator::default_reference().culture())
        .join()
        .unwrap();
    assert_eq!(read, Some(turkish));
    std::thread::spawn(|| {
        KeyEvaluator::default_reference()
            .set_culture_override(Some(TextCulture::get("ja-JP").unwrap()))
    })
    .join()
    .unwrap();
    assert_eq!(value.culture(), Some(TextCulture::get("ja-JP").unwrap()));
    value.set_culture_override(None);
    assert_eq!(value.culture(), Some(TextCulture::invariant()));
}
