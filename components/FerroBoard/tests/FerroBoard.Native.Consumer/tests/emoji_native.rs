//! Public native emoji API; no sample, internal helpers or compatibility collection types.
use ferroboard_core::{
    emoji::{EmojiCatalog, EmojiError, EmojiHistory},
    text::Utf16Text,
};
use std::{cell::Cell, error::Error, rc::Rc, sync::Arc};

#[test]
fn native_emoji_collections_and_errors_work_in_a_downstream_crate() -> Result<(), Box<dyn Error>> {
    let catalog = EmojiCatalog::parse(&"a;one;alpha\nb;two;beta\n".into(), "custom".into())?;
    let mut result: Vec<_> = catalog.search(&"alpha".into(), 10);
    let first = result.pop().expect("source search result");
    first
        .keywords
        .lock()
        .retain(|word| word != &Utf16Text::from("alpha"));
    first.keywords.lock().push("replacement".into());
    assert!(catalog.search(&"alpha".into(), 10).is_empty());
    assert!(Arc::ptr_eq(
        &catalog.search(&"replacement".into(), 10)[0],
        &first
    ));
    let snapshot = catalog.all();
    catalog.get_emojis(first.category).lock().clear();
    assert_eq!(snapshot.count(), 2);
    assert_eq!(catalog.all().count(), 0);

    let history = EmojiHistory::default();
    let recent = history.recent();
    recent.lock().push(Some("direct".into()));
    assert!(Arc::ptr_eq(&recent, &history.recent()));
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    history.changed().add(Rc::new(move |history| {
        assert!(
            history.recent().try_lock().is_some(),
            "event must not retain recent guard"
        );
        assert!(
            history.pinned().try_lock().is_some(),
            "event must not retain pinned guard"
        );
        let previous = seen.replace(seen.get() + 1);
        if previous == 0 {
            history.pin(&"pinned".into()).unwrap();
        }
    }));
    history.add(&"new".into())?;
    assert_eq!(calls.get(), 2);
    let restored = EmojiHistory::default();
    restored.import(&history.export())?;
    assert_eq!(restored.combined(), history.combined());
    let error = history.import("{").unwrap_err();
    assert!(matches!(error, EmojiError::Json(_)));
    assert!(error.source().is_some());
    assert!(error.to_string().contains("JSON"));
    Ok(())
}
